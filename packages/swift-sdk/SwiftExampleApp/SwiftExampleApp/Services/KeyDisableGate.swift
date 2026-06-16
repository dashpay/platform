import Foundation
import SwiftDashSDK

/// Client-side safety gate for the **Disable Key** action.
///
/// This is a pre-flight UI guard, not protocol logic. It mixes two
/// kinds of refusal:
///
/// - **Master key (consensus):** disabling a lone master key really is
///   rejected by consensus (`validate_master_key_uniqueness_v0`), so
///   refusing it here just avoids spending fees on a transition we
///   already know `rs-drive-abci` will reject.
/// - **Last auth / last transfer (client-side self-brick guards):**
///   these are *not* protocol invariants. The drive-abci
///   identity_update state validator only checks that every disabled
///   key id exists in state and that master-key uniqueness holds — it
///   would happily accept disabling the last enabled authentication or
///   transfer key. We refuse those locally purely as a UX safeguard so
///   the user can't strand (brick) their own identity.
///
/// The gate is computed against the identity's full enabled key set
/// (these guards are only meaningful relative to what else remains
/// enabled), so callers pass `allKeys` (the identity's
/// `identityPublicKeys`) alongside the `target` key being disabled.
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

        // Client-side self-brick guard (not a consensus rule):
        // drive-abci would accept this transition, but disabling the
        // last enabled AUTHENTICATION key would leave the identity
        // unable to authenticate / sign future transitions, so we
        // refuse it locally to keep the user from stranding the
        // identity.
        if target.purpose == .authentication
            && enabledCount(of: .authentication, in: allKeys) <= 1 {
            return .forbidden(
                reason: "This is the only enabled authentication key. Disabling it would leave the identity unable to sign — add another authentication key first (Add Key)."
            )
        }

        // Client-side self-brick guard (not a consensus rule):
        // drive-abci would accept this too, but disabling the last
        // enabled TRANSFER key would break credit withdrawals /
        // transfers (ID-10), and re-adding a transfer key currently
        // requires the Add Key flow (ID-07). We refuse it locally as a
        // UX safeguard.
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
