import SwiftUI

/// OTP verification screen — SwiftUI port of Kotlin
/// `OtpVerificationScreen.kt`: 6-digit input with auto-verify on paste,
/// the 15-second resend countdown ("Resend OTP in 0:15" → tappable
/// "Resend OTP"), and the invalid-OTP error copy.
///
/// No view model — the OTP code + error state live HERE; the sign-in
/// screen owns the phone number and resend timer (the resend is its
/// request, so its state).
public struct OtpVerificationView: View {

    @Environment(\.dismiss) private var dismiss
    @State private var otpCode = ""
    @State private var otpErrorMessage: String?
    @State private var isVerifying = false
    /// Length of the code at the last change — Kotlin `isPasteOperation`:
    /// a jump of >1 means paste (auto-verify only on paste, not on typing
    /// the last digit).
    @State private var previousCodeLength = 0

    private let authClient: AuthClient
    private let sentToPhoneNumber: String
    private let onResend: () async -> Void
    /// Owned by the sign-in screen (the resend is its request).
    private let resendTimerSeconds: Int?

    public init(
        authClient: AuthClient,
        sentToPhoneNumber: String,
        onResend: @escaping () async -> Void,
        resendTimerSeconds: Int?
    ) {
        self.authClient = authClient
        self.sentToPhoneNumber = sentToPhoneNumber
        self.onResend = onResend
        self.resendTimerSeconds = resendTimerSeconds
    }

    public var body: some View {
        VStack(spacing: 20) {
            Spacer(minLength: 64)

            Text("Enter the code sent to \(sentToPhoneNumber)")
                .font(.subheadline)
                .foregroundStyle(.white.opacity(0.8))
                .multilineTextAlignment(.center)

            OtpInputField(
                code: otpCode,
                onCodeChange: { otpCodeChanged($0) },
                onVerify: { Task { await verifyOTP() } }
            )

            resendControl

            if let otpErrorMessage {
                Text(otpErrorMessage)
                    .font(.footnote)
                    .foregroundStyle(Color(red: 1.0, green: 0.45, blue: 0.6))
                    .multilineTextAlignment(.center)
            }

            Button {
                Task { await verifyOTP() }
            } label: {
                Text(isVerifying ? "Verifying…" : "Verify")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            }
            .buttonStyle(.borderedProminent)
            .tint(Color(red: 0.95, green: 0.25, blue: 0.55))
            .disabled(isVerifying)
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Color(red: 0.04, green: 0.04, blue: 0.06))
        #if canImport(UIKit)
        .navigationBarBackButtonHidden(true)
        .toolbar {
            ToolbarItem(placement: .navigationBarLeading) {
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "chevron.left")
                        .foregroundStyle(.white)
                }
            }
        }
        #endif
    }

    // MARK: - Actions (Kotlin onOtpCodeChanged + onVerifyOtpClicked)

    private func otpCodeChanged(_ code: String) {
        otpCode = code
        otpErrorMessage = nil
    }

    private func verifyOTP() async {
        guard !otpCode.allSatisfy(\.isWhitespace), !isVerifying else { return }
        isVerifying = true
        defer { isVerifying = false }
        do {
            try await authClient.verifyPhoneAuth(
                phoneNumber: sentToPhoneNumber, code: otpCode
            )
        } catch {
            // Kotlin: cancel the timer + `InvalidOtp` error copy.
            otpErrorMessage = "Invalid OTP. Please try again."
        }
    }

    /// Kotlin `ResendOtpText`: countdown while the timer runs, then the
    /// tappable resend.
    @ViewBuilder
    private var resendControl: some View {
        if let secondsLeft = resendTimerSeconds {
            Text("Resend OTP in \(Self.remainingTimeText(secondsLeft))")
                .font(.subheadline)
                .foregroundStyle(.white.opacity(0.5))
        } else {
            Button {
                Task { await onResend() }
            } label: {
                Text("Resend OTP")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.white)
            }
        }
    }

    private static func remainingTimeText(_ seconds: Int) -> String {
        String(format: "%d:%02d", seconds / 60, seconds % 60)
    }
}

/// 6-digit OTP input — port of Kotlin `OtpInput` (config length 6): one
/// hidden text field drives six boxed digits; auto-verify fires ONLY on a
/// full-length paste (Kotlin: filtered length − previous length > 1).
private struct OtpInputField: View {

    let code: String
    let onCodeChange: (String) -> Void
    let onVerify: () -> Void

    @State private var previousLength = 0

    var body: some View {
        TextField("", text: Binding(
            get: { code },
            set: { newValue in
                let filtered = String(newValue.filter(\.isNumber).prefix(6))
                let isPasteOperation = filtered.count - previousLength > 1
                previousLength = filtered.count
                onCodeChange(filtered)
                if filtered.count == 6 && isPasteOperation {
                    onVerify()
                }
            }
        ))
        #if canImport(UIKit)
        .keyboardType(.numberPad)
        .textContentType(.oneTimeCode)
        #endif
        .frame(width: 1, height: 1)
        .opacity(0.001)
        .accessibilityHidden(true)
        .background(
            HStack(spacing: 8) {
                ForEach(0..<6, id: \.self) { index in
                    Text(digit(at: index))
                        .font(.title2.weight(.semibold))
                        .foregroundStyle(.white)
                        .frame(width: 44, height: 52)
                        .background(Color(white: 0.13), in: RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(
                                    isActive(index)
                                        ? Color(red: 1.0, green: 0.45, blue: 0.6)
                                        : Color(white: 0.28),
                                    lineWidth: isActive(index) ? 2 : 1
                                )
                        )
                }
            }
        )
    }

    private func digit(at index: Int) -> String {
        index < code.count
            ? String(code[code.index(code.startIndex, offsetBy: index)])
            : ""
    }

    private func isActive(_ index: Int) -> Bool {
        index == code.count
    }
}
