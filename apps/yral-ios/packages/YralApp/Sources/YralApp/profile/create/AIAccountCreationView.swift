import Foundation
import SwiftUI

/// AI account creation — SwiftUI port of the Kotlin AiInfluencer wizard
/// (`AiInfluencerViewModel` steps condensed): describe the persona →
/// review/edit the generated instructions → creating (the multi-step
/// pipeline runs; a failure offers retry, which resumes where it failed).
///
/// No view model — state lives in the draft (@Binding, owned by
/// MainTabView) plus transient @State here; the flow calls
/// `AIInfluencerDataSource` (persona LLM) and `AIAccountCreator` (the
/// creation pipeline) directly.
///
/// Navigation chrome (operator request 2026-09-01): pull down to LEAVE
/// — the draft lives outside the sheet (in MainTabView), so tapping
/// Create again resumes exactly where you left off. The header's Reset
/// is the explicit clear of the whole draft (confirmed when it holds
/// anything). The chevron-back reworks earlier steps.
///
/// Loading is inline: the form stays put and its button swaps to a
/// spinner WITH its working text inline while the call runs.
///
/// Generation results are cached per input (the draft's source
/// fields): re-advancing with UNCHANGED description/instructions reuses
/// what this draft generated earlier — the API is hit only when the
/// input changed.
struct AIAccountCreationView: View {

    // TODO(create-offline-resumability): the draft survives SHEET
    // dismissal (hoisted into MainTabView) but is in-memory only —
    // persist it to disk across launches (description, instructions,
    // profile under review, creation progress) so a user who leaves
    // mid-flow (app killed, phone off) resumes where they were. The
    // multi-step process + LLM latency makes abandonment likely; the
    // pipeline steps are idempotent-guarded server-side, so only this
    // draft needs persisting.

    let authClient: AuthClient
    let sessionStore: SessionStore

    // MARK: - Flow state

    /// The wizard's whole state, owned by MainTabView (OUTSIDE the
    /// sheet) — pull-down dismissal can't lose it, and a later Create
    /// tap resumes exactly here.
    @Binding var draft: AICreationDraft

    /// Transient, view-local feedback (never resumed).
    @State var errorMessage: String?
    /// The in-flight step task — Reset and sheet dismissal stop it;
    /// cancellation propagates into its URLSession awaits.
    @State var flowTask: Task<Void, Never>?
    @State private var isResetDialogShown = false
    /// The last recorded step — the step change drives the directional
    /// slide transition (forward → left, back → right).
    @State var previousStep: FlowStep = .descriptionEntry
    @Environment(\.dismiss) private var dismiss

    /// Kotlin `PROMPT_CHAR_LIMIT`.
    private let promptCharacterLimit = 400

    let influencerDataSource = AIInfluencerDataSource()
    var spacetime: SpacetimeDBRemoteDataSource {
        SpacetimeDBRemoteDataSource(idTokenProvider: { [weak authClient] in
            authClient?.idToken
        })
    }

