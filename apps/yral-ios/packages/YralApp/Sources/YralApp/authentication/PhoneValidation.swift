import Contacts
import Foundation

/// Country for phone sign-in — 1:1 port of Kotlin `Country`
/// (libs/phone-validation/countries/Country.kt). ISO 3166-1 alpha-2 code,
/// display name, dial code, flag URL, and the national-number length
/// bounds used to filter typed input.
public struct Country: Equatable, Identifiable, Sendable {
    public var code: String
    public var name: String
    public var dialCode: String
    public var flagURL: URL
    /// Minimum national number length (excluding country code).
    public var minLength: Int
    /// Maximum national number length (excluding country code).
    public var maxLength: Int

    public var id: String { code }

    public init(
        code: String,
        name: String,
        dialCode: String,
        minLength: Int = 7,
        maxLength: Int = 15
    ) {
        self.code = code
        self.name = name
        self.dialCode = dialCode
        self.flagURL = Country.flagURL(countryCode: code)
        self.minLength = minLength
        self.maxLength = maxLength
    }

    /// flagcdn.com URL — Kotlin `Country.getFlagUrl` (w80 = 160px wide).
    public static func flagURL(countryCode: String) -> URL {
        URL(string: "https://flagcdn.com/w80/\(countryCode.lowercased()).png")!
    }
}

