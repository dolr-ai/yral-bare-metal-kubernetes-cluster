import Foundation
import SwiftUI

/// AI account creation — SwiftUI port of the Kotlin AiInfluencer wizard
/// (`AiInfluencerViewModel` steps condensed): describe the persona →
/// review/edit the generated instructions → creating (the multi-step
/// pipeline runs; a failure offers retry, which resumes where it failed).
///
/// No view model — state lives HERE as @State; the flow calls
/// `AIInfluencerDataSource` (persona LLM) and `AIAccountCreator` (the
/// creation pipeline) directly.
///
/// Navigation chrome: the header's Cancel is the single exit — while a
/// step runs it stops the in-flight call (nothing lost; a retry resumes
/// where it stopped), otherwise it leaves the sheet (with a discard
/// confirmation once anything is typed/generated). The chevron-back
/// reworks earlier steps. The sheet's grabber (see MainTabView) stays
/// visible, but the pull-down gesture is disabled whenever the wizard
/// holds content — pulling down can never silently discard progress.
///
/// Loading is inline, Apple-style: no full-screen waiting step — the
/// form stays put and its button swaps to a spinner while the call
/// runs (the nearest native equivalent of Apple Pay's inline button
/// progress; the exact left-to-right green sweep is private API).
struct AIAccountCreationView: View {

    // TODO(create-offline-resumability): persist the wizard state across
    // launches — descriptionText, generated instructions, the profile being
    // reviewed, and the AICreationProgress record — so a user who leaves
    // mid-flow (app killed, tab away, phone off) returns exactly where they
    // were. The multi-step process + LLM latency makes abandonment likely;
    // draft persistence (SwiftData/UserDefaults file) + the progress record's
    // step guards give natural resumability. The pipeline steps themselves
    // are already idempotent-guarded server-side, so only the UI state needs
    // storing.

    let authClient: AuthClient
    let sessionStore: SessionStore

    // MARK: - Flow state

    @State private var step: FlowStep = .descriptionEntry
    @State private var descriptionText = ""
    @State private var instructionsText = ""
    @State private var errorMessage: String?
    @State private var creationProgress: AICreationProgress?
    /// The in-flight step task (persona/metadata/creation) — the loading
    /// step's Cancel stops it; cancellation propagates into its
    /// URLSession awaits.
    @State private var flowTask: Task<Void, Never>?
    @State private var isDiscardDialogShown = false
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
            if step.showsHeader {
                AICreationHeader(
                    showsBackButton: step.showsBackButton,
                    onBack: goBack,
                    onCancel: requestDismiss
                )
            }

            switch step {
            case .descriptionEntry, .generatingPersona:
                DescriptionEntryForm(
                    descriptionText: $descriptionText,
                    characterLimit: promptCharacterLimit,
                    isWorking: step.isWorking
                ) {
                    flowTask = Task { await generatePersona() }
                }
            case .personaReview, .generatingMetadata:
                PersonaReviewForm(
                    instructionsText: $instructionsText,
                    isWorking: step.isWorking
                ) {
                    flowTask = Task { await generateMetadata() }
                }
            case let .reviewProfile(profile), let .creating(profile):
                ProfileReviewForm(
                    profile: profile,
                    isWorking: step.isWorking
                ) {
                    flowTask = Task { await createAccount(profile: profile) }
                }
            case .done:
                doneView
            }

