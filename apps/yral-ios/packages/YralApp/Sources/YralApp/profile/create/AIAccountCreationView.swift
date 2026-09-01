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
/// Navigation chrome: the header's Cancel leaves the sheet (with a
/// discard confirmation once anything is typed/generated), the
/// chevron-back reworks earlier steps, and the loading step's Cancel
/// STOPS the in-flight call (swipe-down is disabled while it runs).
/// The sheet's grabber (see MainTabView) signals pullability.
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
            case .descriptionEntry:
                DescriptionEntryForm(
                    descriptionText: $descriptionText,
                    characterLimit: promptCharacterLimit
                ) {
                    flowTask = Task { await generatePersona() }
                }
            case .generatingPersona, .generatingMetadata, .creating:
                loading
            case .personaReview:
                PersonaReviewForm(instructionsText: $instructionsText) {
                    flowTask = Task { await generateMetadata() }
                }
            case let .reviewProfile(profile):
                ProfileReviewForm(profile: profile) {
                    flowTask = Task { await createAccount(profile: profile) }
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
        // Swipe-down while a generation/creation runs would silently kill
        // an in-flight pipeline — the gesture is disabled then, and the
        // loading step's own Cancel button is the way out.
        .interactiveDismissDisabled(step.isGenerationInFlight)
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

    /// Anything worth a discard confirmation? A blank first screen
    /// dismisses immediately; typed text or a generated persona asks.
    private var hasWizardContent: Bool {
        switch step {
        case .descriptionEntry:
            return !descriptionText.isBlank
        case .personaReview, .reviewProfile:
            return true
        case .generatingPersona, .generatingMetadata, .creating, .done:
            return false
        }
    }

    private func requestDismiss() {
        if hasWizardContent {
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
            step = .personaReview(instructions: instructionsText)
        case .descriptionEntry, .generatingPersona, .generatingMetadata, .creating, .done:
            break
        }
    }

    /// True when the error is the user's own Cancel from the loading
    /// step — surfaced as a quiet return, not an error message.
    private func isUserCancellation(_ error: Error) -> Bool {
        error is CancellationError || (error as? URLError)?.code == .cancelled
    }

    // MARK: - Loading + done

    private var loading: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
            Text("Your AI is thinking…")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            // Actually stops the in-flight call (cancellation propagates
            // into the task's URLSession awaits); the step's catch returns
            // to the previous screen without an error message.
            Button("Cancel") {
                flowTask?.cancel()
            }
            .font(.subheadline)
        }
        .padding(.top, 120)
    }

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
            step = .personaReview(instructions: instructions)
        } catch {
            step = .descriptionEntry
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
    }

    private func generateMetadata() async {
        errorMessage = nil
        step = .generatingMetadata(instructions: instructionsText)
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
                step = .personaReview(instructions: instructionsText)
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
            step = .personaReview(instructions: instructionsText)
            if !isUserCancellation(error) {
                errorMessage = errorText(of: error)
            }
        }
    }

    private func createAccount(profile: AIProfileDetails) async {
        errorMessage = nil
        step = .creating
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
    case personaReview(instructions: String)
    case generatingMetadata(instructions: String)
    case reviewProfile(AIProfileDetails)
    case creating
    case done

    /// Header on the three interactive steps only — loading steps carry
    /// their own Cancel, and Done dismisses.
    var showsHeader: Bool {
        switch self {
        case .descriptionEntry, .personaReview, .reviewProfile:
            return true
        case .generatingPersona, .generatingMetadata, .creating, .done:
            return false
        }
    }

    var showsBackButton: Bool {
        switch self {
        case .personaReview, .reviewProfile:
            return true
        case .descriptionEntry, .generatingPersona, .generatingMetadata, .creating, .done:
            return false
        }
    }

    /// True while a network step runs — swipe-down is disabled then.
    var isGenerationInFlight: Bool {
        switch self {
        case .generatingPersona, .generatingMetadata, .creating:
            return true
        case .descriptionEntry, .personaReview, .reviewProfile, .done:
            return false
        }
    }
}
