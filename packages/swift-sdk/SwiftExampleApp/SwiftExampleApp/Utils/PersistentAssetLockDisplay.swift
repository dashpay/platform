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
    /// proof. Only these locks can fund a Platform identity right
    /// now; the Resumable Registrations row's Resume button gates
    /// on this. Built (0) and Broadcast (1) have no signed proof
    /// yet — submitting them would fail at the Platform layer.
    var canFundIdentity: Bool { statusRaw >= 2 }

    /// `true` when the lock should be surfaced on the Resumable
    /// Registrations section at all. Lower bar than `canFundIdentity`
    /// — a Broadcast (1) lock isn't actionable yet but the user
    /// should see it (as "Waiting for InstantSendLock…") so an
    /// in-flight crash-recovery situation has visible continuity
    /// through the IS-lock arrival.
    var isVisibleAsResumable: Bool { statusRaw >= 1 }
}

/// Human-readable label for `PersistentAssetLock.statusRaw`. Kept
/// here (rather than as a `case` block re-implemented in every
/// view) so the 0/1/2/3 → label mapping has one home. Mirrors the
/// Rust-side `AssetLockStatus` enum.
extension PersistentAssetLock {
    var statusLabel: String {
        switch statusRaw {
        case 0: return "Built"
        case 1: return "Broadcast"
        case 2: return "InstantSendLocked"
        case 3: return "ChainLocked"
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
