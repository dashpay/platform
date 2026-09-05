import DashSDKFFI
import Foundation

/// Which provider-key family a rotation key picker lists. Raw values are
/// the account-type tags every provider-key FFI uses.
public enum RotationKeyKind: UInt8, Sendable {
    case operatorBLS = 10
    case votingECDSA = 8
}

/// One wallet provider key with its network-wide usage — a rotation
/// key-picker row. `usedByProTxHash` (wire order) names the masternode-list
/// entry currently using the key; `nil` means unused network-wide, which
/// for operator keys is a consensus requirement of a ProUpRegTx.
public struct ProviderKeyCandidate: Sendable {
    public let index: UInt32
    /// Modern-serialization public key bytes (48 BLS operator, 33 secp voting).
    public let publicKey: Data
    /// P2PKH address (voting keys only).
    public let address: String?
    public let usedByProTxHash: Data?
}

/// A zeroed 32-byte tuple for txid out-params.
private func zeroTxidTuple() -> (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
) {
    (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
}

/// Rust reads C strings up to the first NUL, so an embedded U+0000 would
/// silently truncate the value — the transaction would then use a different
/// payout/key/service than the caller supplied. Matches the guard the other
/// wrappers apply (`ManagedPlatformAddressWallet`, `Mnemonic`).
private func requireNoEmbeddedNul(_ value: String, _ label: String) throws {
    guard !value.utf8.contains(0) else {
        throw PlatformWalletError.invalidParameter(
            "\(label) contains an embedded NUL character")
    }
}

extension PlatformWalletManager {

    // MARK: - Key candidates

    /// The wallet's first `count` provider keys of `kind`, each joined
    /// against the live masternode list. Throws
    /// `.masternodeListUnavailable` before the list has synced — "unused"
    /// cannot be asserted without it. The FFI blocks (derivation + list
    /// join), so it runs on a detached task.
    public func providerKeyCandidates(
        walletId: Data,
        kind: RotationKeyKind,
        count: UInt32 = 20
    ) async throws -> [ProviderKeyCandidate] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32,
            count <= UInt32(PLATFORM_WALLET_PROVIDER_KEY_CANDIDATES_MAX)
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, wallet id not 32 bytes, or count above the candidates maximum")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> [ProviderKeyCandidate] in
            var outEntries: UnsafeMutablePointer<ProviderKeyCandidateFFI>?
            var outCount: UInt = 0
            let ffiResult = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                platform_wallet_manager_provider_key_candidates(
                    handle,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    kind.rawValue,
                    count,
                    &outEntries,
                    &outCount)
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            guard let entries = outEntries, outCount > 0 else { return [] }
            defer { platform_wallet_manager_free_provider_key_candidates(entries, outCount) }
            return (0..<Int(outCount)).map { i in
                let entry = entries[i]
                var publicKeyTuple = entry.public_key
                let publicKey = Swift.withUnsafeBytes(of: &publicKeyTuple) {
                    Data($0.prefix(Int(entry.public_key_len)))
                }
                var usedByTuple = entry.used_by_pro_tx_hash
                let usedBy = entry.used
                    ? Swift.withUnsafeBytes(of: &usedByTuple) { Data($0) }
                    : nil
                return ProviderKeyCandidate(
                    index: entry.index,
                    publicKey: publicKey,
                    address: entry.address.map { String(cString: $0) },
                    usedByProTxHash: usedBy)
            }
        }.value
    }

    // MARK: - Registrar update (key rotation)

    /// Rotate a wallet-owned masternode's operator and/or voting key to
    /// fresh wallet keys with an owner-signed ProUpRegTx. Pure bridge — the
    /// preflights (owner key vs the ProRegTx, network-wide operator-key
    /// uniqueness, the payout rule) live in `platform-wallet`.
    ///
    /// Rotating the operator key PoSe-bans the node with its service fields
    /// reset until `masternodeUpdateServiceWithValues` reactivates it —
    /// capture the entry's service values BEFORE calling this.
    ///
    /// `payoutAddress` is always required: the payload replaces the payout
    /// script on-chain. Returns the txid (32 wire-order bytes); a
    /// `.transactionBroadcastUnconfirmed` error is ambiguous — never retry.
    public func masternodeUpdateRegistrar(
        walletId: Data,
        proTxHash: Data,
        ownerKeyIndex: UInt32,
        newOperatorKeyIndex: UInt32?,
        newVotingKeyIndex: UInt32?,
        payoutAddress: String
    ) async throws -> Data {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }
        try requireNoEmbeddedNul(payoutAddress, "payout address")
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> Data in
            let resolver = MnemonicResolver()
            var txidTuple = zeroTxidTuple()
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                    proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                        payoutAddress.withCString { cPayout in
                            platform_wallet_manager_masternode_update_registrar(
                                handle,
                                widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                ownerKeyIndex,
                                newOperatorKeyIndex != nil, newOperatorKeyIndex ?? 0,
                                newVotingKeyIndex != nil, newVotingKeyIndex ?? 0,
                                cPayout,
                                resolver.handle,
                                &txidTuple)
                        }
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return Swift.withUnsafeBytes(of: &txidTuple) { Data($0) }
        }.value
    }

    /// Prepare-only sibling of `masternodeUpdateRegistrar` for the
    /// review-before-broadcast step; ownership matches
    /// `masternodePrepareUpdateService`.
    public func masternodePrepareUpdateRegistrar(
        walletId: Data,
        proTxHash: Data,
        ownerKeyIndex: UInt32,
        newOperatorKeyIndex: UInt32?,
        newVotingKeyIndex: UInt32?,
        payoutAddress: String
    ) async throws -> FinalizedCoreTransaction {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }
        try requireNoEmbeddedNul(payoutAddress, "payout address")
        let handle = self.handle
        let transactionHandle = try await Task.detached(priority: .userInitiated) { () -> Handle in
            let resolver = MnemonicResolver()
            var outHandle: Handle = NULL_HANDLE
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                    proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                        payoutAddress.withCString { cPayout in
                            platform_wallet_manager_masternode_prepare_update_registrar(
                                handle,
                                widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                ownerKeyIndex,
                                newOperatorKeyIndex != nil, newOperatorKeyIndex ?? 0,
                                newVotingKeyIndex != nil, newVotingKeyIndex ?? 0,
                                cPayout,
                                resolver.handle,
                                &outHandle)
                        }
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return outHandle
        }.value
        return try FinalizedCoreTransaction(handle: transactionHandle)
    }

    /// `masternodeUpdateRegistrar` for a TRACKED masternode: the owner key
    /// is the host-vaulted key text (WIF or 64-char hex); the fee and the
    /// new keys still come from `walletId`.
    public func trackedMasternodeUpdateRegistrar(
        walletId: Data,
        proTxHash: Data,
        ownerKey: String,
        newOperatorKeyIndex: UInt32?,
        newVotingKeyIndex: UInt32?,
        payoutAddress: String
    ) async throws -> Data {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }
        try requireNoEmbeddedNul(ownerKey, "owner key")
        try requireNoEmbeddedNul(payoutAddress, "payout address")
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> Data in
            let resolver = MnemonicResolver()
            var txidTuple = zeroTxidTuple()
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                    proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                        ownerKey.withCString { cKey in
                            payoutAddress.withCString { cPayout in
                                platform_wallet_manager_tracked_masternode_update_registrar(
                                    handle,
                                    widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                    ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                    cKey,
                                    newOperatorKeyIndex != nil, newOperatorKeyIndex ?? 0,
                                    newVotingKeyIndex != nil, newVotingKeyIndex ?? 0,
                                    cPayout,
                                    resolver.handle,
                                    &txidTuple)
                            }
                        }
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return Swift.withUnsafeBytes(of: &txidTuple) { Data($0) }
        }.value
    }

    /// Prepare-only sibling of `trackedMasternodeUpdateRegistrar`.
    public func trackedMasternodePrepareUpdateRegistrar(
        walletId: Data,
        proTxHash: Data,
        ownerKey: String,
        newOperatorKeyIndex: UInt32?,
        newVotingKeyIndex: UInt32?,
        payoutAddress: String
    ) async throws -> FinalizedCoreTransaction {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or wallet id / proTxHash not 32 bytes")
        }
        try requireNoEmbeddedNul(ownerKey, "owner key")
        try requireNoEmbeddedNul(payoutAddress, "payout address")
        let handle = self.handle
        let transactionHandle = try await Task.detached(priority: .userInitiated) { () -> Handle in
            let resolver = MnemonicResolver()
            var outHandle: Handle = NULL_HANDLE
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                walletId.withUnsafeBytes { (widRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                    proTxHash.withUnsafeBytes { (ptRaw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                        ownerKey.withCString { cKey in
                            payoutAddress.withCString { cPayout in
                                platform_wallet_manager_tracked_masternode_prepare_update_registrar(
                                    handle,
                                    widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                    ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                                    cKey,
                                    newOperatorKeyIndex != nil, newOperatorKeyIndex ?? 0,
                                    newVotingKeyIndex != nil, newVotingKeyIndex ?? 0,
                                    cPayout,
                                    resolver.handle,
                                    &outHandle)
                            }
                        }
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return outHandle
        }.value
        return try FinalizedCoreTransaction(handle: transactionHandle)
    }

    // MARK: - Stage two: explicit-values service update

    /// Reactivate a masternode after an operator-key rotation: broadcast a
    /// ProUpServTx re-asserting caller-captured service values, signed with
    /// the wallet's operator key at `operatorKeyIndex` (post-rotation, the
    /// operator key is by definition a wallet key). For an evonode all
    /// three platform values are required; for a regular masternode all
    /// must be nil. `operatorPayoutAddress` follows the same reward-driven
    /// rule as the unban path.
    public func masternodeUpdateServiceWithValues(
        walletId: Data,
        proTxHash: Data,
        operatorKeyIndex: UInt32,
        serviceAddress: String,
        platformNodeId: Data? = nil,
        platformP2PPort: UInt16? = nil,
        platformHTTPPort: UInt16? = nil,
        operatorPayoutAddress: String? = nil
    ) async throws -> Data {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32,
            platformNodeId == nil || platformNodeId?.count == 20
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, wallet id / proTxHash not 32 bytes, or platform node id not 20 bytes")
        }
        try requireNoEmbeddedNul(serviceAddress, "service address")
        if let operatorPayoutAddress {
            try requireNoEmbeddedNul(operatorPayoutAddress, "operator payout address")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> Data in
            let resolver = MnemonicResolver()
            var txidTuple = zeroTxidTuple()
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                Self.withServiceValuePointers(
                    walletId: walletId, proTxHash: proTxHash, platformNodeId: platformNodeId
                ) { widPtr, ptPtr, nodeIdPtr in
                    serviceAddress.withCString { cService -> PlatformWalletFFIResult in
                        func call(_ cPayout: UnsafePointer<CChar>?) -> PlatformWalletFFIResult {
                            platform_wallet_manager_masternode_update_service_with_values(
                                handle, widPtr, ptPtr,
                                operatorKeyIndex,
                                cService,
                                nodeIdPtr != nil, nodeIdPtr,
                                platformP2PPort != nil, platformP2PPort ?? 0,
                                platformHTTPPort != nil, platformHTTPPort ?? 0,
                                cPayout,
                                resolver.handle,
                                &txidTuple)
                        }
                        if let operatorPayoutAddress {
                            return operatorPayoutAddress.withCString { call($0) }
                        }
                        return call(nil)
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return Swift.withUnsafeBytes(of: &txidTuple) { Data($0) }
        }.value
    }

    /// Prepare-only sibling of `masternodeUpdateServiceWithValues`.
    public func masternodePrepareUpdateServiceWithValues(
        walletId: Data,
        proTxHash: Data,
        operatorKeyIndex: UInt32,
        serviceAddress: String,
        platformNodeId: Data? = nil,
        platformP2PPort: UInt16? = nil,
        platformHTTPPort: UInt16? = nil,
        operatorPayoutAddress: String? = nil
    ) async throws -> FinalizedCoreTransaction {
        guard isConfigured, handle != NULL_HANDLE,
            walletId.count == 32, proTxHash.count == 32,
            platformNodeId == nil || platformNodeId?.count == 20
        else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, wallet id / proTxHash not 32 bytes, or platform node id not 20 bytes")
        }
        try requireNoEmbeddedNul(serviceAddress, "service address")
        if let operatorPayoutAddress {
            try requireNoEmbeddedNul(operatorPayoutAddress, "operator payout address")
        }
        let handle = self.handle
        let transactionHandle = try await Task.detached(priority: .userInitiated) { () -> Handle in
            let resolver = MnemonicResolver()
            var outHandle: Handle = NULL_HANDLE
            let ffiResult = withExtendedLifetime(resolver) { () -> PlatformWalletFFIResult in
                Self.withServiceValuePointers(
                    walletId: walletId, proTxHash: proTxHash, platformNodeId: platformNodeId
                ) { widPtr, ptPtr, nodeIdPtr in
                    serviceAddress.withCString { cService -> PlatformWalletFFIResult in
                        func call(_ cPayout: UnsafePointer<CChar>?) -> PlatformWalletFFIResult {
                            platform_wallet_manager_masternode_prepare_update_service_with_values(
                                handle, widPtr, ptPtr,
                                operatorKeyIndex,
                                cService,
                                nodeIdPtr != nil, nodeIdPtr,
                                platformP2PPort != nil, platformP2PPort ?? 0,
                                platformHTTPPort != nil, platformHTTPPort ?? 0,
                                cPayout,
                                resolver.handle,
                                &outHandle)
                        }
                        if let operatorPayoutAddress {
                            return operatorPayoutAddress.withCString { call($0) }
                        }
                        return call(nil)
                    }
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return outHandle
        }.value
        return try FinalizedCoreTransaction(handle: transactionHandle)
    }

    /// Nested pointer marshalling for the stage-two calls: wallet id +
    /// proTxHash + optional 20-byte platform node id.
    private nonisolated static func withServiceValuePointers<T>(
        walletId: Data,
        proTxHash: Data,
        platformNodeId: Data?,
        _ body: (UnsafePointer<UInt8>?, UnsafePointer<UInt8>?, UnsafePointer<UInt8>?) -> T
    ) -> T {
        walletId.withUnsafeBytes { widRaw in
            proTxHash.withUnsafeBytes { ptRaw in
                let widPtr = widRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                let ptPtr = ptRaw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                if let platformNodeId {
                    return platformNodeId.withUnsafeBytes { nodeRaw in
                        body(widPtr, ptPtr, nodeRaw.baseAddress?.assumingMemoryBound(to: UInt8.self))
                    }
                }
                return body(widPtr, ptPtr, nil)
            }
        }
    }
}
