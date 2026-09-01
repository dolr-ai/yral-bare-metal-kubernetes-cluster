import SwiftUI
#if canImport(UIKit)
import AuthenticationServices
#endif

/// Sign-up / sign-in screen — SwiftUI port of Kotlin `SignupView` in its
/// default `LoginMode.BOTH` shape: headline, phone section (country
/// picker + number + continue button), terms-of-service consent, "or"
/// divider, and the Google/Apple icon tiles.
///
/// No view model — screen state lives HERE as @State; the auth actions
/// call `AuthClient` directly (inline by default; the resend countdown
/// is a 15-second Task beside the state it drives).
public struct SignInView: View {

    // MARK: - Screen state (no view model — colocated @State)

    @State private var selectedCountry: Country?
    @State private var phoneNumber = ""
    @State private var phoneValidationError: String?
    @State private var isRequestingOTP = false
    @State private var sentToPhoneNumber: String?
    @State private var resendTimerSeconds: Int?
    @State private var resendTimerTask: Task<Void, Never>?
    @State private var socialAuthError: String?
    @State private var isCountrySelectorShown = false

    @Environment(\.openURL) private var openURL

    /// Kotlin `OTP_RESEND_TIMER_SECONDS`.
    private let otpResendTimerSeconds = 15

    private let authClient: AuthClient

    public init(authClient: AuthClient) {
        self.authClient = authClient
    }

    public var body: some View {
        NavigationStack {
            VStack(spacing: 28) {
                VStack(spacing: 8) {
                    Text("Continue to sign up for free")
                        .font(.title2.weight(.semibold))
                        .multilineTextAlignment(.center)
                    Text("Create your account to start watching and earning")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity)

                VStack(spacing: 20) {
                    phoneSection
                    termsOfServiceText
                    orDivider
                    socialSection
                }

                if let socialAuthError {
                    Text(socialAuthError)
                        .font(.footnote)
                        .foregroundStyle(.pink)
                        .multilineTextAlignment(.center)
                        .padding(.top, 8)
                }
            }
            .padding(.horizontal, 16)
            .padding(.top, 46)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .background(Color.black)
            #if canImport(UIKit)
                .toolbar(.hidden, for: .navigationBar)
            #endif
            .navigationDestination(
                isPresented: Binding(
                    get: { sentToPhoneNumber != nil },
                    set: {
                        if !$0 {
                            sentToPhoneNumber = nil
                            resetOTPScreenState()
                        }
                    }
                )
            ) {
                if let phoneNumber = sentToPhoneNumber {
                    OtpVerificationView(
                        authClient: authClient,
                        sentToPhoneNumber: phoneNumber,
                        onResend: { Task { await requestOTP(for: phoneNumber) } },
                        resendTimerSeconds: resendTimerSeconds
                    )
                }
            }
            .navigationDestination(isPresented: $isCountrySelectorShown) {
                CountrySelectorView(
                    onSelect: { country in
                        selectedCountry = country
                        isCountrySelectorShown = false
                    },
                    onBack: { isCountrySelectorShown = false }
                )
            }
        }
        .onAppear(perform: detectDefaultCountry)
    }

    // MARK: - Country detection (Kotlin LoginViewModel init)

    /// Kotlin init: English-language devices default to India (temporary
    /// until server-side geo); others use the device region; US fallback.
    private func detectDefaultCountry() {
        guard selectedCountry == nil else { return }
        let deviceLanguage = (Locale.current.language.languageCode?.identifier ?? "")
            .lowercased()
        let regionCode =
            deviceLanguage == "en"
            ? "IN"
            : Locale.current.region?.identifier ?? Locale.current.regionCode
        selectedCountry =
            regionCode.flatMap { CountriesDataSource.country(byCode: $0) }
            ?? CountriesDataSource.country(byCode: "US")
    }

    // MARK: - Phone section (Kotlin PhoneSignupSection)

