import Foundation
import SwiftData

/// SwiftData row for a TRACKED (wallet-independent) masternode — a node the
/// user follows that belongs to no wallet.
///
/// Deliberately NOT a `PersistentMasternode`: that model is keyed and
/// network-scoped by its owning `walletId`, which a tracked node doesn't
/// have. This row is keyed by `(networkRaw, proTxHash)` directly, survives
/// deleting any single wallet, and is removed only by untracking (or a
/// reset-all).
///
/// The row is pure storage for the Rust tracked-masternode registry
/// (`platform_wallet::masternode::tracked`): `snapshotJSON` is the
/// versioned, Rust-produced cache of everything learned about the node
/// (its list entry, Platform identity key hashes, registration details) —
/// PUBLIC material only, decoded exclusively by Rust. Keys the user
/// attaches to a tracked node live in the host's secure storage
/// (Keychain), never here.
@Model
public final class PersistentTrackedMasternode {
    #Unique<PersistentTrackedMasternode>([\.networkRaw, \.proTxHash])
    #Index<PersistentTrackedMasternode>([\.networkRaw])

    /// `Network.rawValue` of the network the node lives on.
    public var networkRaw: UInt32
    /// proTxHash (32 raw wire bytes) — same orientation as
    /// `PersistentMasternode.proTxHash`.
    public var proTxHash: Data
    /// Optional user label.
    public var label: String?
    /// Unix seconds when the user tracked it.
    public var addedAt: UInt64
    /// Versioned snapshot document produced by Rust
    /// (`snapshot_to_json`); stored opaquely and handed back verbatim on
    /// restore.
    public var snapshotJSON: String

    public var network: Network? {
        get { Network(rawValue: networkRaw) }
        set { networkRaw = newValue?.rawValue ?? networkRaw }
    }

    public init(
        networkRaw: UInt32,
        proTxHash: Data,
        label: String?,
        addedAt: UInt64,
        snapshotJSON: String
    ) {
        self.networkRaw = networkRaw
        self.proTxHash = proTxHash
        self.label = label
        self.addedAt = addedAt
        self.snapshotJSON = snapshotJSON
    }
}
