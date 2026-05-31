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
    /// to a table scan.
    #Index<PersistentWallet>([\.networkRaw])
    #Unique<PersistentWallet>([\.walletId, \.networkRaw])

    /// 32-byte wallet ID (SHA256 of root public key). Not unique on
    /// its own — the same seed yields the same `walletId` on every
    /// network, so a wallet that exists on multiple chains has one
    /// row per network. Uniqueness is the composite
    /// `(walletId, networkRaw)` declared above.
    public var walletId: Data
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
    /// User imported this wallet from an existing mnemonic (as
    /// opposed to generating a fresh one). Cosmetic flag that
    /// drives the "📥 Imported" badge; defaulted to `false` for
    /// rows that predate the column.
    public var isImported: Bool = false
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
        network: Network? = nil,
        name: String? = nil,
        walletDescription: String? = nil,
        birthHeight: UInt32 = 0,
        syncedHeight: UInt32 = 0,
        isImported: Bool = false
    ) {
        self.walletId = walletId
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

    public static func predicate(
        walletId: Data,
        network: Network
    ) -> Predicate<PersistentWallet> {
        let networkRaw = network.rawValue
        return #Predicate<PersistentWallet> {
            $0.walletId == walletId && $0.networkRaw == networkRaw
        }
    }
}
