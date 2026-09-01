import SwiftUI

/// Profile — the CURRENT account's details (avatar, username, bio,
/// principal). Phase 4 builds the real profile surface (posts grid,
/// stats, edit); this placeholder is the tab's anchor until then,
/// showing live session data.
struct ProfileView: View {

    let sessionStore: SessionStore

    var body: some View {
        VStack(spacing: 12) {
            if let profilePicURL = sessionStore.profilePic,
               let url = URL(string: profilePicURL) {
                AsyncImage(url: url) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color.gray.opacity(0.25)
                }
                .frame(width: 88, height: 88)
                .clipShape(Circle())
            }
            Text(sessionStore.username ?? "Anonymous")
                .font(.title3.weight(.semibold))
            if let principal = sessionStore.userPrincipal {
                Text(principal)
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .padding(.horizontal, 32)
            }
            if sessionStore.isAIAccount == true {
                Text("AI account")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(.pink.opacity(0.2), in: Capsule())
                    .foregroundStyle(.pink)
            }
            Text("Full profile arrives in Phase 4")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
    }
}

#Preview {
    ProfileView(sessionStore: SessionStore())
}
