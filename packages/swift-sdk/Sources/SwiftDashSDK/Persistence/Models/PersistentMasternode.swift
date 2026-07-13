import Foundation
import SwiftData

/// SwiftData model for a masternode entity aggregated by Rust from a
/// wallet's provider special transactions (ProRegTx / ProUpServTx /
/// ProUpRegTx / ProUpRevTx), grouped by proTxHash.
///
/// **The aggregation is done in Rust** (`aggregate_masternodes` in
/// `rs-platform-wallet-ffi`) and delivered as a flat array over the FFI
/// query seam; this row is pure storage of that result. Swift never
/// decodes DIP-3 payloads or decides grouping/status — it only persists
/// and loads.
///
/// Not scoped by a `networkRaw` column of its own — network membership
/// is recovered by joining `walletId` to the parent
/// `PersistentWallet.networkRaw`, the same pivot `PersistentPlatformAddress`
/// / shielded rows use. `(walletId, proTxHash)` is unique so a masternode
/// tracked under one wallet collapses onto a single row across re-syncs.
@Model
public final class PersistentMasternode {
    #Unique<PersistentMasternode>([\.walletId, \.proTxHash])

    /// Owning wallet id (32 bytes). Network is derived via the wallet.
    public var walletId: Data
    /// proTxHash (32 raw wire bytes) — the ProRegTx's own txid; updates
    /// and revocations link to it.
    public var proTxHash: Data
    /// Registration txid (== proTxHash when the ProRegTx was in the set;
    /// otherwise the same bytes, with `hasRegistration == false`).
    public var registrationTxid: Data

    /// Latest known service endpoint `"ip:port"` (latest ProUpServTx
    /// wins; seeded by the ProRegTx address). `nil` if never seen.
    public var serviceAddress: String?
    /// `true` = evonode / HPMN (ProRegTx `masternode_type`).
    public var isEvonode: Bool

    /// Owner / voting key hashes (hash160, 20 bytes) from the ProRegTx /
    /// latest ProUpRegTx.
    public var ownerKeyHash: Data?
    public var votingKeyHash: Data?
    /// Base58 owner / voting addresses, encoded on the Rust side for the
    /// network. Used by the address-list subtitle to join a provider-key
    /// account address (also base58) to its masternode without any
    /// Swift-side key hashing.
    public var ownerAddress: String?
    public var votingAddress: String?

    /// ProRegTx collateral outpoint.
    public var collateralTxid: Data?
    public var collateralVout: UInt32

    /// A ProUpRevTx was seen. Retained as data only — the *displayed*
    /// status comes from [`statusRaw`] (the DML), since a revoked node
    /// naturally leaves the list and shows as Retired.
    public var revoked: Bool
    public var revocationReason: UInt16
    /// DML-derived status: 0 Active, 1 Inactive, 2 Retired, 3 Unknown.
    /// Rust returns Unknown (3) when the deterministic masternode list
    /// isn't available yet; the persist step then KEEPS the previously
    /// stored value rather than overwriting with Unknown. Defaults to
    /// Unknown for a freshly inserted row.
    public var statusRaw: UInt8 = 3

    /// Core height of the ProRegTx (0 when unseen) and whether a
    /// registration was in the aggregated set.
    public var registrationHeight: UInt32
    public var hasRegistration: Bool
    /// Count of provider txs seen for this proTxHash.
    public var txCount: UInt32

    /// Stable position in Rust's registration-order sort (ascending
    /// registration height, then proTxHash). Query rows sort by this so
    /// ordering is identical everywhere.
    public var orderIndex: UInt32
    /// 1-based position within the entity's own type sequence, computed
    /// by Rust over the same registration-order sort — evonodes and
    /// regular masternodes number independently ("Evonode 2",
    /// "Masternode 5").
    public var typeIndex: UInt32 = 0

    public var createdAt: Date
    public var lastUpdated: Date

    public init(
        walletId: Data,
        proTxHash: Data,
        registrationTxid: Data,
        serviceAddress: String? = nil,
        isEvonode: Bool = false,
        ownerKeyHash: Data? = nil,
        votingKeyHash: Data? = nil,
        ownerAddress: String? = nil,
        votingAddress: String? = nil,
        collateralTxid: Data? = nil,
        collateralVout: UInt32 = 0,
        revoked: Bool = false,
        revocationReason: UInt16 = 0,
        statusRaw: UInt8 = 3,
        registrationHeight: UInt32 = 0,
        hasRegistration: Bool = false,
        txCount: UInt32 = 0,
        orderIndex: UInt32 = 0,
        typeIndex: UInt32 = 0
    ) {
        self.walletId = walletId
        self.proTxHash = proTxHash
        self.registrationTxid = registrationTxid
        self.serviceAddress = serviceAddress
        self.isEvonode = isEvonode
        self.ownerKeyHash = ownerKeyHash
        self.votingKeyHash = votingKeyHash
        self.ownerAddress = ownerAddress
        self.votingAddress = votingAddress
        self.collateralTxid = collateralTxid
        self.collateralVout = collateralVout
        self.revoked = revoked
        self.revocationReason = revocationReason
        self.statusRaw = statusRaw
        self.registrationHeight = registrationHeight
        self.hasRegistration = hasRegistration
        self.txCount = txCount
        self.orderIndex = orderIndex
        self.typeIndex = typeIndex
        self.createdAt = Date()
        self.lastUpdated = Date()
    }

    // MARK: - Display helpers

    /// proTxHash in block-explorer (reversed) hex — matches
    /// `PersistentTransaction.txidHex`'s display-order convention.
    public var proTxHashHex: String {
        proTxHash.reversed().map { String(format: "%02x", $0) }.joined()
    }

    /// Short `aabbcc…ddeeff` proTxHash for compact rows.
    public var proTxHashShort: String {
        let hex = proTxHashHex
        guard hex.count >= 12 else { return hex }
        return "\(String(hex.prefix(6)))…\(String(hex.suffix(6)))"
    }

    /// User-facing number (1-based) within the entity's own type
    /// sequence — evonodes and masternodes number independently.
    public var displayNumber: Int {
        Int(typeIndex)
    }

    /// "Evonode" for HPMN, else "Masternode".
    public var typeName: String {
        isEvonode ? "Evonode" : "Masternode"
    }

    /// "Evonode 2" / "Masternode 5" — the canonical row title, shared by
    /// the masternode list and the address-usage subtitle so the number
    /// never diverges between screens.
    public var displayTitle: String {
        "\(typeName) \(displayNumber)"
    }

    /// Typed view of [`statusRaw`].
    public var status: MasternodeStatus {
        MasternodeStatus(rawValue: statusRaw) ?? .unknown
    }

    /// Display status: Active / Inactive / Retired / Unknown.
    public var statusName: String {
        status.displayName
    }
}

/// DML-derived masternode status, mirroring the Rust
/// `MasternodeStatus` discriminant emitted over the FFI.
public enum MasternodeStatus: UInt8 {
    /// In the masternode list and valid / enabled.
    case active = 0
    /// In the list but flagged invalid (PoSe-banned).
    case inactive = 1
    /// Not in the list (collateral spent / revoked / expired).
    case retired = 2
    /// The list isn't available yet — status is indeterminate.
    case unknown = 3

    public var displayName: String {
        switch self {
        case .active: return "Active"
        case .inactive: return "Inactive"
        case .retired: return "Retired"
        case .unknown: return "Unknown"
        }
    }
}
