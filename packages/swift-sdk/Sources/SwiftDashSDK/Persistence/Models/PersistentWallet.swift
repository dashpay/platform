import Foundation
import SwiftData

/// SwiftData model for persisting core wallet metadata.
///
/// Represents a single HD wallet with its sync state.
/// Owns accounts via cascade delete — removing a wallet removes all
/// its accounts, transactions, and UTXOs.
///
/// The wallet-level cached balance fields were removed — the canonical
/// "live" Core balance is summed on demand from
/// `PlatformWalletManager.accountBalances(for:)` (Rust in-memory FFI).
/// Per-account totals continue to live on `PersistentAccount`.
@Model
public final class PersistentWallet {
    /// Index `networkRaw` so per-network wallet scans (used everywhere
    /// from the network-scoped storage explorer to the per-network
    /// "is there a wallet on this chain yet" lookups) don't degrade
    /// to a table scan. Also index `walletGroupId` so the Wallet Info
    /// "Networks" lookup — which fetches every sibling-network row for
    /// a seed by its group id — stays a keyed scan.
    #Index<PersistentWallet>([\.networkRaw], [\.walletGroupId])
    #Unique<PersistentWallet>([\.walletId])

    /// 32-byte NETWORK-SCOPED wallet ID, and the row's primary
    /// uniqueness key. Since the network-scoping change the same seed
    /// yields a DISTINCT `walletId` per network (a domain-tagged network
    /// byte is folded into the digest), so a wallet that exists on
    /// multiple chains has one row per network, each with its own id —
    /// the network is already baked into the id, so `walletId` alone is
    /// globally unique (an earlier `(walletId, networkRaw)` composite
    /// was a leftover from the pre-scoping model, where one seed shared
    /// a single id across networks and `networkRaw` was the only
    /// distinguishing column). To gather a seed's sibling-network rows,
    /// group by `walletGroupId` (which is the same across networks),
    /// not by this id.
    public var walletId: Data
    /// 32-byte NETWORK-INDEPENDENT group id shared by every network's
    /// wallet derived from the same seed (Rust computes it as the
    /// no-network digest of the root key). Distinct from `walletId`,
    /// which is network-scoped. Used to group a seed's sibling-network
    /// rows in the Wallet Info "Networks" section. Defaults to empty
    /// for rows written before this column existed (pre-release, no
    /// migration); consumers treat empty as "legacy — this single row
    /// only".
    public var walletGroupId: Data = Data()
    /// Network this wallet belongs to. `nil` means "not yet known" —
    /// the row was created by a changeset before `persistWalletMetadata`
    /// filled the network in. Views treat `nil` as unknown.
    ///
    /// Stored as the `Network.rawValue` `UInt32?` so SwiftData
    /// `#Predicate` expressions can evaluate it directly. See
    /// `PersistentIdentity.networkRaw` for the full rationale.
    public var networkRaw: UInt32?

