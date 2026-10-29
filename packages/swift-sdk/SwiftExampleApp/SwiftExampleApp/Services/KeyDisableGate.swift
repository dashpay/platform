import Foundation
import SwiftDashSDK

/// Client-side safety gate for the **Disable Key** action.
///
/// This is a pre-flight UI guard. It decides nothing about DIP-11 itself:
///
/// - **Last valid key of a security level (DIP-11):** Rust judges it
///   (`ManagedPlatformWallet.checkKeysCanBeDisabled`, dpp's
///   `check_keys_can_be_disabled` at the device clock) and the caller
///   hands the verdict in as a `RuleVerdict`. The identity update builder
///   refuses the same updates before signing, so the gate only spares the
///   user a doomed attempt.
/// - **Last transfer key (client-side self-brick guard):** not a protocol
///   or DIP rule, and not in Rust — drive-abci would accept it — but
///   disabling the last enabled transfer key would break credit
///   withdrawals, so we refuse it locally.
///
/// Callers pass `allKeys` (the identity's `identityPublicKeys`) alongside
/// the `target` key being disabled, for the transfer guard.
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

    /// Rust's DIP-11 verdict for disabling the target key alone.
    enum RuleVerdict: Equatable {
        /// DIP-11 allows disabling the key.
        case allowed
        /// DIP-11 forbids it: the refusal names the level left without a
        /// valid key.
        case refused(ManagedPlatformWallet.KeyDisableRefusal)
        /// No verdict: the wallet is not loaded, or the check failed or is
        /// still running. `reason` is user-facing copy.
        case unavailable(reason: String)
    }

    /// Ask Rust for the DIP-11 verdict on disabling `keyId` of
    /// `identityId` at the device clock. Without a loaded wallet, or when
    /// the check fails, there is no verdict and the gate forbids the action.
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

    /// Evaluate the disable gate for `target` within `allKeys`, given
    /// Rust's `ruleVerdict` for disabling `target` alone.
    ///
    /// `allKeys` should be the identity's current `identityPublicKeys`
    /// (the DPP projection). `target` must be one of them.
    static func evaluate(
        target: IdentityPublicKey,
        allKeys: [IdentityPublicKey],
        ruleVerdict: RuleVerdict
    ) -> Evaluation {
        // Already disabled → nothing to do.
        if target.disabledAt != nil {
            return .alreadyDisabled
        }

        switch ruleVerdict {
        case .refused(let refusal):
            return .forbidden(
                reason: "No other valid authentication key at the \(refusal.securityLevel.name) level would remain. Disabling this key would leave the identity unable to sign what needs that level — add another key first (Add Key)."
            )
        case .unavailable(let reason):
            return .forbidden(reason: reason)
        case .allowed:
            break
        }

        // Client-side self-brick guard (not a consensus or DIP rule):
        // drive-abci would accept this, but disabling the last enabled
        // TRANSFER key would break credit withdrawals / transfers
        // (ID-10), and re-adding a transfer key currently requires the
        // Add Key flow (ID-07). We refuse it locally as a UX safeguard.
        if target.purpose == .transfer
            && enabledCount(of: .transfer, in: allKeys) <= 1 {
            return .forbidden(
                reason: "This is the only enabled transfer key. Disabling it would break credit withdrawals — add another transfer key first (Add Key)."
            )
        }

        return .allowed
    }

    /// Count of enabled (non-disabled) keys with the given purpose.
    private static func enabledCount(
        of purpose: KeyPurpose,
        in keys: [IdentityPublicKey]
    ) -> Int {
        keys.filter { $0.purpose == purpose && $0.disabledAt == nil }.count
    }
}
