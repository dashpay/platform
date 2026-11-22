import Foundation
import LocalAuthentication

/// The system Face ID / Touch ID / passcode prompt, put in front of a
/// screen that reveals a private key.
///
/// This is a gate in front of a reveal, not a protection of the key
/// itself: the mnemonic Keychain item has no access control on its read
/// path (see `WalletStorage`), so any code in the process can derive a
/// key while the device is unlocked. On a simulator the prompt needs
/// Face ID enrolled (Features > Face ID > Enrolled), otherwise it
/// reports unavailable.
enum DeviceOwnerAuthentication {
    enum Outcome {
        /// The device owner authenticated: go ahead with the reveal.
        case authorized
        /// The user, the app or the system dismissed the prompt. Show
        /// nothing.
        case cancelled
        /// Authentication is unavailable or failed. The message is ready
        /// to show.
        case failed(String)
    }

    /// Ask the device owner to authenticate with the same policy as the
    /// recovery phrase reveal (`.deviceOwnerAuthentication`, so a passcode
    /// works when biometrics are not set up).
    @MainActor
    static func authorize(reason: String) async -> Outcome {
        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            return .failed(
                "Authentication is unavailable on this device: "
                    + (policyError?.localizedDescription ?? "unknown")
            )
        }
        do {
            let authorized = try await context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: reason
            )
            return authorized ? .authorized : .cancelled
        } catch let error as LAError {
            switch error.code {
            case .userCancel, .appCancel, .systemCancel:
                return .cancelled
            default:
                return .failed("Authorization failed: \(error.localizedDescription)")
            }
        } catch {
            return .failed("Authorization failed: \(error.localizedDescription)")
        }
    }
}
