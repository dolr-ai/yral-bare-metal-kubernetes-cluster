import SwiftUI

/// The creation wizard's own chrome — Reset (and chevron-back when an
/// earlier step exists). Reset is the explicit clear of the whole draft
/// (confirmed when it holds anything); pulling down just LEAVES — the
/// draft lives in MainTabView and resumes on the next Create tap.
struct AICreationHeader: View {
    let showsBackButton: Bool
    let onBack: () -> Void
    let onReset: () -> Void

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
            Button("Reset", action: onReset)
        }
    }
}
