// PersistentAssetLockDisplay.swift
// SwiftExampleApp
//
// Consolidates the asset-lock display properties (status label,
// short outpoint, fundability predicate) that previously lived as
// inline switches and duplicated helpers across several views.
// Keeping them on `PersistentAssetLock` itself means the 0/1/2/3
// `AssetLockStatus` discriminants — a protocol-mirrored constant
// from the Rust side — exist in exactly one place in the example
// app. If/when we push these to a Rust-derived FFI property on the
// persistent row, this file is the single point to update.

import Foundation
import SwiftDashSDK

/// Visibility / actionability predicates over the `AssetLockStatus`
/// discriminants, plus the human-facing label. The predicates are
/// defined on `AssetLockResumeRow` so they're free at every
/// conformer (including `FakeAssetLockRow` in tests).
extension AssetLockResumeRow {
    /// `true` when the lock has a usable IS-lock or chain-lock
    /// proof AND has not yet been consumed. Only these locks can
    /// fund a Platform identity; the Resumable Registrations row's
    /// Resume button gates on this. Built (0) and Broadcast (1)
    /// have no signed proof yet — submitting them would fail at the
    /// Platform layer. Consumed (4) was already used to fund an
    /// identity and cannot be reused; the persisted row survives
    /// only for historical lookup.
    ///
    /// RecoveredFromChain (5) also qualifies: the restore scan and the
    /// chainlock-promotion path attach a real `ChainAssetLockProof`
    /// before writing that status, so the lock is exactly as fundable
    /// as a ChainLocked (3) one. What is unknown for a `5` is whether
    /// Platform already consumed it — and Platform, not the client, is
    /// the arbiter of that: it rejects an already-spent outpoint with a
    /// typed error. A user-driven Resume is the surface allowed to ask.
    /// (Do not feed `5` into an automatic retry sweep — blind retries
    /// of historical locks are the failure mode the status exists to
    /// prevent.)
    var canFundIdentity: Bool { statusRaw == 2 || statusRaw == 3 || statusRaw == 5 }

    /// `true` when the lock should be surfaced on the Resumable
    /// Registrations section at all. Lower bar than `canFundIdentity`
    /// — a Broadcast (1) lock isn't actionable yet but the user
    /// should see it (as "Waiting for InstantSendLock…") so an
    /// in-flight crash-recovery situation has visible continuity
    /// through the IS-lock arrival.
    ///
    /// Excluding `4` (Consumed) is load-bearing: it is the terminal
    /// state for a lock that already funded an identity, and the
    /// underlying `resume_asset_lock` rejects Consumed entries with
    /// "already Consumed — nothing to resume". Without that exclusion,
    /// the "Delete identity locally" action — which removes
    /// `PersistentIdentity` but leaves the `PersistentAssetLock` alive
    /// — frees the `(walletId, identityIndex)` slot in the anti-join
    /// and the Consumed row would re-surface as a perpetual-spinner
    /// row that can't be advanced.
    ///
    /// `5` (RecoveredFromChain) IS included. It sits above the terminal
    /// `4` numerically but is not terminal: it is what the restore scan
    /// and the chainlock-promotion path write for a lock with proven
    /// Core finality and unknown Platform-side consumption. Treating
    /// this as a contiguous `1...3` range silently hid every such row,
    /// which is the whole visible symptom — a chain-locked top-up the
    /// user funded appears nowhere and reads as lost funds.
    var isVisibleAsResumable: Bool {
        (statusRaw >= 1 && statusRaw <= 3) || statusRaw == 5
    }
}

/// Human-readable label for `PersistentAssetLock.statusRaw`. Kept
/// here (rather than as a `case` block re-implemented in every
/// view) so the 0...5 → label mapping has one home. Mirrors the
/// Rust-side `AssetLockStatus` enum.
extension PersistentAssetLock {
    var statusLabel: String {
        switch statusRaw {
        case 0: return "Built"
        case 1: return "Broadcast"
        case 2: return "InstantSendLocked"
        case 3: return "ChainLocked"
        case 4: return "Consumed"
        // Core finality proven, Platform-side consumption unknown.
        // Rendered as "Unknown(5)" before this case existed.
        case 5: return "RecoveredFromChain"
        default: return "Unknown(\(statusRaw))"
        }
    }

    /// First 8 hex chars of the txid plus the vout, derived from
    /// the canonical `<txid>:<vout>` outpoint encoding. Used by
    /// every UI that lists asset-lock rows so the txid prefix
    /// reads the same way across surfaces.
    var shortOutPointDisplay: String {
        let parts = outPointHex.split(
            separator: ":",
            maxSplits: 1,
            omittingEmptySubsequences: false
        )
        guard parts.count == 2 else { return outPointHex }
        return "\(parts[0].prefix(8)):\(parts[1])"
    }
}
