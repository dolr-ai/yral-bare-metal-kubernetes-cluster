import Foundation

// MARK: - Generation actions — cached per input; a stopped run never
// resurrects a Reset draft. Extracted from AIAccountCreationView (file
// lint limits; the actions are the wizard's I/O half — the pure flow
// semantics live on FlowStep/AICreationDraft).

extension AIAccountCreationView {

    func generatePersona() async {
        errorMessage = nil
        // Unchanged description → reuse this draft's earlier generation;
        // the API is hit only when the input changed.
        if draft.personaSourceDescription == draft.descriptionText {
            previousStep = draft.step
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
            previousStep = draft.step
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
            previousStep = draft.step
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
            guard metadata.isValid else {
                draft.step = .personaReview
                errorMessage = metadata.validationReason ?? "The AI's instructions were rejected — edit and retry."
                return
            }
            let profile = Self.profile(from: metadata, systemInstructions: sourceInstructions)
            draft.profileUnderReview = profile
            draft.profileSourceInstructions = sourceInstructions
            previousStep = draft.step
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
        // Resume the in-flight creation for the SAME PERSONA if a
        // previous attempt failed midway (Kotlin BotCreationProgress
        // semantics). Keyed on the persona (instructions + avatar), NOT
        // the name — a name-collision edit retries without re-minting.
        if draft.creationProgress?.personaKey != profile.personaKey {
            draft.creationProgress = AICreationProgress(personaKey: profile.personaKey)
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
            // starts on the next Create tap. The done reveal slides
            // forward from the profile form.
            previousStep = draft.step
            draft = AICreationDraft()
            draft.step = .done
        } catch {
            if isUserCancellation(error) {
                // Reset clears the draft — never resurrect the profile.
                // A pull-down keeps it: restore the review form so a
                // retry resumes the pipeline where it stopped.
                if draft.profileUnderReview?.personaKey == profile.personaKey {
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

    /// Pure — builds the creation payload from validated metadata
    /// (nil when the metadata is missing required fields).
    static func profile(
        from metadata: AIInfluencerMetadata,
        systemInstructions: String
    ) -> AIProfileDetails? {
        guard
            let name = metadata.name,
            let displayName = metadata.displayName,
            let avatarURL = metadata.avatarURL
        else { return nil }
        return AIProfileDetails(
            systemInstructions: systemInstructions,
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
    }
}
