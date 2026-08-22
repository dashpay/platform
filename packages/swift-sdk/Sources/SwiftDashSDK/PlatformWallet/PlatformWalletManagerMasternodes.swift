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
    /// Whether Rust could actually check platform-node ownership for this
    /// query (the wallet's derived platform-node index had entries). When
    /// false, `platformInWallet` is "unchecked" (empty/not-yet-rehydrated
    /// pool) and the persister must retain the prior value instead of
    /// clobbering it. When true, `platformInWallet` is definitive (true OR
    /// false), so an on-chain rotation to an external key correctly clears it.
    public let platformOwnershipChecked: Bool
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
                platformKeyIndex: entry.platform_key_index,
                platformOwnershipChecked: entry.platform_ownership_checked
            )
        }
    }

    /// Which masternode-withdrawal signing keys this wallet holds for the
    /// masternode `proTxHash` (stored WIRE order), plus its registered payout
    /// address. Seedless and local — no resolver, no network. The same
    /// resolution `masternodeWithdraw` signs with, so a UI that gates its
    /// Withdraw button / destination field on this can never enable a path
    /// the claim then refuses.
    public func masternodeWithdrawalKeys(
        walletId: Data,
        proTxHash: Data
    ) throws -> MasternodeWithdrawalKeys {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }

        var out = MasternodeWithdrawalKeysFFI()
        let ffiResult = walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                platform_wallet_manager_masternode_withdrawal_keys(
                    handle,
                    widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    &out
                )
            }
        }
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
        defer {
            if let payout = out.payout_address {
                platform_wallet_string_free(payout)
            }
        }
        return MasternodeWithdrawalKeys(
            ownerKeyIndex: out.owner_key_in_wallet ? out.owner_key_index : nil,
            transferKeyInWallet: out.transfer_key_in_wallet,
            payoutAddress: out.payout_address.map { String(cString: $0) }
        )
    }

    /// Claim (withdraw) `amountCredits` from a masternode's Platform
    /// identity to L1, signed with the wallet key `signingKey` derived
    /// through the Keychain-backed mnemonic resolver (the seed never becomes
    /// resident — same path as core sends). Returns the identity's remaining
    /// balance; throws on failure (surfaced to the confirmation UI).
    ///
    /// - `.owner`: Platform pays the registered payout address;
    ///   `destinationAddress` must be nil.
    /// - `.transfer`: `destinationAddress` (base58, this wallet's network)
    ///   is the destination; nil ⇒ the registered payout address.
    ///
    /// Pure bridge — the whole orchestration (masternode lookup, identity
    /// fetch, key selection + guards, derivation, sign, broadcast) lives in
    /// `platform-wallet` behind this one FFI call, per CLAUDE.md. The FFI
    /// blocks on the network round-trip, so it runs on a detached task.
    /// `proTxHash` is passed in stored WIRE order; Rust reverses it to the
    /// display-order identity id.
    public func masternodeWithdraw(
        walletId: Data,
        proTxHash: Data,
        amountCredits: UInt64,
        signingKey: MasternodeWithdrawalSigningKey,
        destinationAddress: String? = nil
    ) async throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }
        if signingKey == .owner, destinationAddress != nil {
            throw PlatformWalletError.invalidParameter(
                "an owner-key withdrawal pays the registered payout address; no destination can be chosen")
        }

        let handle = self.handle
        let useOwnerKey = signingKey == .owner
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            // Resolver-backed signer: the mnemonic is fetched from the
            // Keychain inside the resolver vtable Rust-side. Kept alive
            // across the synchronous FFI call, whose callback fires during it.
            let resolver = MnemonicResolver()
            var outBalance: UInt64 = 0
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                    proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                        func call(_ destPtr: UnsafePointer<CChar>?) -> PlatformWalletFFIResult {
                            platform_wallet_manager_masternode_withdraw(
                                handle,
                                widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                amountCredits,
                                useOwnerKey,
                                destPtr,
                                resolver.handle,
                                &outBalance
                            )
                        }
                        if let destinationAddress {
                            return destinationAddress.withCString { call($0) }
                        }
                        return call(nil)
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return outBalance
        }.value
    }
}

/// Which wallet key signs a masternode (evonode) credit withdrawal.
public enum MasternodeWithdrawalSigningKey: Sendable, Equatable {
    /// The `ProviderOwnerKeys` key — pays the registered payout address only.
    case owner
    /// The payout-script (identity `TRANSFER`) key — any destination.
    case transfer
}

/// Preflight for a masternode credit withdrawal: which signing keys this
/// wallet holds and where an owner-key claim is paid. See
/// `PlatformWalletManager.masternodeWithdrawalKeys(walletId:proTxHash:)`.
public struct MasternodeWithdrawalKeys: Sendable, Equatable {
    /// `ProviderOwnerKeys` index of the masternode's owner key when this
    /// wallet holds it; `nil` otherwise.
    public let ownerKeyIndex: UInt32?
    /// This wallet holds the payout-script key, so the destination may be
    /// changed.
    public let transferKeyInWallet: Bool
    /// Registered payout address, or `nil` when the node has no encodable
    /// payout script.
    public let payoutAddress: String?

    public init(ownerKeyIndex: UInt32?, transferKeyInWallet: Bool, payoutAddress: String?) {
        self.ownerKeyIndex = ownerKeyIndex
        self.transferKeyInWallet = transferKeyInWallet
        self.payoutAddress = payoutAddress
    }

    public var ownerKeyInWallet: Bool { ownerKeyIndex != nil }

    /// At least one wallet-held key can sign a withdrawal.
    public var canWithdraw: Bool { ownerKeyInWallet || transferKeyInWallet }

    /// The destination may differ from the payout address.
    public var canChooseDestination: Bool { transferKeyInWallet }

    /// The key a claim should sign with: the transfer key when available
    /// (it permits any destination), else the owner key.
    public var preferredSigningKey: MasternodeWithdrawalSigningKey? {
        if transferKeyInWallet { return .transfer }
        if ownerKeyInWallet { return .owner }
        return nil
    }
}
