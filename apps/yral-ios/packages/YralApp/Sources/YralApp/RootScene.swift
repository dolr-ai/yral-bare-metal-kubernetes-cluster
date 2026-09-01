import SwiftUI

/// The root SwiftUI scene content for the Yral app.
///
/// Phase 0: a placeholder surface proving the full toolchain — SPM package →
/// thin Xcode shell → simulator/device/TestFlight — is wired end-to-end.
/// Feature phases (auth, feed, chat, profile, …) replace this root content
/// slice by slice.
struct RootScene: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48))
                .foregroundStyle(.tint)
            Text("Yral")
                .font(.largeTitle.bold())
            Text("Native SwiftUI app — Phase 0 scaffold")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .padding()
    }
}

#Preview {
    RootScene()
}