/// All countries with their sign-in dial data — 1:1 port of Kotlin
/// `CountriesDataSource.getAllCountries()`. Length overrides only where
/// the Kotlin source has them; everything else uses the 7–15 default.
/// All sign-in countries — Kotlin CountriesDataSource verbatim.
/// File-scope data constant (not a function) — the list is 190 entries.
private let allCountriesList: [Country] = [
    Country(code: "AF", name: "Afghanistan", dialCode: "+93"),
    Country(code: "AL", name: "Albania", dialCode: "+355"),
    Country(code: "DZ", name: "Algeria", dialCode: "+213"),
    Country(code: "AD", name: "Andorra", dialCode: "+376"),
    Country(code: "AO", name: "Angola", dialCode: "+244"),
    Country(
        code: "AG", name: "Antigua and Barbuda", dialCode: "+1-268", minLength: 10, maxLength: 10),
    Country(code: "AR", name: "Argentina", dialCode: "+54", minLength: 10, maxLength: 10),
    Country(code: "AM", name: "Armenia", dialCode: "+374"),
    Country(code: "AU", name: "Australia", dialCode: "+61", minLength: 9, maxLength: 9),
    Country(code: "AT", name: "Austria", dialCode: "+43", minLength: 10, maxLength: 13),
    Country(code: "AZ", name: "Azerbaijan", dialCode: "+994", minLength: 9, maxLength: 9),
    Country(code: "BS", name: "Bahamas", dialCode: "+1-242", minLength: 10, maxLength: 10),
    Country(code: "BH", name: "Bahrain", dialCode: "+973", minLength: 8, maxLength: 8),
    Country(code: "BD", name: "Bangladesh", dialCode: "+880", minLength: 10, maxLength: 10),
    Country(code: "BB", name: "Barbados", dialCode: "+1-246", minLength: 10, maxLength: 10),
    Country(code: "BY", name: "Belarus", dialCode: "+375"),
    Country(code: "BE", name: "Belgium", dialCode: "+32"),
    Country(code: "BZ", name: "Belize", dialCode: "+501"),
    Country(code: "BJ", name: "Benin", dialCode: "+229"),
    Country(code: "BT", name: "Bhutan", dialCode: "+975"),
    Country(code: "BO", name: "Bolivia", dialCode: "+591"),
    Country(code: "BA", name: "Bosnia and Herzegovina", dialCode: "+387"),
    Country(code: "BW", name: "Botswana", dialCode: "+267"),
    Country(code: "BR", name: "Brazil", dialCode: "+55"),
    Country(code: "BN", name: "Brunei", dialCode: "+673"),
    Country(code: "BG", name: "Bulgaria", dialCode: "+359"),
    Country(code: "BF", name: "Burkina Faso", dialCode: "+226"),
    Country(code: "BI", name: "Burundi", dialCode: "+257"),
    Country(code: "KH", name: "Cambodia", dialCode: "+855"),
    Country(code: "CM", name: "Cameroon", dialCode: "+237"),
    Country(code: "CA", name: "Canada", dialCode: "+1", minLength: 10, maxLength: 10),
    Country(code: "CV", name: "Cape Verde", dialCode: "+238"),
    Country(code: "CF", name: "Central African Republic", dialCode: "+236"),
    Country(code: "TD", name: "Chad", dialCode: "+235"),
    Country(code: "CL", name: "Chile", dialCode: "+56"),
    Country(code: "CN", name: "China", dialCode: "+86", minLength: 11, maxLength: 11),
    Country(code: "CO", name: "Colombia", dialCode: "+57"),
    Country(code: "KM", name: "Comoros", dialCode: "+269"),
    Country(code: "CG", name: "Congo", dialCode: "+242"),
    Country(code: "CD", name: "Congo (Democratic Republic)", dialCode: "+243"),
    Country(code: "CR", name: "Costa Rica", dialCode: "+506"),
    Country(code: "HR", name: "Croatia", dialCode: "+385"),
    Country(code: "CU", name: "Cuba", dialCode: "+53"),
    Country(code: "CY", name: "Cyprus", dialCode: "+357"),
    Country(code: "CZ", name: "Czech Republic", dialCode: "+420"),
    Country(code: "DK", name: "Denmark", dialCode: "+45"),
    Country(code: "DJ", name: "Djibouti", dialCode: "+253"),
    Country(code: "DM", name: "Dominica", dialCode: "+1-767", minLength: 10, maxLength: 10),
    Country(
        code: "DO", name: "Dominican Republic", dialCode: "+1-809", minLength: 10, maxLength: 10),
    Country(code: "EC", name: "Ecuador", dialCode: "+593"),
    Country(code: "EG", name: "Egypt", dialCode: "+20"),
    Country(code: "SV", name: "El Salvador", dialCode: "+503"),
    Country(code: "GQ", name: "Equatorial Guinea", dialCode: "+240"),
    Country(code: "ER", name: "Eritrea", dialCode: "+291"),
    Country(code: "EE", name: "Estonia", dialCode: "+372"),
    Country(code: "ET", name: "Ethiopia", dialCode: "+251"),
    Country(code: "FJ", name: "Fiji", dialCode: "+679"),
    Country(code: "FI", name: "Finland", dialCode: "+358"),
    Country(code: "FR", name: "France", dialCode: "+33"),
    Country(code: "GA", name: "Gabon", dialCode: "+241"),
    Country(code: "GM", name: "Gambia", dialCode: "+220"),
    Country(code: "GE", name: "Georgia", dialCode: "+995"),
    Country(code: "DE", name: "Germany", dialCode: "+49"),
    Country(code: "GH", name: "Ghana", dialCode: "+233"),
    Country(code: "GR", name: "Greece", dialCode: "+30"),
    Country(code: "GD", name: "Grenada", dialCode: "+1-473", minLength: 10, maxLength: 10),
    Country(code: "GT", name: "Guatemala", dialCode: "+502"),
    Country(code: "GN", name: "Guinea", dialCode: "+224"),
    Country(code: "GW", name: "Guinea-Bissau", dialCode: "+245"),
    Country(code: "GY", name: "Guyana", dialCode: "+592"),
    Country(code: "HT", name: "Haiti", dialCode: "+509"),
    Country(code: "HN", name: "Honduras", dialCode: "+504"),
    Country(code: "HU", name: "Hungary", dialCode: "+36"),
    Country(code: "IS", name: "Iceland", dialCode: "+354"),
    Country(code: "IN", name: "India", dialCode: "+91", minLength: 10, maxLength: 10),
    Country(code: "ID", name: "Indonesia", dialCode: "+62"),
    Country(code: "IR", name: "Iran", dialCode: "+98"),
    Country(code: "IQ", name: "Iraq", dialCode: "+964"),
    Country(code: "IE", name: "Ireland", dialCode: "+353"),
    Country(code: "IL", name: "Israel", dialCode: "+972"),
    Country(code: "IT", name: "Italy", dialCode: "+39"),
    Country(code: "CI", name: "Ivory Coast", dialCode: "+225"),
    Country(code: "JM", name: "Jamaica", dialCode: "+1-876", minLength: 10, maxLength: 10),
    Country(code: "JP", name: "Japan", dialCode: "+81"),
    Country(code: "JO", name: "Jordan", dialCode: "+962"),
    Country(code: "KZ", name: "Kazakhstan", dialCode: "+7"),
    Country(code: "KE", name: "Kenya", dialCode: "+254"),
    Country(code: "KI", name: "Kiribati", dialCode: "+686"),
    Country(code: "KW", name: "Kuwait", dialCode: "+965"),
    Country(code: "KG", name: "Kyrgyzstan", dialCode: "+996"),
    Country(code: "LA", name: "Laos", dialCode: "+856"),
    Country(code: "LV", name: "Latvia", dialCode: "+371"),
    Country(code: "LB", name: "Lebanon", dialCode: "+961"),
    Country(code: "LS", name: "Lesotho", dialCode: "+266"),
    Country(code: "LR", name: "Liberia", dialCode: "+231"),
    Country(code: "LY", name: "Libya", dialCode: "+218"),
    Country(code: "LI", name: "Liechtenstein", dialCode: "+423"),
    Country(code: "LT", name: "Lithuania", dialCode: "+370"),
    Country(code: "LU", name: "Luxembourg", dialCode: "+352"),
    Country(code: "MG", name: "Madagascar", dialCode: "+261"),
    Country(code: "MW", name: "Malawi", dialCode: "+265"),
    Country(code: "MY", name: "Malaysia", dialCode: "+60"),
    Country(code: "MV", name: "Maldives", dialCode: "+960"),
    Country(code: "ML", name: "Mali", dialCode: "+223"),
    Country(code: "MT", name: "Malta", dialCode: "+356"),
    Country(code: "MH", name: "Marshall Islands", dialCode: "+692"),
    Country(code: "MR", name: "Mauritania", dialCode: "+222"),
    Country(code: "MU", name: "Mauritius", dialCode: "+230"),
    Country(code: "MX", name: "Mexico", dialCode: "+52"),
    Country(code: "FM", name: "Micronesia", dialCode: "+691"),
    Country(code: "MD", name: "Moldova", dialCode: "+373"),
    Country(code: "MC", name: "Monaco", dialCode: "+377"),
    Country(code: "MN", name: "Mongolia", dialCode: "+976"),
    Country(code: "ME", name: "Montenegro", dialCode: "+382"),
    Country(code: "MA", name: "Morocco", dialCode: "+212"),
    Country(code: "MZ", name: "Mozambique", dialCode: "+258"),
    Country(code: "MM", name: "Myanmar", dialCode: "+95"),
    Country(code: "NA", name: "Namibia", dialCode: "+264"),
    Country(code: "NR", name: "Nauru", dialCode: "+691"),
    Country(code: "NP", name: "Nepal", dialCode: "+977"),
    Country(code: "NL", name: "Netherlands", dialCode: "+31"),
    Country(code: "NZ", name: "New Zealand", dialCode: "+64"),
    Country(code: "NI", name: "Nicaragua", dialCode: "+505"),
    Country(code: "NE", name: "Niger", dialCode: "+227"),
    Country(code: "NG", name: "Nigeria", dialCode: "+234"),
    Country(code: "KP", name: "North Korea", dialCode: "+850"),
    Country(code: "MK", name: "North Macedonia", dialCode: "+389"),
    Country(code: "NO", name: "Norway", dialCode: "+47"),
    Country(code: "OM", name: "Oman", dialCode: "+968"),
    Country(code: "PK", name: "Pakistan", dialCode: "+92"),
    Country(code: "PW", name: "Palau", dialCode: "+680"),
    Country(code: "PS", name: "Palestine", dialCode: "+970"),
    Country(code: "PA", name: "Panama", dialCode: "+507"),
    Country(code: "PG", name: "Papua New Guinea", dialCode: "+675"),
    Country(code: "PY", name: "Paraguay", dialCode: "+595"),
    Country(code: "PE", name: "Peru", dialCode: "+51"),
    Country(code: "PH", name: "Philippines", dialCode: "+63"),
    Country(code: "PL", name: "Poland", dialCode: "+48"),
    Country(code: "PT", name: "Portugal", dialCode: "+351"),
    Country(code: "QA", name: "Qatar", dialCode: "+974"),
    Country(code: "RO", name: "Romania", dialCode: "+40"),
    Country(code: "RU", name: "Russia", dialCode: "+7"),
    Country(code: "RW", name: "Rwanda", dialCode: "+250"),
    Country(
        code: "KN", name: "Saint Kitts and Nevis", dialCode: "+1-869", minLength: 10, maxLength: 10),
    Country(code: "LC", name: "Saint Lucia", dialCode: "+1-758", minLength: 10, maxLength: 10),
    Country(
        code: "VC", name: "Saint Vincent and the Grenadines",
        dialCode: "+1-784", minLength: 10, maxLength: 10
    ),
    Country(code: "WS", name: "Samoa", dialCode: "+685"),
    Country(code: "SM", name: "San Marino", dialCode: "+378"),
    Country(code: "ST", name: "Sao Tome and Principe", dialCode: "+239"),
    Country(code: "SA", name: "Saudi Arabia", dialCode: "+966"),
    Country(code: "SN", name: "Senegal", dialCode: "+221"),
    Country(code: "RS", name: "Serbia", dialCode: "+381"),
    Country(code: "SC", name: "Seychelles", dialCode: "+248"),
    Country(code: "SL", name: "Sierra Leone", dialCode: "+232"),
    Country(code: "SG", name: "Singapore", dialCode: "+65"),
    Country(code: "SK", name: "Slovakia", dialCode: "+421"),
    Country(code: "SI", name: "Slovenia", dialCode: "+386"),
    Country(code: "SB", name: "Solomon Islands", dialCode: "+677"),
    Country(code: "SO", name: "Somalia", dialCode: "+252"),
    Country(code: "ZA", name: "South Africa", dialCode: "+27"),
    Country(code: "KR", name: "South Korea", dialCode: "+82"),
    Country(code: "SS", name: "South Sudan", dialCode: "+211"),
    Country(code: "ES", name: "Spain", dialCode: "+34"),
    Country(code: "LK", name: "Sri Lanka", dialCode: "+94"),
    Country(code: "SD", name: "Sudan", dialCode: "+249"),
    Country(code: "SR", name: "Suriname", dialCode: "+597"),
    Country(code: "SE", name: "Sweden", dialCode: "+46"),
    Country(code: "CH", name: "Switzerland", dialCode: "+41"),
    Country(code: "SY", name: "Syria", dialCode: "+963"),
    Country(code: "TW", name: "Taiwan", dialCode: "+886"),
    Country(code: "TJ", name: "Tajikistan", dialCode: "+992"),
    Country(code: "TZ", name: "Tanzania", dialCode: "+255"),
    Country(code: "TH", name: "Thailand", dialCode: "+66"),
    Country(code: "TL", name: "Timor-Leste", dialCode: "+670"),
    Country(code: "TG", name: "Togo", dialCode: "+228"),
    Country(code: "TO", name: "Tonga", dialCode: "+676"),
    Country(
        code: "TT", name: "Trinidad and Tobago", dialCode: "+1-868", minLength: 10, maxLength: 10),
    Country(code: "TN", name: "Tunisia", dialCode: "+216"),
    Country(code: "TR", name: "Turkey", dialCode: "+90"),
    Country(code: "TM", name: "Turkmenistan", dialCode: "+993"),
    Country(code: "TV", name: "Tuvalu", dialCode: "+688"),
    Country(code: "UG", name: "Uganda", dialCode: "+256"),
    Country(code: "UA", name: "Ukraine", dialCode: "+380"),
    Country(code: "AE", name: "United Arab Emirates", dialCode: "+971"),
    Country(code: "GB", name: "United Kingdom", dialCode: "+44", minLength: 10, maxLength: 10),
    Country(code: "US", name: "United States", dialCode: "+1", minLength: 10, maxLength: 10),
    Country(code: "UY", name: "Uruguay", dialCode: "+598"),
    Country(code: "UZ", name: "Uzbekistan", dialCode: "+998"),
    Country(code: "VU", name: "Vanuatu", dialCode: "+678"),
    Country(code: "VA", name: "Vatican City", dialCode: "+379"),
    Country(code: "VE", name: "Venezuela", dialCode: "+58"),
    Country(code: "VN", name: "Vietnam", dialCode: "+84"),
    Country(code: "YE", name: "Yemen", dialCode: "+967"),
    Country(code: "ZM", name: "Zambia", dialCode: "+260"),
    Country(code: "ZW", name: "Zimbabwe", dialCode: "+263")
]