    var body: some View {
        VStack(spacing: 16) {
            if draft.step.showsHeader {
                AICreationHeader(
                    showsBackButton: draft.step.showsBackButton,
                    onBack: goBack,
                    onReset: requestReset
                )
            }

            stepContent
                // Identity per FORM (not per step): working steps keep
                // the form mounted (no re-identify, no flicker); only
                // cross-form moves re-identify → slide.
                .id(draft.step.formIdentity)
                .transition(stepTransition)

            if let errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.pink)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 16)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 20)
        .padding(.bottom, 16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Color.black)
        .animation(.easeInOut(duration: 0.35), value: draft.step)
        // The celebration overlay — confetti + party horn on the done
        // step only (3D-accelerated via Canvas/Metal; zero hit-testing).
        #if canImport(UIKit)
            .overlay {
                if draft.step == .done {
                    CelebrationView()
                }
            }
        #endif
        // Pull-down LEAVES (never discards — the draft lives in
        // MainTabView): in-flight work is stopped here and the draft
        // resumes on the next Create tap. A completed wizard's draft is
        // spent, so it resets for the next run.
        .onDisappear {
            flowTask?.cancel()
            if draft.step == .done {
                draft = AICreationDraft()
            }
        }
        .confirmationDialog(
            "Start over?",
            isPresented: $isResetDialogShown,
            titleVisibility: .visible
        ) {
            Button("Start over", role: .destructive) { resetDraft() }
            Button("Keep working", role: .cancel) {}
        } message: {
            Text("This clears your description and everything generated so far.")
        }
        #if canImport(UIKit)
            .toolbar(.hidden, for: .navigationBar)
        #endif
    }

    /// The step forms — extracted so the transition modifier wraps the
    /// whole step (header excluded: it stays put while forms slide).
    @ViewBuilder
    private var stepContent: some View {
        switch draft.step {
        case .descriptionEntry, .generatingPersona:
            DescriptionEntryForm(
                descriptionText: $draft.descriptionText,
                characterLimit: promptCharacterLimit,
                isWorking: draft.step.isWorking
            ) {
                flowTask = Task { await generatePersona() }
            }
        case .personaReview, .generatingMetadata:
            PersonaReviewForm(
                instructionsText: $draft.instructionsText,
                isWorking: draft.step.isWorking
            ) {
                flowTask = Task { await generateMetadata() }
            }
        case .reviewProfile, .creating:
            if let profileUnderReview = draft.profileUnderReview {
                ProfileReviewForm(
                    profile: Binding(
                        get: { draft.profileUnderReview ?? profileUnderReview },
                        set: { draft.profileUnderReview = $0 }
                    ),
                    isWorking: draft.step.isWorking
                ) {
                    flowTask = Task { await createAccount(profile: draft.profileUnderReview ?? profileUnderReview) }
                }
            }
        case .done:
            doneView
        }
    }

    /// The directional slide: forward progress → new step slides in
    /// from the RIGHT while the old slides LEFT; the back chevron → the
    /// reverse (like a navigation push/pop, operator request 2026-09-01).
    private var stepTransition: AnyTransition {
        switch FlowStep.transition(from: previousStep, to: draft.step) {
        case .forward:
            return .asymmetric(
                insertion: .move(edge: .trailing).combined(with: .opacity),
                removal: .move(edge: .leading).combined(with: .opacity)
            )
        case .backward:
            return .asymmetric(
                insertion: .move(edge: .leading).combined(with: .opacity),
                removal: .move(edge: .trailing).combined(with: .opacity)
            )
        case nil:
            return .opacity
        }
    }

    /// Does the draft hold anything worth confirming on Reset? A blank
    /// first screen (and the done screen) has nothing to clear.
    private var hasDraftContent: Bool {
        switch draft.step {
        case .descriptionEntry:
            return !draft.descriptionText.isBlank
        case .personaReview, .reviewProfile,
             .generatingPersona, .generatingMetadata, .creating:
            return true
        case .done:
            return false
        }
    }

    private func requestReset() {
        if hasDraftContent {
            isResetDialogShown = true
        } else {
            resetDraft()
        }
    }

    /// Clear the whole draft — description, generated persona, profile,
    /// and the resumable creation record — back to a blank first screen.
    private func resetDraft() {
        flowTask?.cancel()
        previousStep = draft.step
        draft = AICreationDraft()
        errorMessage = nil
    }

    private func goBack() {
        previousStep = draft.step
        switch draft.step {
        case .personaReview:
            draft.step = .descriptionEntry
        case .reviewProfile:
            draft.step = .personaReview
        case .descriptionEntry, .generatingPersona, .generatingMetadata,
             .creating, .done:
            break
        }
    }

    /// True when the error is the user's own Reset/dismissal stopping
    /// an in-flight call — surfaced as a quiet return, not an error.
    func isUserCancellation(_ error: Error) -> Bool {
        error is CancellationError || (error as? URLError)?.code == .cancelled
    }

    // MARK: - Done

    private var doneView: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 48))
                .foregroundStyle(.pink)
            Text("Your AI account is live")
                .font(.title2.weight(.semibold))
            Button("Done") {
                dismiss()
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
        }
        .padding(.top, 120)
    }

    /// Kotlin `extractServerMessage` parity — HTTP error bodies carry the
    /// user-facing server message.
    func errorText(of error: Error) -> String {
        if case let NetworkError.http(_, body) = error, let body, !body.isEmpty {
            return body
        }
        return (error as? LocalizedError)?.errorDescription ?? String(describing: error)
    }
}

// MARK: - Previews

#if DEBUG
#Preview("wizard — describe your AI") {
    let sessionStore = SessionStore()
    AIAccountCreationView(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: sessionStore
        ),
        sessionStore: sessionStore,
        draft: .constant(AICreationDraft())
    )
    .preferredColorScheme(.dark)
}

#Preview("wizard — done (with confetti)") {
    let sessionStore = SessionStore()
    AIAccountCreationView(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: sessionStore
        ),
        sessionStore: sessionStore,
        draft: .constant(AICreationDraft(step: .done))
    )
    .preferredColorScheme(.dark)
}
#endif
