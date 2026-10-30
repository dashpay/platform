import Foundation
import SwiftDashSDK

/// Client-side safety gate for the **Disable Key** action.
///
/// A pre-flight UI guard that decides nothing itself. Which keys may be
/// disabled is the wallet's call (`ManagedPlatformWallet.checkKeysCanBeDisabled`:
/// the last valid authentication key of a security level under DIP-11,
/// and the last enabled transfer key), judged in Rust and handed in as a
/// `RuleVerdict`. The wallet's identity-update entry
/// points refuse the same updates, so the gate only spares the user a
/// doomed attempt and turns the structured refusal into copy.
enum KeyDisableGate {
    /// Result of evaluating whether `target` may be disabled.
    enum Evaluation: Equatable {
        /// The key is already disabled on-chain — no action to take.
        case alreadyDisabled
        /// The key may be disabled.
        case allowed
        /// The key may not be disabled; `reason` is user-facing copy.
        case forbidden(reason: String)
    }

    /// Rust's verdict for disabling the target key alone.
    enum RuleVerdict: Equatable {
        /// The wallet allows disabling the key.
        case allowed
        /// The wallet refuses it, for the reason the refusal names.
        case refused(ManagedPlatformWallet.KeyDisableRefusal)
        /// No verdict: the wallet is not loaded, or the check failed or is
        /// still running. `reason` is user-facing copy.
        case unavailable(reason: String)
    }

    /// Ask Rust whether the wallet lets `keyId` of `identityId` be
    /// disabled. Without a loaded wallet, or when the check fails, there is
    /// no verdict and the gate forbids the action.
    static func ruleVerdict(
        wallet: ManagedPlatformWallet?,
        identityId: Identifier,
        keyId: UInt32
    ) async -> RuleVerdict {
        guard let wallet else {
            return .unavailable(
                reason: "Identity not attached to a loaded wallet — cannot check or sign the disable transition."
            )
        }
        do {
            if let refusal = try await wallet.checkKeysCanBeDisabled(
                identityId: identityId,
                keyIds: [keyId]
            ) {
                return .refused(refusal)
            }
            return .allowed
        } catch {
            return .unavailable(reason: error.localizedDescription)
        }
    }

    /// Evaluate the disable gate for `target`, given Rust's `ruleVerdict`
    /// for disabling it alone.
    static func evaluate(
        target: IdentityPublicKey,
        ruleVerdict: RuleVerdict
    ) -> Evaluation {
        // Already disabled → nothing to do.
        if target.disabledAt != nil {
            return .alreadyDisabled
        }

        switch ruleVerdict {
        case .allowed:
            return .allowed
        case .unavailable(let reason):
            return .forbidden(reason: reason)
        case .refused(let refusal):
            return .forbidden(reason: reason(for: refusal))
        }
    }

    /// User-facing copy for a refusal.
    private static func reason(for refusal: ManagedPlatformWallet.KeyDisableRefusal) -> String {
        switch refusal {
        case .lastAuthenticationKey(_, let securityLevel):
            return "No other valid authentication key at the \(securityLevel.name) level would remain. Disabling this key would leave the identity unable to sign what needs that level — add another key first (Add Key)."
        case .lastEnabledKey(_, let purpose):
            let name = purpose.name.lowercased()
            return "No other enabled \(name) key able to sign would remain. Disabling this key would break credit withdrawals and transfers — add another \(name) key first (Add Key)."
        }
    }
}