            if let workingStatusText = step.workingStatusText {
                Text(workingStatusText)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 16)
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
        // Pull-down can never discard progress: the gesture is disabled
        // whenever the wizard holds content (typed text, a generated
        // persona, an in-flight call) — Cancel, with its discard
        // confirmation, is the single exit. A blank first screen and the
        // done screen keep the gesture (nothing to lose).
        .interactiveDismissDisabled(hasWizardContent)
        .confirmationDialog(
            "Discard this AI?",
            isPresented: $isDiscardDialogShown,
            titleVisibility: .visible
        ) {
            Button("Discard and close", role: .destructive) { dismiss() }
            Button("Keep creating", role: .cancel) {}
        } message: {
            Text("Your progress so far will be lost.")
        }
        #if canImport(UIKit)
            .toolbar(.hidden, for: .navigationBar)
        #endif
    }

    /// Anything the wizard holds that pull-down or Cancel must not
    /// silently discard? A blank first screen (and the done screen)
    /// has nothing to lose; every other step does.
    private var hasWizardContent: Bool {
        switch step {
        case .descriptionEntry:
            return !descriptionText.isBlank
        case .personaReview, .reviewProfile,
             .generatingPersona, .generatingMetadata, .creating:
            return true
        case .done:
            return false
        }
    }

    private func requestDismiss() {
        // While a step runs, Cancel stops the in-flight call (a retry
        // resumes where it stopped) instead of discarding the wizard.
        if step.isWorking {
            flowTask?.cancel()
        } else if hasWizardContent {
            isDiscardDialogShown = true
        } else {
            dismiss()
        }
    }

    private func goBack() {
        switch step {
        case .personaReview:
            step = .descriptionEntry
        case .reviewProfile:
            step = .personaReview
        case .descriptionEntry, .generatingPersona, .generatingMetadata,
             .creating, .done:
            break
        }
    }

    /// True when the error is the user's own Cancel from the loading
    /// step — surfaced as a quiet return, not an error message.
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

    // MARK: - Actions

    private func generatePersona() async {
        errorMessage = nil
        step = .generatingPersona
        do {
            guard let idToken = authClient.idToken else {
                throw AuthError.oauthFailed(errorDescription: "Not signed in")
            }
            let instructions = try await influencerDataSource.generatePrompt(
                prompt: descriptionText,
                idToken: idToken
            )
            instructionsText = instructions
            step = .personaReview
        } catch {
            step = .descriptionEntry
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
    }

    private func generateMetadata() async {
        errorMessage = nil
        step = .generatingMetadata
        do {
            guard let idToken = authClient.idToken else {
                throw AuthError.oauthFailed(errorDescription: "Not signed in")
            }
            let metadata = try await influencerDataSource.validateAndGenerateMetadata(
                systemInstructions: instructionsText,
                idToken: idToken
            )
            guard metadata.isValid,
                  let name = metadata.name,
                  let displayName = metadata.displayName,
                  let avatarURL = metadata.avatarURL
            else {
                step = .personaReview
                errorMessage = metadata.validationReason ?? "The AI's instructions were rejected — edit and retry."
                return
            }
            let profile = AIProfileDetails(
                systemInstructions: instructionsText,
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
            step = .reviewProfile(profile)
        } catch {
            step = .personaReview
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
    }

    private func createAccount(profile: AIProfileDetails) async {
        errorMessage = nil
        step = .creating(profile)
        // Resume an in-flight creation for the SAME profile if a previous
        // attempt failed midway (Kotlin BotCreationProgress semantics).
        if creationProgress?.profileKey != profile.profileKey {
            creationProgress = AICreationProgress(profileKey: profile.profileKey)
        }
        guard var progress = creationProgress else { return }
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
            creationProgress = nil
            step = .done
        } catch {
            // A cancelled run keeps the partially-completed progress
            // record — a retry resumes where the pipeline stopped.
            creationProgress = progress
            step = .reviewProfile(profile)
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
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

/// The wizard's step machine — navigation semantics live ON the step
/// (which screens show the header/back button, when the sheet's
/// swipe-down is disabled) so the view reads them straight off `step`.
private enum FlowStep {
    case descriptionEntry
    case generatingPersona
    case personaReview
    case generatingMetadata
    case reviewProfile(AIProfileDetails)
    case creating(AIProfileDetails)
    case done

    /// The header shows on every step (its Cancel is the single exit);
    /// only the done screen drops it for its own Done button.
    var showsHeader: Bool {
        switch self {
        case .done:
            return false
        case .descriptionEntry, .generatingPersona, .personaReview,
             .generatingMetadata, .reviewProfile, .creating:
            return true
        }
    }

    /// No reworking steps while a call runs — Cancel stops it first.
    var showsBackButton: Bool {
        switch self {
        case .personaReview, .reviewProfile:
            return true
        case .descriptionEntry, .generatingPersona, .generatingMetadata,
             .creating, .done:
            return false
        }
    }

    /// True while a step's network call runs — its form stays on screen
    /// with the button spinner; the header's Cancel stops the call.
    var isWorking: Bool {
        switch self {
        case .generatingPersona, .generatingMetadata, .creating:
            return true
        case .descriptionEntry, .personaReview, .reviewProfile, .done:
            return false
        }
    }

    /// Reassurance under the spinning button for the long LLM calls
    /// (the metadata call can legitimately run to its 90s timeout).
    var workingStatusText: String? {
        switch self {
        case .generatingPersona:
            return "Your AI is thinking…"
        case .generatingMetadata:
            return "Writing your AI's profile…"
        case .creating:
            return "Creating your AI account…"
        case .descriptionEntry, .personaReview, .reviewProfile, .done:
            return nil
        }
    }
}
