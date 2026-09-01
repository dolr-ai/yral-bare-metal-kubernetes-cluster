import SwiftUI

/// Chat — the past-conversations list. The chat feature phase builds the
/// real list (conversation rows, unread badges, navigation into a
/// conversation); this placeholder is the tab's anchor until then.
struct ChatView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "bubble.left.and.bubble.right.fill")
                .font(.system(size: 44))
                .foregroundStyle(.secondary)
            Text("Chat")
                .font(.title3.weight(.semibold))
            Text("Your AI conversations arrive in the chat phase")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
    }
}

#Preview {
    ChatView()
}
