import SwiftUI

/// Step 3 — review the generated profile before committing to the
/// creation pipeline (Kotlin ProfileReview). While the pipeline runs,
/// the form stays put and the button becomes a spinner (inline
/// loading); a retry resumes where the pipeline stopped. On success
/// the button flips to a tick + "Go to Profile" — NO separate done
/// screen; the celebration (confetti + horn) plays over THIS form
/// (operator request 2026-09-01).
struct ProfileReviewForm: View {
    @Binding var profile: AIProfileDetails
    let isWorking: Bool
    let hasSucceeded: Bool
    let onCreate: () -> Void
    let onGoToProfile: () -> Void

    // TODO(static-placeholder-avatars): ship ~20 static avatar images
    // IN THE BUNDLE and assign one to each profile deterministically
    // (index = hash(name) % 20 or similar — same profile → same image,
    // no network dependency at all). The metadata API's remote
    // avatarURL (fetched via AsyncImage here) is a stopgap: it depends
    // on the avatar service being up and the URL staying valid.
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(spacing: 12) {
                if let avatarURL = URL(string: profile.avatarURL) {
                    AsyncImage(url: avatarURL) { image in
                        image.resizable().scaledToFill()
                    } placeholder: {
                        Color.gray.opacity(0.25)
                    }
                    .frame(width: 240, height: 240)
                    .clipShape(Circle())
                }
            }
            .frame(maxWidth: .infinity)

            // EDITABLE USERNAME — the account handle is the one field
            // the backend OWNS uniqueness on ("Name … already taken");
            // a collision means: hit back, edit it, retry — the retry
            // resumes the same creation (no re-mint) with the new name.
            VStack(alignment: .leading, spacing: 4) {
                Text("@\(profile.name)")
                    .font(.title2.weight(.semibold))
                TextField("username", text: $profile.name)
                    .font(.subheadline)
                    .padding(10)
                    .background(
                        Color.gray.opacity(0.2),
                        in: RoundedRectangle(cornerRadius: 8)
                    )
                    .disabled(isWorking || hasSucceeded)
                    .onChange(of: profile.name) { _, newValue in
                        // Usernames are lowercase alphanumeric +
                        // underscore; spaces and uppercase are stripped
                        // as typed (server enforces the format too).
                        let sanitized = newValue
                            .lowercased()
                            .filter { $0.isLetter || $0.isNumber || $0 == "_" }
                        if sanitized != newValue {
                            profile.name = sanitized
                        }
                    }
                Text(profile.displayName)
                    .font(.caption)
                    .foregroundStyle(.secondary)
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

            createButton
        }
    }

    /// The button's states — waiting (inline spinner) → success (tick
    /// + "Go to Profile"). Success animates ON THIS FORM (no extra
    /// screen — operator request 2026-09-01).
    @ViewBuilder
    private var createButton: some View {
        if hasSucceeded {
            Button(action: onGoToProfile) {
                HStack(spacing: 8) {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.headline)
                    Text("Go to Profile")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .transition(.scale.combined(with: .opacity))
        } else {
            Button(action: onCreate) {
                HStack(spacing: 8) {
                    if isWorking {
                        ProgressView()
                    }
                    Text(isWorking ? "Creating your AI account…" : "Create AI account")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(isWorking)
        }
    }
}

#if DEBUG
#Preview("idle") {
    ProfileReviewForm(
        profile: .constant(
            AIProfileDetails(
                systemInstructions: "You are a witty travel photographer…",
                name: "wander_lens",
                displayName: "Wander Lens",
                description: "A witty travel photographer sharing hidden gems and offbeat stories from the road.",
                avatarURL: "https://images.yral.com/avatar.png",
                initialGreeting: "Hey! Ready for hidden gems?",
                suggestedMessages: ["Show me a hidden gem", "What's your funniest travel fail?"],
                personalityTraits: ["wit": "high"],
                category: "travel",
                isNSFW: false
            )
        ),
        isWorking: false,
        hasSucceeded: false,
        onCreate: {},
        onGoToProfile: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
}

#Preview("working (inline spinner)") {
    ProfileReviewForm(
        profile: .constant(
            AIProfileDetails(
                systemInstructions: "You are a witty travel photographer…",
                name: "wander_lens",
                displayName: "Wander Lens",
                description: "A witty travel photographer sharing hidden gems.",
                avatarURL: "https://images.yral.com/avatar.png",
                initialGreeting: "Hey!",
                suggestedMessages: [],
                personalityTraits: [:],
                category: "travel",
                isNSFW: false
            )
        ),
        isWorking: true,
        hasSucceeded: false,
        onCreate: {},
        onGoToProfile: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
}

#Preview("succeeded (tick + Go to Profile)") {
    ProfileReviewForm(
        profile: .constant(
            AIProfileDetails(
                systemInstructions: "You are a witty travel photographer…",
                name: "wander_lens",
                displayName: "Wander Lens",
                description: "A witty travel photographer sharing hidden gems.",
                avatarURL: "https://images.yral.com/avatar.png",
                initialGreeting: "Hey!",
                suggestedMessages: [],
                personalityTraits: [:],
                category: "travel",
                isNSFW: false
            )
        ),
        isWorking: false,
        hasSucceeded: true,
        onCreate: {},
        onGoToProfile: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
}
#endif
