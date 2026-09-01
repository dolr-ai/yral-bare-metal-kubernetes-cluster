import SwiftUI

/// Country selector — SwiftUI port of Kotlin `CountrySelectorScreen.kt`:
/// search bar + flag/name/dial-code list; tapping a row returns it to the
/// sign-in screen. State lives HERE (@State) — search filters
/// `CountriesDataSource.searchCountries` (name/dial-code/code match).
struct CountrySelectorView: View {

    /// Selection callback — the sign-in screen passes a binding-style
    /// closure (Kotlin `CountrySelectorComponent.onCountrySelected`).
    let onSelect: (Country) -> Void
    let onBack: () -> Void

    @State private var searchQuery = ""
    @Environment(\.dismiss) private var dismiss

    private var countries: [Country] {
        CountriesDataSource.searchCountries(query: searchQuery)
    }

    var body: some View {
        VStack(spacing: 0) {
            header

            searchField

            List {
                ForEach(countries) { country in
                    Button {
                        onSelect(country)
                        dismiss()
                    } label: {
                        CountryRow(country: country)
                    }
                    .buttonStyle(.plain)
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
        }
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
        #if canImport(UIKit)
            .toolbar(.hidden, for: .navigationBar)
        #endif
    }

    // MARK: - Header (Kotlin Header)

    private var header: some View {
        ZStack {
            Text("Country")
                .font(.title3.bold())
                .foregroundStyle(.white)
            HStack {
                Button {
                    onBack()
                    dismiss()
                } label: {
                    Image(systemName: "chevron.left")
                        .font(.headline)
                        .foregroundStyle(.white)
                }
                Spacer()
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    // MARK: - Search (Kotlin SearchBar)

    private var searchField: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(.white.opacity(0.5))
            TextField("Search by country name", text: $searchQuery)
                .foregroundStyle(.white)
                #if canImport(UIKit)
                    .autocapitalization(.none)
                    .autocorrectionDisabled()
                #endif
        }
        .padding(.horizontal, 12)
        .frame(height: 44)
        .background(Color(white: 0.13), in: RoundedRectangle(cornerRadius: 8))
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

/// One list row — Kotlin `CountryListItem`: flag, name, dial code.
private struct CountryRow: View {
    let country: Country

    var body: some View {
        HStack(spacing: 8) {
            AsyncImage(url: country.flagURL) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Color(white: 0.2)
            }
            .frame(width: 32, height: 23)
            .clipShape(RoundedRectangle(cornerRadius: 4))

            Text(country.name)
                .foregroundStyle(.white)
                .frame(maxWidth: .infinity, alignment: .leading)

            Text(country.dialCode)
                .foregroundStyle(.white.opacity(0.55))
        }
        .padding(.vertical, 8)
        .listRowBackground(Color(red: 0.04, green: 0.04, blue: 0.06))
        .listRowSeparatorTint(Color(white: 0.28))
    }
}

#Preview {
    CountrySelectorView(
        onSelect: { _ in },
        onBack: {}
    )
}
