import DashSDKFFI
import Foundation

/// What the host can do with a masternode given the key roles it holds for
/// it. Computed in Rust (`platform_wallet_masternode_capabilities`) so iOS
/// and Android gate actions identically.
public struct MasternodeCapabilities: Sendable, Equatable {
    /// Withdraw the owner identity's claimable balance (owner key or the
    /// payout-address key).
    public let canWithdraw: Bool
    /// Cast governance / contested-resource votes.
    public let canVote: Bool
    /// Sign ProUpServTx (operator BLS key).
    public let canUpdateService: Bool
    /// Prove which Tenderdash node this is (no wallet action uses it).
    public let identifiesPlatformNode: Bool

    /// Capabilities for the key roles the host holds.
    public init(holding roles: Set<MasternodeKeyRole>) {
        let mask = roles.reduce(UInt8(0)) { $0 | (1 << $1.rawValue) }
        var bits: UInt8 = 0
        var result = platform_wallet_masternode_capabilities(mask, &bits)
        platform_wallet_ffi_result_free(&result)
        canWithdraw = bits & 1 != 0
        canVote = bits & (1 << 1) != 0
        canUpdateService = bits & (1 << 2) != 0
        identifiesPlatformNode = bits & (1 << 3) != 0
    }
}

extension PlatformWalletManager {
    /// Whether tracked masternodes survive an app restart with the
    /// configured persistence backend. When `false` (a host that didn't
    /// wire the tracked-masternode persistence callbacks), tracking still
    /// works but is session-scoped.
    public var trackedMasternodesAreDurable: Bool {
        persistenceCapabilities.contains(
            PlatformWalletPersistenceCapabilities.trackedMasternodes)
    }

    /// Track the masternode `proTxHash` (32 WIRE-order bytes, as
    /// `MasternodeLocateMatch.proTxHash` carries it) independently of any
    /// wallet, with an optional label. Local — seeds the record from the
    /// current masternode list; call `refreshTrackedMasternode` afterwards
    /// for the Platform / registration details. Returns the new record
    /// (`source == .tracked`). Throws `.invalidParameter` when already
    /// tracked.
    @discardableResult
    public func trackMasternode(
        proTxHash: Data,
        label: String? = nil
    ) throws -> PlatformMasternode {
        try trackedEntryCall(proTxHash: proTxHash) { handle, hashPtr, out, count in
            if let label, !label.isEmpty {
                return label.withCString { cLabel in
                    platform_wallet_manager_track_masternode(handle, hashPtr, cLabel, out, count)
                }
            }
            return platform_wallet_manager_track_masternode(handle, hashPtr, nil, out, count)
        }
    }

