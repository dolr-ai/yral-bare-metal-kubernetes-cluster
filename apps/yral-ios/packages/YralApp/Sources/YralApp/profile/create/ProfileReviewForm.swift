import SwiftUI

/// Step 3 — review the generated profile before committing to the
/// creation pipeline (Kotlin ProfileReview). While the pipeline runs,
/// the form stays put and the button becomes a spinner (inline
/// loading); a retry resumes where the pipeline stopped.
struct ProfileReviewForm: View {
    let profile: AIProfileDetails
    let isWorking: Bool
    let onCreate: () -> Void

    // TODO(static-placeholder-avatars): ship ~20 static avatar images
    // IN THE BUNDLE and assign one to each profile deterministically
    // (index = hash(name) % 20 or similar — same profile → same image,
    // no network dependency at all). The metadata API's remote
    // avatarURL (fetched via AsyncImage here) is a stopgap: it depends
    // on the avatar service being up and the URL staying valid.
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(spacing: 12) {
                Text(profile.displayName)
                    .font(.title2.weight(.semibold))

                if let avatarURL = URL(string: profile.avatarURL) {
                    AsyncImage(url: avatarURL) { image in
                        image.resizable().scaledToFill()
                    } placeholder: {
                        Color.gray.opacity(0.25)
                    }
                    .frame(width: 140, height: 140)
                    .clipShape(Circle())
                }
            }
            .frame(maxWidth: .infinity)

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