    private var phoneSection: some View {
        VStack(spacing: 12) {
            HStack(spacing: 8) {
                CountryPickerButton(country: selectedCountry) {
                    isCountrySelectorShown = true
                }

                PhoneInputRow(
                    nationalNumber: $phoneNumber,
                    selectedCountry: selectedCountry,
                    isError: phoneValidationError != nil
                )
            }

            if let validationError = phoneValidationError {
                Text(validationError)
                    .font(.footnote)
                    .foregroundStyle(.pink)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }

            Button {
                Task { await requestOTP() }
            } label: {
                Text(isRequestingOTP ? "Sending code…" : "Continue")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(.pink)
            .disabled(isRequestingOTP)
        }
    }

    // MARK: - OTP request (Kotlin onPhoneLoginClicked)

    /// Kotlin guard chain: country missing → "select country"; blank →
    /// "enter phone number"; invalid → "invalid phone number format".
    /// Success → navigate to the OTP screen + start the resend countdown.
    private func requestOTP() async {
        guard let country = selectedCountry else {
            phoneValidationError = "Please select country"
            return
        }
        if phoneNumber.allSatisfy(\.isWhitespace) {
            phoneValidationError = "Please enter phone number"
            return
        }
        guard
            PhoneValidator.isValid(
                nationalNumber: phoneNumber, regionCode: country.code
            )
        else {
            phoneValidationError = "Invalid phone number format"
            return
        }

        let formattedNumber = PhoneValidator.formatE164(
            nationalNumber: phoneNumber, regionCode: country.code
        )
        isRequestingOTP = true
        phoneValidationError = nil
        defer { isRequestingOTP = false }

        do {
            _ = try await authClient.phoneAuthLogin(phoneNumber: formattedNumber)
            sentToPhoneNumber = formattedNumber
            startResendTimer()
        } catch {
            phoneValidationError = "Failed to send verification code"
        }
    }

    /// Resend for an already-verified-shape number (from the OTP screen).
    private func requestOTP(for formattedNumber: String) async {
        guard resendTimerSeconds == nil else { return }
        do {
            _ = try await authClient.phoneAuthLogin(phoneNumber: formattedNumber)
            startResendTimer()
        } catch {
            // Kotlin restarts the timer on a failed resend too (no
            // rapid-fire retries).
            startResendTimer()
        }
    }

    /// Kotlin `startResendTimer`: 15 → 0 countdown; nil = resend enabled.
    private func startResendTimer() {
        resendTimerTask?.cancel()
        resendTimerSeconds = otpResendTimerSeconds
        resendTimerTask = Task {
            for seconds in stride(from: otpResendTimerSeconds, through: 0, by: -1) {
                guard !Task.isCancelled else { return }
                resendTimerSeconds = seconds
                if seconds > 0 {
                    try? await Task.sleep(for: .seconds(1))
                }
            }
            resendTimerSeconds = nil
        }
    }

    private func resetOTPScreenState() {
        resendTimerTask?.cancel()
        resendTimerTask = nil
        resendTimerSeconds = nil
    }

    // MARK: - Terms consent (Kotlin TermsOfServiceText)

    private var termsOfServiceText: some View {
        (Text("By continuing, you agree to our ")
            + Text("Terms of Service").underline().foregroundStyle(.pink))
            .font(.footnote)
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity)
            .contentShape(Rectangle())
            .onTapGesture {
                // Terms link — Kotlin `LoginViewModel.getTncLink()`
                // (flag-managed; the production value).
                if let termsURL = URL(string: "https://www.yral.com/terms") {
                    openURL(termsURL)
                }
            }
    }

    // MARK: - "or" divider (Kotlin OrDivider)

    private var orDivider: some View {
        HStack(spacing: 12) {
            Rectangle().fill(.separator).frame(height: 1)
            Text("or").font(.subheadline).foregroundStyle(.tertiary)
            Rectangle().fill(.separator).frame(height: 1)
        }
        .padding(.vertical, 8)
    }

    // MARK: - Social section (Kotlin SocialSignupSection — icon tiles)
    //
    // TODO(auth-native-google) and TODO(auth-native-apple) — the two
    // NATIVE implementations that replace this browser flow — live in
    // BrowserAuthSession.swift beside this screen.

    private var socialSection: some View {
        HStack(spacing: 12) {
            socialTile(.google, icon: Text("G").font(.title2.weight(.bold)))
            socialTile(.apple, icon: Image(systemName: "apple.logo").font(.title2))
        }
    }

    private func socialTile(
        _ provider: SocialProvider, icon: some View
    ) -> some View {
        Button {
            Task { await startSocialSignIn(provider: provider) }
        } label: {
            ZStack {
                RoundedRectangle(cornerRadius: 12)
                    .fill(Color.gray.opacity(0.2))
                    .frame(height: 52)
                icon.foregroundStyle(.primary)
            }
        }
        .buttonStyle(.plain)
    }

    /// Social sign-in — the browser flow lives in `BrowserAuthSession`
    /// (colocated beside this screen). The underlying error reason is
    /// surfaced (a generic copy hides the actual cause while testing).
    private func startSocialSignIn(provider: SocialProvider) async {
        socialAuthError = nil
        do {
            let result = try await BrowserAuthSession.signIn(
                provider: provider, authClient: authClient
            )
            try await authClient.handleOAuthCallbackResult(result)
        } catch {
            let reason = (error as? LocalizedError)?.errorDescription
                ?? String(describing: error)
            socialAuthError = "Sign-in failed: \(reason)"
        }
    }
}

/// Country picker button — Kotlin `CountryPickerButton`: flag + dial code
/// + chevron; navigates to `CountrySelectorView`.
private struct CountryPickerButton: View {
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

/// Dial-code prefix + digit-only number field — Kotlin `PhoneInputField`:
/// digits filtered and capped at the country's max length.
private struct PhoneInputRow: View {
    @Binding var nationalNumber: String
    let selectedCountry: Country?
    let isError: Bool

    var body: some View {
        HStack(spacing: 8) {
            Text(selectedCountry?.dialCode ?? "+1")
                .font(.subheadline.weight(.semibold))
            TextField("Enter mobile number", text: Binding(
                get: { nationalNumber },
                set: { newValue in
                    let maximumLength = selectedCountry?.maxLength ?? 15
                    nationalNumber = String(
                        newValue.filter(\.isNumber).prefix(maximumLength)
                    )
                }
            ))
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

#Preview {
    SignInView(
        authClient: AuthClient(
            authDataSource: AuthDataSource(),
            redirectScheme: "com.yral.iosApp",
            sessionStore: SessionStore()
        )
    )
}
