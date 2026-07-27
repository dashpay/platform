import Foundation
import SwiftData

/// SwiftData model for persisting DIP-17 Platform Payment addresses.
///
/// Each record represents one HD-derived Platform Payment address,
/// combining the derivation metadata (populated by the Rust
/// `on_persist_account_address_pools_fn` callback at wallet creation
/// / pool extension) with the credit balance + nonce snapshot reported
/// by the BLAST sync round. Records are upserted incrementally —
/// address emits seed the row, balance emits refresh `balance` /
/// `nonce` / `isUsed` / `last_seen_height`.
///
/// Peer of [`PersistentCoreAddress`], which handles every non-
/// PlatformPayment address pool (BIP44, CoinJoin, identity, provider,
/// DashPay). Splitting the two keeps each model shaped around its
/// actual data — Core addresses carry base58check strings, Platform
/// addresses carry bech32m (DIP-0018).
///
/// Cascade-owned by `PersistentAccount` (PlatformPayment account,
/// type tag 14).
@Model
public final class PersistentPlatformAddress {
    /// Index `walletId` so per-wallet platform-address scans —
    /// `predicate(walletId:)`, the storage explorer's network scope
    /// fallback, BLAST-sync re-upsert paths — hit an index instead
    /// of scanning the whole table.
    #Index<PersistentPlatformAddress>([\.walletId])

    /// DIP-0018 bech32m-encoded address (`dash1…` / `tdash1…`). Unique
    /// across the SwiftData store — a collision would imply a wallet-
    /// id / derivation path collision.
    @Attribute(.unique) public var address: String
    /// `PlatformAddress` type byte: 0 = P2PKH, 1 = P2SH. Matches the
    /// discriminant emitted by the Rust-side FFI.
    public var addressType: UInt8
    /// 20-byte address hash. Kept denormalized so the BLAST balance
    /// callback (which gets hashes, not full addresses) can upsert in
    /// one fetch.
    @Attribute(.unique) public var addressHash: Data
    /// 33-byte compressed secp256k1 public key, or empty Data if the
    /// Rust side couldn't produce one (pool entries that stored only
    /// a script, etc.).
    public var publicKey: Data
    /// DIP-17 account index (field `account` in `PlatformPayment`).
    public var accountIndex: UInt32
    /// DIP-17 derivation index within the account.
    public var addressIndex: UInt32
    /// BIP32 derivation path (e.g. `"m/9'/5'/17'/0'/0'/0"`).
    public var derivationPath: String
    /// Marked used by the Rust address pool (first-seen tx or explicit
    /// `mark_used`), or auto-flipped by BLAST when a non-zero
    /// balance / nonce first arrives.
    public var isUsed: Bool
    /// Credit balance in credits (1e11 credits per DASH).
    public var balance: UInt64
    /// Current anti-replay nonce.
    public var nonce: UInt32
    /// Platform block height where this address first appeared in a
    /// balance changeset. Zero until the address is seen on-chain.
    public var firstSeenHeight: UInt32
    /// Platform block height this row's `balance` is current **as of**
    /// — the balance height pin (`AddressFunds::as_of_height` in Rust).
    /// Round-tripped verbatim through the persistence callbacks so the
    /// sync's delta-replay gate survives restarts. Zero means "unknown
    /// provenance" (rows persisted before the pin existed).
    public var lastSeenHeight: UInt64
    /// 32-byte wallet ID that owns this address. Denormalized from
    /// `account.wallet.walletId` so per-wallet `@Query` filters don't
    /// have to traverse two optional relationships.
    public var walletId: Data
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent account (PlatformPayment, type tag 14).
    public var account: PersistentAccount?

    public init(
        address: String,
        addressType: UInt8,
        addressHash: Data,
        publicKey: Data = Data(),
        accountIndex: UInt32,
        addressIndex: UInt32,
        derivationPath: String,
        isUsed: Bool = false,
        balance: UInt64 = 0,
        nonce: UInt32 = 0,
        walletId: Data
    ) {
        self.address = address
        self.addressType = addressType
        self.addressHash = addressHash
        self.publicKey = publicKey
        self.accountIndex = accountIndex
        self.addressIndex = addressIndex
        self.derivationPath = derivationPath
        self.isUsed = isUsed
        self.balance = balance
        self.nonce = nonce
        self.firstSeenHeight = 0
        self.lastSeenHeight = 0
        self.walletId = walletId
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

// MARK: - Queries

extension PersistentPlatformAddress {
    public static func predicate(walletId: Data) -> Predicate<PersistentPlatformAddress> {
        #Predicate<PersistentPlatformAddress> { entry in
            entry.walletId == walletId
        }
    }

    public static var nonZeroBalancesPredicate: Predicate<PersistentPlatformAddress> {
        #Predicate<PersistentPlatformAddress> { entry in
            entry.balance > 0
        }
    }
}
