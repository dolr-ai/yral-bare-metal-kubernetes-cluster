import SwiftUI

/// Dial-code prefix + digit-only number field — Kotlin `PhoneInputField`:
/// digits filtered and capped at the country's max length.
///
/// One component per file (the naming rule), so `CountryPickerButtonComponent`
/// lives in its own file beside this one.
struct PhoneInputRowComponent: View {
  @Binding var nationalNumber: String
  let selectedCountry: Country?
  let isError: Bool

  var body: some View {
    HStack(spacing: 8) {
      Text(selectedCountry?.dialCode ?? "+1")
        .font(.subheadline.weight(.semibold))
      TextField(
        "Enter mobile number",
        text: Binding(
          get: { nationalNumber },
          set: { newValue in
            let maximumLength = selectedCountry?.maxLength ?? 15
            nationalNumber = String(
              newValue.filter(\.isNumber).prefix(maximumLength)
            )
          }
        )
      )
      #if canImport(UIKit)
        .keyboardType(.numberPad)
      #endif
    }
    .padding(.horizontal, 12)
    .frame(height: 44)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(
      Color.gray.opacity(0.2),
      in: RoundedRectangle(cornerRadius: 8)
    )
    .overlay(
      RoundedRectangle(cornerRadius: 8)
        .stroke(
          isError ? Color.pink : Color.gray.opacity(0.35),
          lineWidth: 1
        )
    )
  }
}

#if DEBUG
  #Preview("empty") {
    PhoneInputRowComponent(
      nationalNumber: .constant(""),
      selectedCountry: Country(code: "IN", name: "India", dialCode: "+91"),
      isError: false
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
  }

  #Preview("error") {
    PhoneInputRowComponent(
      nationalNumber: .constant("98"),
      selectedCountry: Country(code: "IN", name: "India", dialCode: "+91"),
      isError: true
    )
    .padding(16)
    .background(Color.black)
    .preferredColorScheme(.dark)
  }
#endif
