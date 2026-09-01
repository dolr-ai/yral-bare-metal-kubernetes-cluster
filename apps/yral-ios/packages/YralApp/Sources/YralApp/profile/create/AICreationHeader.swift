import SwiftUI

/// The creation wizard's own chrome — Cancel (and chevron-back when an
/// earlier step exists). The sheet's grabber handle (see MainTabView's
/// `.presentationDragIndicator`) is the pull-down cue; this header is
/// the explicit back/cancel affordance on every interactive step.
struct AICreationHeader: View {
    let showsBackButton: Bool
    let onBack: () -> Void
    let onCancel: () -> Void

    var body: some View {
        HStack {
            if showsBackButton {
                Button(action: onBack) {
                    Image(systemName: "chevron.left")
                        .font(.body.weight(.semibold))
                        .frame(width: 44, height: 44, alignment: .leading)
                }
            }
            Spacer()
            Button("Cancel", action: onCancel)
        }
    }
}
