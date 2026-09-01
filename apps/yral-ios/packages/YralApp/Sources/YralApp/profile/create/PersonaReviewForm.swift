import SwiftUI

/// Step 2 — review/edit the generated persona instructions
/// (Kotlin PersonaReview). While the profile generates, the form stays
/// put and the button becomes a spinner (inline loading).
struct PersonaReviewForm: View {
    @Binding var instructionsText: String
    let isWorking: Bool
    let onContinue: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Persona")
                .font(.title2.weight(.semibold))
            Text("Edit the AI account's instructions, then continue to generate its profile.")
                .font(.subheadline)
                .foregroundStyle(.secondary)

            TextEditor(text: $instructionsText)
                .font(.body)
                .frame(maxHeight: 220)
                .scrollContentBackground(.hidden)
                .padding(8)
                .background(
                    Color.gray.opacity(0.2),
                    in: RoundedRectangle(cornerRadius: 8)
                )
                .disabled(isWorking)

            Button(action: onContinue) {
                Group {
                    if isWorking {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Generate profile")
                            .frame(maxWidth: .infinity)
                    }
                }
                .font(.headline)
                .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(isWorking || instructionsText.isBlank)
        }
    }
}
