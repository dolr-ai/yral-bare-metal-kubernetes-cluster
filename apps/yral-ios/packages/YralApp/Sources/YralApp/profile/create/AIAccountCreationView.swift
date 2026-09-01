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
    @State private var errorMessage: String?
    /// The in-flight step task — Reset and sheet dismissal stop it;
    /// cancellation propagates into its URLSession awaits.
    @State private var flowTask: Task<Void, Never>?
    @State private var isResetDialogShown = false
    @Environment(\.dismiss) private var dismiss

    /// Kotlin `PROMPT_CHAR_LIMIT`.
    private let promptCharacterLimit = 400

    private let influencerDataSource = AIInfluencerDataSource()
    private var spacetime: SpacetimeDBRemoteDataSource {
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
                if let profile = draft.profileUnderReview {
                    ProfileReviewForm(
                        profile: profile,
                        isWorking: draft.step.isWorking
                    ) {
                        flowTask = Task { await createAccount(profile: profile) }
                    }
                }
            case .done:
                doneView
            }

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
        draft = AICreationDraft()
        errorMessage = nil
    }

    private func goBack() {
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
    private func isUserCancellation(_ error: Error) -> Bool {
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
    private func errorText(of error: Error) -> String {
        if case let NetworkError.http(_, body) = error, let body, !body.isEmpty {
            return body
        }
        return (error as? LocalizedError)?.errorDescription ?? String(describing: error)
    }
}

// MARK: - Generation actions — cached per input; a stopped run never
// resurrects a Reset draft.

private extension AIAccountCreationView {

    func generatePersona() async {
        errorMessage = nil
        // Unchanged description → reuse this draft's earlier generation;
        // the API is hit only when the input changed.
        if draft.personaSourceDescription == draft.descriptionText {
            draft.step = .personaReview
            return
        }
        draft.step = .generatingPersona
        do {
            guard let idToken = authClient.idToken else {
                throw AuthError.oauthFailed(errorDescription: "Not signed in")
            }
            let instructions = try await influencerDataSource.generatePrompt(
                prompt: draft.descriptionText,
                idToken: idToken
            )
            draft.instructionsText = instructions
            draft.personaSourceDescription = draft.descriptionText
            draft.step = .personaReview
        } catch {
            // Reset/dismissal stopped the run — restoring the form is
            // idempotent with a Reset draft, and correct for a resume.
            draft.step = .descriptionEntry
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
    }

    func generateMetadata() async {
        errorMessage = nil
        let sourceInstructions = draft.instructionsText
        // Unchanged instructions → reuse this draft's earlier profile;
        // the API is hit only when the input changed.
        if draft.profileSourceInstructions == sourceInstructions,
           draft.profileUnderReview != nil {
            draft.step = .reviewProfile
            return
        }
        draft.step = .generatingMetadata
        do {
            guard let idToken = authClient.idToken else {
                throw AuthError.oauthFailed(errorDescription: "Not signed in")
            }
            let metadata = try await influencerDataSource.validateAndGenerateMetadata(
                systemInstructions: sourceInstructions,
                idToken: idToken
            )
            guard metadata.isValid,
                  let name = metadata.name,
                  let displayName = metadata.displayName,
                  let avatarURL = metadata.avatarURL
            else {
                draft.step = .personaReview
                errorMessage = metadata.validationReason ?? "The AI's instructions were rejected — edit and retry."
                return
            }
            let profile = AIProfileDetails(
                systemInstructions: sourceInstructions,
                name: name,
                displayName: displayName,
                description: metadata.description ?? "",
                avatarURL: avatarURL,
                initialGreeting: metadata.initialGreeting ?? "",
                suggestedMessages: metadata.suggestedMessages,
                personalityTraits: metadata.personalityTraits,
                category: metadata.category ?? "general",
                isNSFW: metadata.isNSFW
            )
            draft.profileUnderReview = profile
            draft.profileSourceInstructions = sourceInstructions
            draft.step = .reviewProfile
        } catch {
            if isUserCancellation(error) {
                // Reset clears the draft — never resurrect it. A pull-down
                // leaves the instructions intact, so restore the form for
                // the next resume.
                if draft.instructionsText == sourceInstructions {
                    draft.step = .personaReview
                }
                return
            }
            draft.step = .personaReview
            errorMessage = errorText(of: error)
        }
    }

    func createAccount(profile: AIProfileDetails) async {
        errorMessage = nil
        draft.step = .creating
        // Resume an in-flight creation for the SAME profile if a previous
        // attempt failed midway (Kotlin BotCreationProgress semantics).
        if draft.creationProgress?.profileKey != profile.profileKey {
            draft.creationProgress = AICreationProgress(profileKey: profile.profileKey)
        }
        guard var progress = draft.creationProgress else { return }
        do {
            _ = try await AIAccountCreator.create(
                profile: profile,
                progress: &progress,
                context: AIAccountCreator.CreationContext(
                    authClient: authClient,
                    sessionStore: sessionStore,
                    influencerDataSource: influencerDataSource,
                    spacetime: spacetime
                )
            )
            // Creation succeeded — the draft is spent; a fresh wizard
            // starts on the next Create tap.
            draft = AICreationDraft()
            draft.step = .done
        } catch {
            if isUserCancellation(error) {
                // Reset clears the draft — never resurrect the profile.
                // A pull-down keeps it: restore the review form so a
                // retry resumes the pipeline where it stopped.
                if draft.profileUnderReview?.profileKey == profile.profileKey {
                    draft.creationProgress = progress
                    draft.step = .reviewProfile
                }
                return
            }
            // A failed run keeps the partially-completed progress
            // record — a retry resumes where the pipeline stopped.
            draft.creationProgress = progress
            draft.profileUnderReview = profile
            draft.step = .reviewProfile
            errorMessage = errorText(of: error)
        }
    }
}
