import Foundation
import SwiftData

/// SwiftData model for persisting core wallet metadata.
///
/// Represents a single HD wallet with its sync state and balance.
/// Owns accounts via cascade delete — removing a wallet removes all
/// its accounts, transactions, and UTXOs.
@Model
public final class PersistentWallet {
    /// 32-byte wallet ID (SHA256 of root public key).
    @Attribute(.unique) public var walletId: Data
    /// Network name (mainnet, testnet, devnet).
    public var network: String
    /// Optional wallet name.
    public var name: String?
    /// Birth height — block height when the wallet was created.
    public var birthHeight: UInt32
    /// Last synced core block height.
    public var syncedHeight: UInt32
    /// Timestamp of last sync (Unix seconds).
    public var lastSynced: UInt64
    /// Confirmed balance in duffs.
    public var balanceConfirmed: UInt64
    /// Unconfirmed balance in duffs.
    public var balanceUnconfirmed: UInt64
    /// Immature balance in duffs.
    public var balanceImmature: UInt64
    /// Locked balance in duffs.
    public var balanceLocked: UInt64
    /// Wallet is spend-disabled — either bootstrapped watch-only
    /// (no seed) or every account is watch-only. Surfaces as the
    /// "👁 Watch-only" badge in the wallets list. Default `false`
    /// keeps the schema migration trivial for rows that predate
    /// this column.
    public var isWatchOnly: Bool = false
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
        network: String,
        name: String? = nil,
        birthHeight: UInt32 = 0,
        syncedHeight: UInt32 = 0,
        isWatchOnly: Bool = false,
        isImported: Bool = false
    ) {
        self.walletId = walletId
        self.network = network
        self.name = name
        self.birthHeight = birthHeight
        self.syncedHeight = syncedHeight
        self.lastSynced = 0
        self.balanceConfirmed = 0
        self.balanceUnconfirmed = 0
        self.balanceImmature = 0
        self.balanceLocked = 0
        self.isWatchOnly = isWatchOnly
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

    /// Parse the stored `network` raw string back into the app
    /// enum. Falls back to `.testnet` when the string doesn't
    /// match a known case — shouldn't happen in practice because
    /// we write via `AppNetwork.rawValue`, but the fallback keeps
    /// view code from having to handle the failure path.
    public var networkEnum: AppNetwork {
        AppNetwork(rawValue: network) ?? .testnet
    }
}

// MARK: - Queries

extension PersistentWallet {
    public static func predicate(walletId: Data) -> Predicate<PersistentWallet> {
        #Predicate<PersistentWallet> { $0.walletId == walletId }
    }
}
