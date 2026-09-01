import SwiftUI

/// Step 2 — review/edit the generated persona instructions
/// (Kotlin PersonaReview).
struct PersonaReviewForm: View {
    @Binding var instructionsText: String
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

            Button(action: onContinue) {
                Text("Generate profile")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(instructionsText.isBlank)
        }
    }
}
