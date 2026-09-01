import SwiftUI

/// AI account creation — SwiftUI port of the Kotlin AiInfluencer wizard
/// (`AiInfluencerViewModel` steps condensed): describe the persona →
/// review/edit the generated instructions → creating (the multi-step
/// pipeline runs; a failure offers retry, which resumes where it failed).
///
/// No view model — state lives HERE as @State; the flow calls
/// `AIInfluencerDataSource` (persona LLM) and `AIAccountCreator` (the
/// creation pipeline) directly.
struct AIAccountCreationView: View {

    let authClient: AuthClient
    let sessionStore: SessionStore

    // MARK: - Flow state

    private enum FlowStep {
        case descriptionEntry
        case generatingPersona
        case personaReview(instructions: String)
        case generatingMetadata(instructions: String)
        case reviewProfile(AIProfileDetails)
        case creating
        case done
    }

    @State private var step: FlowStep = .descriptionEntry
    @State private var descriptionText = ""
    @State private var instructionsText = ""
    @State private var errorMessage: String?
    @State private var creationProgress: AICreationProgress?
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
            switch step {
            case .descriptionEntry:
                descriptionEntry
            case .generatingPersona, .generatingMetadata, .creating:
                loading
            case let .personaReview(instructions):
                personaReview(instructions)
            case let .reviewProfile(profile):
                profileReview(profile)
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
        .padding(.top, 46)
        .padding(.bottom, 16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Color.black)
        #if canImport(UIKit)
            .toolbar(.hidden, for: .navigationBar)
        #endif
    }

    // MARK: - Step 1: describe the persona

    private var descriptionEntry: some View {
        DescriptionEntryForm(
            descriptionText: $descriptionText,
            characterLimit: promptCharacterLimit
        ) {
            Task { await generatePersona() }
        }
    }

    // MARK: - Step 2: review the generated persona

    private func personaReview(_ instructions: String) -> some View {
        PersonaReviewForm(instructionsText: $instructionsText) {
            Task { await generateMetadata() }
        }
    }

    // MARK: - Step 3: review the profile

    private func profileReview(_ profile: AIProfileDetails) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(profile.displayName)
                .font(.title2.weight(.semibold))

            if let avatarURL = URL(string: profile.avatarURL) {
                AsyncImage(url: avatarURL) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color.gray.opacity(0.25)
                }
                .frame(width: 96, height: 96)
                .clipShape(Circle())
            }

            VStack(alignment: .leading, spacing: 6) {
                Text(profile.description)
                    .font(.subheadline)
                if !profile.suggestedMessages.isEmpty {
                    Text("Says things like: \(profile.suggestedMessages.prefix(3).joined(separator: " • "))")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
                Text("Category: \(profile.category)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            Button {
                Task { await createAccount(profile: profile) }
            } label: {
                Text("Create AI account")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
        }
    }

    // MARK: - Loading + done

    private var loading: some View {
        VStack(spacing: 16) {
            ProgressView()
                .controlSize(.large)
            Text("Your AI is thinking…")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Button("Cancel") {
                step = .descriptionEntry
                errorMessage = nil
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
            errorMessage = errorText(of: error)
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
            errorMessage = errorText(of: error)
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
            creationProgress = progress
            step = .reviewProfile(profile)
            errorMessage = errorText(of: error)
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

/// Step 1 — describe the AI account (Kotlin DescriptionEntry).
private struct DescriptionEntryForm: View {
    @Binding var descriptionText: String
    let characterLimit: Int
    let onContinue: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Create your AI")
                .font(.title2.weight(.semibold))
            Text("Describe the AI account you want — personality, style, what it posts about.")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextField(
                "e.g. A witty travel photographer sharing hidden gems…",
                text: $descriptionText,
                axis: .vertical
            )
            .lineLimit(5...10)
            .padding(12)
            .background(
                Color.gray.opacity(0.2),
                in: RoundedRectangle(cornerRadius: 8)
            )
            .onChange(of: descriptionText) { _, newValue in
                if newValue.count > characterLimit {
                    descriptionText = String(newValue.prefix(characterLimit))
                }
            }

            Button(action: onContinue) {
                Text("Continue")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(descriptionText.isBlank)
        }
    }
}

/// Step 2 — review/edit the generated persona instructions
/// (Kotlin PersonaReview).
private struct PersonaReviewForm: View {
    @Binding var instructionsText: String
    let onContinue: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Persona")
                .font(.title2.weight(.semibold))
            Text("Edit the AI account's instructions, then continue to generate its profile.")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextEditor(text: $instructionsText)
                .font(.body)
                .frame(maxHeight: 220)
                .scrollContentBackground(.hidden)
                .padding(8)
                .background(
                    Color.gray.opacity(0.2),
                    in: RoundedRectangle(cornerRadius: 8)
                )

            Button(action: onContinue) {
                Text("Generate profile")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(instructionsText.isBlank)
        }
    }
}
