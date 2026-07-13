import Foundation
import DashSDKFFI

/// One aggregated masternode, surfaced by the Rust query
/// `platform_wallet_manager_list_masternodes` (grouped by proTxHash over
/// the wallet's retained provider special transactions). A value
/// snapshot; the SwiftData mirror is `PersistentMasternode`.
///
/// All aggregation / DIP-3 decoding happens in Rust — this is pure
/// bridging.
public struct PlatformMasternode: Sendable {
    /// proTxHash (32 raw wire bytes) — the group key / registration txid.
    public let proTxHash: Data
    public let hasRegistration: Bool
    public let registrationHeight: UInt32
    /// Stable cross-type registration-order position (sort key).
    public let orderIndex: UInt32
    /// 1-based index within this masternode's type — pairs with
    /// `isEvonode` to render "Evonode N" / "Masternode N".
    public let typeIndex: UInt32
    public let isEvonode: Bool
    public let revoked: Bool
    public let revocationReason: UInt16
    /// DML-derived status discriminant (0 Active … 3 Unknown). `3`
    /// (Unknown) means the persist layer should keep the prior value.
    public let status: UInt8
    public let txCount: UInt32
    public let collateralTxid: Data?
    public let collateralVout: UInt32
    public let ownerKeyHash: Data?
    public let votingKeyHash: Data?
    public let serviceAddress: String?
    /// Base58 owner / voting P2PKH addresses (Rust-encoded for the
    /// network) — the join key for a provider-key account's address rows.
    public let ownerAddress: String?
    public let votingAddress: String?
}

extension PlatformWalletManager {
    /// Aggregate the wallet's masternodes from Rust. Empty when the wallet
    /// has none or the manager isn't configured. Pure marshalling — no
    /// aggregation or payload decoding on the Swift side.
    public func masternodes(for walletId: Data) -> [PlatformMasternode] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else {
            return []
        }

        var outEntries: UnsafePointer<MasternodeEntryFFI>?
        var outCount: UInt = 0

        let ffiResult = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            let base = raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
            return platform_wallet_manager_list_masternodes(
                handle,
                base,
                &outEntries,
                &outCount
            )
        }

        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            recordLastError(PlatformWalletError(result: result))
            return []
        }

        guard let entries = outEntries, outCount > 0 else {
            return []
        }

        defer {
            platform_wallet_manager_free_masternodes(
                UnsafeMutablePointer(mutating: entries),
                outCount
            )
        }

        return (0..<Int(outCount)).map { i in
            var entry = entries[i]
            let proTx = withUnsafeBytes(of: &entry.pro_tx_hash) { Data($0) }
            let collateralTxid = entry.has_collateral
                ? withUnsafeBytes(of: &entry.collateral_txid) { Data($0) }
                : nil
            let ownerHash = entry.has_owner_key_hash
                ? withUnsafeBytes(of: &entry.owner_key_hash) { Data($0) }
                : nil
            let votingHash = entry.has_voting_key_hash
                ? withUnsafeBytes(of: &entry.voting_key_hash) { Data($0) }
                : nil
            return PlatformMasternode(
                proTxHash: proTx,
                hasRegistration: entry.has_registration,
                registrationHeight: entry.registration_height,
                orderIndex: entry.order_index,
                typeIndex: entry.type_index,
                isEvonode: entry.is_evonode,
                revoked: entry.revoked,
                revocationReason: entry.revocation_reason,
                status: entry.status,
                txCount: entry.tx_count,
                collateralTxid: collateralTxid,
                collateralVout: entry.has_collateral ? entry.collateral_vout : 0,
                ownerKeyHash: ownerHash,
                votingKeyHash: votingHash,
                serviceAddress: entry.service_address.map { String(cString: $0) },
                ownerAddress: entry.owner_address.map { String(cString: $0) },
                votingAddress: entry.voting_address.map { String(cString: $0) }
            )
        }
    }
}