public enum CountriesDataSource {

    public static func allCountries() -> [Country] {
        allCountriesList
    }

    /// Kotlin `CountryRepository.getCountryByCode` — nil for unknown codes.
    public static func country(byCode code: String) -> Country? {
        allCountries().first { $0.code == code }
    }

    /// Kotlin `CountryRepository.searchCountries` — case-insensitive match
    /// on name, dial code, or code.
    public static func searchCountries(query: String) -> [Country] {
        let trimmedQuery = query.trimmingCharacters(in: .whitespaces)
        guard !trimmedQuery.isEmpty else { return allCountries() }
        return allCountries().filter { country in
            country.name.localizedCaseInsensitiveContains(trimmedQuery)
                || country.dialCode.localizedCaseInsensitiveContains(trimmedQuery)
                || country.code.localizedCaseInsensitiveContains(trimmedQuery)
        }
    }
}

/// Phone validation + E.164 formatting — port of Kotlin `PhoneValidator`
/// (iOS variant). Pure: digits-only length check (7–15 per E.164),
/// CNPhoneNumber structural validation, dial-code-prefixed E.164 output.
/// Kept free of UIKit so it unit-tests on the macOS host.
public enum PhoneValidator {

    /// Structural validity for a NATIONAL number (no dial code) in the
    /// given region — Kotlin `isValid`: 7–15 digits + CNPhoneNumber check.
    public static func isValid(nationalNumber: String, regionCode: String) -> Bool {
        let cleanNumber = nationalNumber.trimmingCharacters(in: .whitespaces)
        guard !cleanNumber.isEmpty else { return false }
        let digitCount = cleanNumber.filter(\.isNumber).count
        guard (7...15).contains(digitCount) else { return false }
        // CNPhoneNumber(stringValue:) is non-failable on both iOS and
        // macOS (the Kotlin `?: return false` was a KN interop artifact).
        let cnNumber = CNPhoneNumber(stringValue: cleanNumber)
        guard !cnNumber.stringValue.isEmpty else { return false }
        return true
    }

    /// E.164 formatting — Kotlin `format(E164)`: national number with the
    /// region's dial code prefixed (dial codes already include `+`).
    public static func formatE164(nationalNumber: String, regionCode: String) -> String {
        let cleanNumber = nationalNumber.filter { $0.isNumber || $0 == "+" }
        if cleanNumber.hasPrefix("+") { return cleanNumber }
        guard let country = CountriesDataSource.country(byCode: regionCode) else {
            return "+\(cleanNumber)"
        }
        return "\(country.dialCode)\(cleanNumber)"
    }
}