    /// Stop tracking. Returns whether a row existed. Keys the app stored
    /// for this node live in ITS secure storage and are the app's to
    /// delete.
    @discardableResult
    public func untrackMasternode(proTxHash: Data) throws -> Bool {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        var removed = false
        let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_manager_untrack_masternode(
                handle, raw.baseAddress?.assumingMemoryBound(to: UInt8.self), &removed)
        }
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
        return removed
    }

    /// Rename a tracked masternode (`nil` / blank clears the label).
    public func setTrackedMasternodeLabel(proTxHash: Data, label: String?) throws {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            let hashPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
            if let label, !label.isEmpty {
                return label.withCString { cLabel in
                    platform_wallet_manager_set_tracked_masternode_label(handle, hashPtr, cLabel)
                }
            }
            return platform_wallet_manager_set_tracked_masternode_label(handle, hashPtr, nil)
        }
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
    }

    /// Every tracked masternode (`source == .tracked`), status resolved
    /// against the CURRENT masternode list, sorted by when they were
    /// tracked. Empty when nothing is tracked or the manager isn't
    /// configured. Same record shape as `masternodes(for:)`, so list UIs
    /// render both with one code path.
    public func trackedMasternodes() -> [PlatformMasternode] {
        guard isConfigured, handle != NULL_HANDLE else { return [] }
        var outEntries: UnsafePointer<MasternodeEntryV2FFI>?
        var outCount: UInt = 0
        let ffiResult = platform_wallet_manager_list_tracked_masternodes(
            handle, &outEntries, &outCount)
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            recordLastError(PlatformWalletError(result: result))
            return []
        }
        guard let entries = outEntries, outCount > 0 else { return [] }
        defer {
            platform_wallet_manager_free_masternodes_v2(
                UnsafeMutablePointer(mutating: entries), outCount)
        }
        return Self.masternodeModels(from: entries, count: Int(outCount))
    }

    /// Refresh everything the wallet layer can learn about a tracked
    /// masternode — its list entry, its Platform owner / operator
    /// identities (owner + payout key hashes, claimable balance) and, once,
    /// its ProRegTx (registration height, collateral). Network; runs on a
    /// detached task. Partial results are kept even when a step fails (the
    /// error is still thrown).
    @discardableResult
    public func refreshTrackedMasternode(proTxHash: Data) async throws -> PlatformMasternode {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> PlatformMasternode in
            var outEntries: UnsafePointer<MasternodeEntryV2FFI>?
            var outCount: UInt = 0
            let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                platform_wallet_manager_refresh_tracked_masternode(
                    handle,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    &outEntries,
                    &outCount)
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            guard let entries = outEntries, outCount > 0 else {
                throw PlatformWalletError.notFound("refresh returned no record")
            }
            defer {
                platform_wallet_manager_free_masternodes_v2(
                    UnsafeMutablePointer(mutating: entries), outCount)
            }
            guard let record = Self.masternodeModels(from: entries, count: Int(outCount)).first
            else {
                throw PlatformWalletError.notFound("refresh returned no record")
            }
            return record
        }.value
    }

    /// Withdraw from a TRACKED masternode's owner identity with a
    /// host-supplied key. `role` is `.owner` (pays the registered payout
    /// address; `destinationAddress` must be nil) or `.ownerPayout`
    /// (destination optional, defaults to the payout address itself).
    /// `key` is the private key text as the user holds it — WIF or 64-char
    /// hex; it is passed through for this one signing call and never
    /// retained. Returns the identity's new balance in credits.
    ///
    /// `.masternodeWithdrawalUnconfirmed` carries the same do-NOT-retry
    /// contract as the wallet-scoped withdraw: re-read the claimable
    /// balance before anything else.
    public func trackedMasternodeWithdraw(
        proTxHash: Data,
        amountCredits: UInt64,
        role: MasternodeKeyRole,
        key: String,
        destinationAddress: String? = nil
    ) async throws -> UInt64 {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> UInt64 in
            var newBalance: UInt64 = 0
            let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
                let hashPtr = raw.baseAddress?.assumingMemoryBound(to: UInt8.self)
                return key.withCString { cKey in
                    if let destination = destinationAddress {
                        return destination.withCString { cDest in
                            platform_wallet_manager_tracked_masternode_withdraw(
                                handle, hashPtr, amountCredits, role.rawValue, cKey, cDest,
                                &newBalance)
                        }
                    }
                    return platform_wallet_manager_tracked_masternode_withdraw(
                        handle, hashPtr, amountCredits, role.rawValue, cKey, nil, &newBalance)
                }
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            return newBalance
        }.value
    }

    // MARK: - Shared marshalling

    /// One-entry-call helper for FFI functions returning a masternode
    /// entry array.
    private func trackedEntryCall(
        proTxHash: Data,
        _ call: (
            Handle,
            UnsafePointer<UInt8>?,
            UnsafeMutablePointer<UnsafePointer<MasternodeEntryV2FFI>?>,
            UnsafeMutablePointer<UInt>
        ) -> PlatformWalletFFIResult
    ) throws -> PlatformMasternode {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        var outEntries: UnsafePointer<MasternodeEntryV2FFI>?
        var outCount: UInt = 0
        let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            call(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outEntries,
                &outCount)
        }
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
        guard let entries = outEntries, outCount > 0 else {
            throw PlatformWalletError.notFound("no record returned")
        }
        defer {
            platform_wallet_manager_free_masternodes_v2(
                UnsafeMutablePointer(mutating: entries), outCount)
        }
        guard let record = Self.masternodeModels(from: entries, count: Int(outCount)).first else {
            throw PlatformWalletError.notFound("no record returned")
        }
        return record
    }
}
