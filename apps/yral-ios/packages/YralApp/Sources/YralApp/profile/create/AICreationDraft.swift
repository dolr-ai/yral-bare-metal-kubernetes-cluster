import Foundation

/// The create wizard's whole state — owned by MainTabView (OUTSIDE the
/// sheet) so pulling down to leave never loses it: tapping Create again
/// resumes exactly where the user left off (operator request
/// 2026-09-01). Reset inside the sheet is the explicit clear.
///
/// The two source fields are this draft's generation cache: each records
/// which input its artifact was generated from, so re-advancing with
/// UNCHANGED input skips the API and reuses the artifact.
struct AICreationDraft {
    var step: FlowStep = .descriptionEntry
    var descriptionText = ""
    var instructionsText = ""
    /// The profile generated from `profileSourceInstructions` (step 3).
    var profileUnderReview: AIProfileDetails?
    /// Resumable creation record — a stopped/failed pipeline resumes
    /// where it left off (Kotlin BotCreationProgress semantics).
    var creationProgress: AICreationProgress?
    /// Cache: the description `instructionsText` was generated from.
    var personaSourceDescription: String?
    /// Cache: the instructions `profileUnderReview` was generated from.
    var profileSourceInstructions: String?
}

/// The wizard's step machine — navigation semantics live ON the step
/// (which screens show the header/back button, when a form's inputs are
/// locked) so the view reads them straight off `step`. Steps carry no
/// payload — the draft holds the data — so the whole flow is one
/// portable value.
enum FlowStep: Equatable {
    case descriptionEntry
    case generatingPersona
    case personaReview
    case generatingMetadata
    case reviewProfile
    case creating
    case done

    /// The header shows on every step (its Reset clears the draft);
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

    /// No reworking steps while a call runs — Reset stops it first.
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
    /// with the button spinner; Reset stops the call.
    var isWorking: Bool {
        switch self {
        case .generatingPersona, .generatingMetadata, .creating:
            return true
        case .descriptionEntry, .personaReview, .reviewProfile, .done:
            return false
        }
    }
}
