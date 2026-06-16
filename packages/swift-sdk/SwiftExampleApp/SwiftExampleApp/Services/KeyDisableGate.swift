import Foundation
import SwiftDashSDK

/// Client-side safety gate for the **Disable Key** action.
///
/// Mirrors the consensus rules Drive enforces on an
/// `IdentityUpdateTransition` that disables a key, so the app never
/// broadcasts a transition that's doomed to be rejected. This is a
/// pre-flight UI guard, not protocol logic — the authoritative checks
/// still run in `rs-drive-abci`; we just refuse to spend fees on a
/// transition we already know fails.
///
/// The gate is computed against the identity's full enabled key set
/// (a disable is only valid relative to what else remains enabled),
/// so callers pass `allKeys` (the identity's `identityPublicKeys`)
/// alongside the `target` key being disabled.
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

    /// Evaluate the disable gate for `target` within `allKeys`.
    ///
    /// `allKeys` should be the identity's current `identityPublicKeys`
    /// (the DPP projection). `target` must be one of them.
    static func evaluate(
        target: IdentityPublicKey,
        allKeys: [IdentityPublicKey]
    ) -> Evaluation {
        // Already disabled → nothing to do.
        if target.disabledAt != nil {
            return .alreadyDisabled
        }

        // A lone master-key disable is rejected by consensus
        // (`validate_master_key_uniqueness_v0`); master-key rotation
        // is out of scope for this action.
        if target.securityLevel == .master {
            return .forbidden(
                reason: "Master keys can't be disabled here — a master-key disable is rejected by consensus. Master-key rotation is out of scope."
            )
        }

        // Disabling the last enabled AUTHENTICATION key would leave
        // the identity unable to authenticate / sign future
        // transitions — consensus requires at least one.
        if target.purpose == .authentication
            && enabledCount(of: .authentication, in: allKeys) <= 1 {
            return .forbidden(
                reason: "This is the only enabled authentication key. Disabling it would leave the identity unable to sign — add another authentication key first (Add Key)."
            )
        }

        // Disabling the last enabled TRANSFER key breaks credit
        // withdrawals / transfers (ID-10). Re-adding a transfer key
        // currently requires the Add Key flow (ID-07).
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