    /// Type-safe accessor over `networkRaw`. `nil` round-trips as
    /// `nil`; non-nil reads fall back to `.testnet` if the stored
    /// raw value ever drifts out of the `Network` range.
    public var network: Network? {
        get {
            guard let raw = networkRaw else { return nil }
            return Network(rawValue: raw) ?? .testnet
        }
        set { networkRaw = newValue?.rawValue }
    }
    /// Optional wallet name.
    public var name: String?
    /// Optional free-form user-supplied description. Mirrored into
    /// the keychain metadata blob (see `WalletKeychainMetadata`) so
    /// it survives a SwiftData wipe / reinstall via the
    /// orphan-mnemonic recovery flow. No UI surfaces this yet, but
    /// the column is wired so existing rows roll forward without a
    /// schema migration when it lands.
    public var walletDescription: String?
    /// Birth height — block height when the wallet was created.
    public var birthHeight: UInt32
    /// Last synced core block height.
    public var syncedHeight: UInt32
    /// Timestamp of last sync (Unix seconds).
    public var lastSynced: UInt64
    /// Bincode-serialised
    /// `dashcore::ephemerealdata::chain_lock::ChainLock` carrying the
    /// wallet's `WalletMetadata::last_applied_chain_lock` from the
    /// previous session. Roundtripped across app launches so the
    /// asset-lock-resume CL-from-metadata fallback in Rust's
    /// `proof.rs` can fire on catch-up at launch without waiting
    /// for SPV to re-apply a fresh ChainLock. `nil` when no
    /// ChainLock has ever been observed for this wallet (fresh
    /// wallet, or pre-feature row).
    public var lastAppliedChainLockBytes: Data?
    /// NUMERIC block height of the wallet's last applied ChainLock —
    /// the same watermark whose bincode blob sits in
    /// `lastAppliedChainLockBytes`, which is opaque on this side of the
    /// FFI. Delivered separately through the persistence extension's
    /// `on_persist_wallet_changeset_chain_lock_height_fn` and stored
    /// with monotonic-max semantics (chain locks only move forward).
    /// This is one half of the swept-tombstone collection boundary
    /// `min(chainlockHeight, syncedHeight)` — see
    /// `PersistentPendingInput.winnerMinedHeight`. `nil` (fresh wallet,
    /// pre-feature row, or a native library too old to fill the slot)
    /// means no finality boundary is known and no tombstone may be
    /// collected. Optional, so existing stores lightweight-migrate.
    public var lastAppliedChainLockHeight: UInt32?
    /// User imported this wallet from an existing mnemonic (as
    /// opposed to generating a fresh one). Cosmetic flag that
    /// drives the "📥 Imported" badge; defaulted to `false` for
    /// rows that predate the column.
    public var isImported: Bool = false
    /// Verified seed-binding marker: the BIP44 account-0 xpub that the
    /// Keychain-resolved seed was proven to derive, bound to the mnemonic
    /// Keychain item's identity stamp, written after one successful
    /// `platform_wallet_verify_seed_binds_to_wallet_cached` run. On later
    /// launches the unlock path hands this back to Rust (with the item's
    /// current stamp), which skips the mnemonic-resolving derivation when
    /// it still matches — and re-verifies when the xpub OR the Keychain
    /// item changed. Opaque to Swift — Rust decides match-vs-verify; this
    /// column only stores and returns it. `nil` (rows predating the
    /// column, or never verified) means the full check runs at the next
    /// unlock.
    public var seedBindingVerifiedMarker: String?
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Accounts belonging to this wallet.
    @Relationship(deleteRule: .cascade, inverse: \PersistentAccount.wallet)
    public var accounts: [PersistentAccount]

    /// Identities registered against this wallet. Cardinality is
    /// 0..N — a wallet may have zero identities (freshly created)
    /// or many. Deletion semantics: `.nullify` so an identity
    /// survives a wallet delete as an orphaned row (useful for
    /// post-mortem inspection and possible re-association if the
    /// wallet is re-imported from the same seed).
    ///
    /// Paired with `PersistentIdentity.wallet` (plain stored
    /// property; the inverse key lives on this side).
    @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.wallet)
    public var identities: [PersistentIdentity]

    public init(
        walletId: Data,
        walletGroupId: Data = Data(),
        network: Network? = nil,
        name: String? = nil,
        walletDescription: String? = nil,
        birthHeight: UInt32 = 0,
        syncedHeight: UInt32 = 0,
        isImported: Bool = false
    ) {
        self.walletId = walletId
        self.walletGroupId = walletGroupId
        self.networkRaw = network?.rawValue
        self.name = name
        self.walletDescription = walletDescription
        self.birthHeight = birthHeight
        self.syncedHeight = syncedHeight
        self.lastSynced = 0
        self.isImported = isImported
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.accounts = []
        self.identities = []
    }
}

// MARK: - Display Helpers

extension PersistentWallet {
    /// User-facing short label. Prefers the persisted `name`;
    /// falls back to `"Wallet <first-4-bytes-hex>…"` so the row
    /// is still clickable when the user hasn't named it. Mirrors
    /// the fallback logic the removed `HDWallet.label` gave
    /// callers without them having to branch at every call site.
    public var label: String {
        if let name = name, !name.isEmpty {
            return name
        }
        let hex = walletId.prefix(4)
            .map { String(format: "%02x", $0) }
            .joined()
        return hex.isEmpty ? "Wallet" : "Wallet \(hex)…"
    }

}

// MARK: - Queries

extension PersistentWallet {
    public static func predicate(walletId: Data) -> Predicate<PersistentWallet> {
        #Predicate<PersistentWallet> { $0.walletId == walletId }
    }

    /// Fetch every sibling-network row for one seed by its
    /// network-independent group id. See `walletGroupId`.
    public static func predicate(
        walletGroupId: Data
    ) -> Predicate<PersistentWallet> {
        #Predicate<PersistentWallet> { $0.walletGroupId == walletGroupId }
    }
}
