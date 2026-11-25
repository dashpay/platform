import Foundation
import LocalAuthentication

/// The system Face ID / Touch ID / passcode prompt, shared by every screen
/// that reveals a secret (address and provider private keys, the recovery
/// phrase) and by the orphan-wallet recovery flow.
///
/// This is a gate in front of a reveal, not a protection of the key
/// itself: the mnemonic Keychain item has no access control on its read
/// path (see `WalletStorage`), so any code in the process can derive a
/// key while the device is unlocked. On a simulator the prompt needs
/// Face ID enrolled (Features > Face ID > Enrolled), otherwise it
/// reports unavailable.
enum DeviceOwnerAuthentication {
    /// The result of one prompt. The associated strings are the system's
    /// error descriptions; each caller words its own message around them.
    enum Outcome {
        /// The device owner authenticated: go ahead with the reveal.
        case authorized
        /// The user, the app or the system dismissed the prompt.
        case cancelled
        /// `canEvaluatePolicy` refused (no passcode set, biometry locked
        /// out, ...). Carries the policy error's description, or
        /// "unknown" when there is none.
        case unavailable(String)
        /// The prompt ran and failed. Carries the error's description.
        case failed(String)
    }

    /// Ask the device owner to authenticate with
    /// `.deviceOwnerAuthentication`, so a passcode works when biometrics
    /// are not set up.
    @MainActor
    static func authorize(reason: String) async -> Outcome {
        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            return .unavailable(policyError?.localizedDescription ?? "unknown")
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
                return .failed(error.localizedDescription)
            }
        } catch {
            return .failed(error.localizedDescription)
        }
    }
}
