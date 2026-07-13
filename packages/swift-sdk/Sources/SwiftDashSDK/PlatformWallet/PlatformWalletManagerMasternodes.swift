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
    /// Operator BLS public key (48 raw bytes), or nil.
    public let operatorPublicKey: Data?
    /// Platform node id (hash160, 20 bytes) for evonodes, or nil.
    public let platformNodeId: Data?
    /// Base58 payout address (Rust-encoded), or nil for a non-standard /
    /// unseen payout script.
    public let payoutAddress: String?
    /// Base58 P2PKH pseudo-address of `hash160(operator BLS key)`, or nil.
    /// The join key for operator-key ownership against persisted addresses.
    public let operatorPseudoAddress: String?
    /// Base58 P2PKH address of the platform node id (evonode), or nil.
    public let platformNodeAddress: String?

    /// Operator / platform key ownership, resolved in Rust by
    /// derive-and-compare (these keys have no on-chain address). Tag is
    /// 10 (operator) / 11 (platform); meaningful only when `inWallet`.
    public let operatorInWallet: Bool
    public let operatorAccountType: UInt8
    public let operatorKeyIndex: UInt32
    public let platformInWallet: Bool
    public let platformAccountType: UInt8
    public let platformKeyIndex: UInt32
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
                votingAddress: entry.voting_address.map { String(cString: $0) },
                operatorPublicKey: entry.has_operator_key
                    ? withUnsafeBytes(of: &entry.operator_public_key) { Data($0) }
                    : nil,
                platformNodeId: entry.has_platform_node_id
                    ? withUnsafeBytes(of: &entry.platform_node_id) { Data($0) }
                    : nil,
                payoutAddress: entry.payout_address.map { String(cString: $0) },
                operatorPseudoAddress: entry.operator_pseudo_address.map { String(cString: $0) },
                platformNodeAddress: entry.platform_node_address.map { String(cString: $0) },
                operatorInWallet: entry.operator_in_wallet,
                operatorAccountType: entry.operator_account_type,
                operatorKeyIndex: entry.operator_key_index,
                platformInWallet: entry.platform_in_wallet,
                platformAccountType: entry.platform_account_type,
                platformKeyIndex: entry.platform_key_index
            )
        }
    }

    /// Claim (withdraw) `amountCredits` from a masternode's Platform
    /// identity using the wallet-held OWNER key. Owner-key withdrawals pay
    /// the node's registered payout address (no chosen destination). Returns
    /// the new balance; throws on failure (surfaced to the confirmation UI).
    ///
    /// Pure bridge — the whole orchestration (identity fetch, OWNER-key
    /// guard, owner-key derivation + sign, withdraw broadcast) lives in
    /// `platform-wallet` behind this one FFI call, per CLAUDE.md.
    /// `proTxHash` is passed in stored WIRE order; Rust reverses it to the
    /// display-order identity id.
    public func masternodeWithdraw(
        walletId: Data,
        proTxHash: Data,
        amountCredits: UInt64,
        ownerKeyIndex: UInt32
    ) throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }

        var outBalance: UInt64 = 0
        let ffiResult = walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                platform_wallet_manager_masternode_withdraw(
                    handle,
                    widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    amountCredits,
                    ownerKeyIndex,
                    nil,  // dest_address: owner-key path pays the registered payout address
                    true, // use_owner_key
                    &outBalance
                )
            }
        }

        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
        return outBalance
    }
}
