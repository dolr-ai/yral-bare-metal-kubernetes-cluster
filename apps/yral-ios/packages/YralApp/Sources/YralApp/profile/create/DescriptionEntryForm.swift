import SwiftUI

/// Step 1 — describe the AI account (Kotlin DescriptionEntry). While
/// the persona generates, the form stays put and Continue becomes a
/// spinner (inline loading — no full-screen waiting step).
struct DescriptionEntryForm: View {
    @Binding var descriptionText: String
    let characterLimit: Int
    let isWorking: Bool
    let onContinue: () -> Void

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
