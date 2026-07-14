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
    /// Typed public key bytes, or empty Data when the Rust side couldn't
    /// produce one (e.g. a pool entry that stored only a script). The
    /// curve is given by `keyType`: 33-byte compressed secp256k1 (ECDSA),
    /// 48-byte BLS operator key, or 32-byte Ed25519 platform-node key.
    public var publicKey: Data
    /// `KeyTypeTagFFI` raw value identifying the curve of `publicKey`:
    /// 0 ECDSA / 1 BLS / 2 EdDSA. Meaningful only when `publicKey` is
    /// non-empty. The stored default (NOT just the init-parameter
    /// default, which SwiftData migration never consults) keeps
    /// pre-column stores openable: without it, lightweight migration
    /// fails with "missing attribute values on mandatory destination
    /// attribute" and the container refuses to load — a launch crash on
    /// every device that has existing rows. Defaulted rows read as
    /// ECDSA until the next Rust address-pool persist pulse re-tags
    /// typed entries, which it does on every load.
    public var keyType: UInt8 = 0
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
        keyType: UInt8 = 0,
        poolTypeTag: UInt8,
        addressIndex: UInt32,
        derivationPath: String,
        isUsed: Bool = false,
        balance: UInt64 = 0
    ) {
        self.address = address
        self.publicKey = publicKey
        self.keyType = keyType
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
    /// User-facing name for the address pool this row belongs to.
    ///
    /// Pool tags 2/3 are key-wallet's "Absent" / "AbsentHardened"
    /// pools — keys derived on demand *outside* the BIP44
    /// external-receive / internal-change chains (this is where
    /// provider owner / voting / operator keys and other special-purpose
    /// keys live). "Absent" is Rust-enum jargon, so it's surfaced here as
    /// "Additional" / "Additional (Hardened)" — the source of truth the
    /// app-layer address lists reuse (AccountDetailView,
    /// StorageRecordDetailViews, WalletMemoryExplorerView).
    public var poolTypeName: String {
        switch poolTypeTag {
        case 0: return "External"
        case 1: return "Internal"
        case 2: return "Additional"
        case 3: return "Additional (Hardened)"
        default: return "Unknown(\(poolTypeTag))"
        }
    }
}
