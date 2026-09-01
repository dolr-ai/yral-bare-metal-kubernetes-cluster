import SwiftUI

/// Step 1 — describe the AI account (Kotlin DescriptionEntry). While
/// the persona generates, the form stays put and Continue becomes a
/// spinner (inline loading — no full-screen waiting step).
struct DescriptionEntryForm: View {
    @Binding var descriptionText: String
    let characterLimit: Int
    let isWorking: Bool
    let onContinue: () -> Void

    // TODO(textfield-keystroke-lag): typing in this field is laggy —
    // reported since the wizard's first build (NOT the draft hoisting;
    // reproduced before it). Xcode console shows system-keyboard noise
    // only (TUIKeyboardContentView constraint conflicts, variant
    // selector lookup failures, `Result accumulator timeout: 3.000000
    // exceeded` — the input system giving up on a 3s keystroke
    // round-trip, confirming the lag but pointing at no app code).
    // Nothing in our text pipeline does per-keystroke work: the only
    // per-keystroke code is the O(1) limit clamp below. Suspects to
    // profile with Instruments (Hitch/Time Profiler) when
    // investigating:
    //   - debug-build simulator: unoptimized SwiftUI diffing + keyboard
    //     IPC over the simulator bridge (test on a physical device /
    //     Release build first to rule this out)
    //   - .onChange writing the binding back into the draft (@State in
    //     MainTabView since the hoisting) could add per-keystroke
    //     shell re-diffs — but the lag predates the hoisting, so this
    //     is an aggravator at most, not the root cause
    //   - axis: .vertical TextField re-laying out every line growth

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Create your AI")
                .font(.title2.weight(.semibold))
            Text("Describe the AI account you want — personality, style, what it posts about.")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextField(
                "e.g. A witty travel photographer sharing hidden gems…",
                text: $descriptionText,
                axis: .vertical
            )
            .lineLimit(5...10)
            .padding(12)
            .background(
                Color.gray.opacity(0.2),
                in: RoundedRectangle(cornerRadius: 8)
            )
            .onChange(of: descriptionText) { _, newValue in
                if newValue.count > characterLimit {
                    descriptionText = String(newValue.prefix(characterLimit))
                }
            }
            .disabled(isWorking)

            Button(action: onContinue) {
                HStack(spacing: 8) {
                    if isWorking {
                        ProgressView()
                    }
                    Text(isWorking ? "Your AI is thinking…" : "Continue")
                }
                .font(.headline)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(isWorking || descriptionText.isBlank)
        }
    }
}

#if DEBUG
#Preview("idle") {
    DescriptionEntryForm(
        descriptionText: .constant("A witty travel photographer sharing hidden gems"),
        characterLimit: 400,
        isWorking: false,
        onContinue: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
}

#Preview("working (inline spinner)") {
    DescriptionEntryForm(
        descriptionText: .constant("A witty travel photographer"),
        characterLimit: 400,
        isWorking: true,
        onContinue: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
}
#endif
