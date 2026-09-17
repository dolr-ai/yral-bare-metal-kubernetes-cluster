import SwiftUI

/// Country picker button — Kotlin `CountryPickerButton`: flag + dial code
/// + chevron; opens `CountrySelectorScreen`.
///
/// Colocated with `SignInScreen` (the only screen that uses it) but kept in
/// its own file so that screen stays within the file-length lint bound.
struct CountryPickerButtonComponent: View {
  let country: Country?
  let action: () -> Void

  var body: some View {
    Button(action: action) {
      HStack(spacing: 6) {
        if let flagURL = country?.flagURL {
          AsyncImage(url: flagURL) { image in
            image.resizable().scaledToFit()
          } placeholder: {
            Color.gray.opacity(0.25)
          }
          .frame(width: 24, height: 16)
          .clipShape(RoundedRectangle(cornerRadius: 2))
        }
        Text(country?.dialCode ?? "+1")
          .font(.subheadline.weight(.semibold))
        Image(systemName: "chevron.down")
          .font(.caption2)
          .foregroundStyle(.secondary)
      }
      .padding(.horizontal, 12)
      .frame(height: 44)
      .background(
        Color.gray.opacity(0.2),
        in: RoundedRectangle(cornerRadius: 8)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 8)
          .stroke(Color.gray.opacity(0.35), lineWidth: 1)
      )
    }
    .buttonStyle(.plain)
  }
}

#if DEBUG
  #Preview("country selected") {
    // Offline-safe: the flag is fetched from flagcdn, so the preview's
    // AsyncImage shows its placeholder rather than depending on the network.
    CountryPickerButtonComponent(
      country: Country(code: "IN", name: "India", dialCode: "+91", maxLength: 10),
      action: {}
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
  }

  #Preview("no country") {
    CountryPickerButtonComponent(country: nil, action: {})
      .padding(16)
      .background(Color.black)
      .preferredColorScheme(.dark)
  }
#endif
