import SwiftUI

/// Home feed — the AI-influencer feed. Phase 3 builds the real feed
/// (video surface, recsys calls); this placeholder is the tab's anchor
/// until then.
struct HomeFeedView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "play.rectangle.fill")
                .font(.system(size: 44))
                .foregroundStyle(.secondary)
            Text("Home feed")
                .font(.title3.weight(.semibold))
            Text("The AI-influencer feed arrives in Phase 3")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color.black)
    }
}

#Preview {
    HomeFeedView()
}
