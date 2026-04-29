import Foundation
import SwiftData

/// SwiftData model for a single on-chain address tracked in a wallet's
/// address pool (external / internal / absent).
///
/// Populated by the Rust-side `on_persist_account_address_pools_fn`
/// callback, which fires at wallet creation (initial gap-limit fill),
/// on pool extension (`next_unused` past the current tip), and when
/// SPV marks an address used.
///
/// Cascade-owned by `PersistentAccount`.
@Model
public final class PersistentCoreAddress {
    /// Base58check-encoded address. Unique across the SwiftData store
    /// because the same address can't validly exist under two accounts
    /// (collision would imply a wallet-id hash collision).
    @Attribute(.unique) public var address: String
    /// 33-byte compressed secp256k1 public key, or empty Data when the
    /// Rust side couldn't produce one (e.g. BLS accounts, or pool
    /// entries that stored only a script). Watch-only-restored
    /// accounts currently always populate this.
    public var publicKey: Data
    /// `AddressPoolTypeTagFFI` raw value — 0 External, 1 Internal,
    /// 2 Absent, 3 AbsentHardened.
    public var poolTypeTag: UInt8
    /// Derivation index within this pool.
    public var addressIndex: UInt32
    /// BIP32 derivation path (e.g. `"m/44'/1'/0'/0/3"`).
    public var derivationPath: String
    /// Marked used by the Rust address pool (first-seen tx or explicit
    /// `mark_used`).
    public var isUsed: Bool
    /// SPV height where this address first appeared in a transaction.
    /// Zero until the address is seen on-chain.
    public var firstSeenHeight: UInt32
    /// SPV height of the most recent transaction touching this address.
    public var lastSeenHeight: UInt32
    /// Cached balance in duffs from `AddressInfo.balance`. Updated by
    /// subsequent `on_persist_account_address_pools_fn` pulses.
    public var balance: UInt64
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent account.
    public var account: PersistentAccount?

    /// TXOs paid to this address. Cascade-delete: dropping the
    /// address row takes its TXOs with it. The address is the
    /// canonical owning record — no meaningful render path for an
    /// address-less TXO. Pool rebuilds therefore need to reuse
    /// existing rows (the persister upserts by Base58Check string,
    /// which it already does) rather than wholesale-replace, or
    /// the historical TXO chain gets wiped.
    @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.coreAddress)
    public var txos: [PersistentTxo] = []

    public init(
        address: String,
        publicKey: Data = Data(),
        poolTypeTag: UInt8,
        addressIndex: UInt32,
        derivationPath: String,
        isUsed: Bool = false,
        balance: UInt64 = 0
    ) {
        self.address = address
        self.publicKey = publicKey
        self.poolTypeTag = poolTypeTag
        self.addressIndex = addressIndex
        self.derivationPath = derivationPath
        self.isUsed = isUsed
        self.firstSeenHeight = 0
        self.lastSeenHeight = 0
        self.balance = balance
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

extension PersistentCoreAddress {
    public var poolTypeName: String {
        switch poolTypeTag {
        case 0: return "External"
        case 1: return "Internal"
        case 2: return "Absent"
        case 3: return "Absent (Hardened)"
        default: return "Unknown(\(poolTypeTag))"
        }
    }
}
