import Foundation

/// Asset lock lifecycle manager for building, broadcasting, and tracking asset locks.
///
/// Obtained via `ManagedPlatformWallet.assetLockManager()`.
public class ManagedAssetLockManager {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = asset_lock_manager_destroy(handle, &error)
    }

    // MARK: - Types

    /// Funding type for asset lock transactions.
    public enum FundingType: UInt32 {
        case identityRegistration = 0
        case identityTopUp = 1
        case identityTopUpNotBound = 2
        case identityInvitation = 3
        case assetLockAddressTopUp = 4
        case assetLockShieldedAddressTopUp = 5
    }

    /// Status of a tracked asset lock.
    public enum AssetLockStatus: UInt32 {
        case built = 0
        case broadcast = 1
        case instantSendLocked = 2
        case chainLocked = 3
    }

    /// A tracked asset lock.
    public struct TrackedAssetLock {
        public let txid: Data
        public let vout: UInt32
        public let accountIndex: UInt32
        public let fundingType: FundingType
        public let identityIndex: UInt32
        public let amount: UInt64
        public let status: AssetLockStatus
        public let hasProof: Bool
    }

    /// Result of building an asset lock transaction.
    public struct BuildResult {
        /// Serialized signed transaction.
        public let transaction: Data
        /// 32-byte one-time private key.
        public let privateKey: Data
    }

    /// Result of creating a funded asset lock proof.
    public struct FundedProofResult {
        /// Bincode-encoded AssetLockProof.
        public let proofBytes: Data
        /// 32-byte one-time private key.
        public let privateKey: Data
        /// 32-byte transaction ID.
        public let txid: Data
    }

    /// Result of resuming an asset lock.
    public struct ResumeResult {
        /// Bincode-encoded AssetLockProof.
        public let proofBytes: Data
        /// 32-byte one-time private key.
        public let privateKey: Data
    }

    // MARK: - Queries

    /// List all tracked asset locks.
    public func listTrackedLocks() throws -> [TrackedAssetLock] {
        var locksPtr: UnsafeMutablePointer<TrackedAssetLockFFI>? = nil
        var count: UInt = 0
        var error = PlatformWalletFFIError()

        let result = asset_lock_manager_list_tracked_locks(handle, &locksPtr, &count, &error)
        guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
            throw PlatformWalletError(result: result, error: error)
        }
        defer { asset_lock_manager_free_tracked_locks(locksPtr, count) }

        guard let locks = locksPtr, count > 0 else { return [] }

        return (0..<Int(count)).map { i in
            let lock = locks[i]
            let txidData = withUnsafeBytes(of: lock.txid) { Data($0) }
            return TrackedAssetLock(
                txid: txidData,
                vout: lock.vout,
                accountIndex: lock.account_index,
                fundingType: FundingType(rawValue: lock.funding_type) ?? .identityRegistration,
                identityIndex: lock.identity_index,
                amount: lock.amount,
                status: AssetLockStatus(rawValue: lock.status) ?? .built,
                hasProof: lock.has_proof
            )
        }
    }

    // MARK: - Build

    /// Build an asset lock transaction.
    public func buildTransaction(
        amountDuffs: UInt64,
        accountIndex: UInt32 = 0,
        fundingType: FundingType,
        identityIndex: UInt32 = 0
    ) throws -> BuildResult {
        var txBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var txLen: UInt = 0
        var privateKey: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var error = PlatformWalletFFIError()

        let result = asset_lock_manager_build_transaction(
            handle, amountDuffs, accountIndex, fundingType.rawValue, identityIndex,
            &txBytesPtr, &txLen, &privateKey, &error
        )
        guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
            throw PlatformWalletError(result: result, error: error)
        }
        guard let txPtr = txBytesPtr, txLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but transaction buffer was empty")
        }
        defer { asset_lock_manager_free_tx_bytes(txPtr, txLen) }

        let txData = Data(bytes: txPtr, count: Int(txLen))
        let keyData = withUnsafeBytes(of: &privateKey) { Data($0) }
        return BuildResult(transaction: txData, privateKey: keyData)
    }

    /// Build, broadcast, and wait for an asset lock proof.
    public func createFundedProof(
        amountDuffs: UInt64,
        accountIndex: UInt32 = 0,
        fundingType: FundingType,
        identityIndex: UInt32 = 0
    ) throws -> FundedProofResult {
        var proofBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var proofLen: UInt = 0
        var privateKey: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var txid: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var error = PlatformWalletFFIError()

        let result = asset_lock_manager_create_funded_proof(
            handle, amountDuffs, accountIndex, fundingType.rawValue, identityIndex,
            &proofBytesPtr, &proofLen, &privateKey, &txid, &error
        )
        guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
            throw PlatformWalletError(result: result, error: error)
        }
        guard let proofPtr = proofBytesPtr, proofLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but proof buffer was empty")
        }
        defer { asset_lock_manager_free_proof_bytes(proofPtr, proofLen) }

        return FundedProofResult(
            proofBytes: Data(bytes: proofPtr, count: Int(proofLen)),
            privateKey: withUnsafeBytes(of: &privateKey) { Data($0) },
            txid: withUnsafeBytes(of: &txid) { Data($0) }
        )
    }

    // MARK: - Resume

    /// Resume a tracked asset lock from whatever stage it's at.
    public func resume(
        txid: Data,
        vout: UInt32 = 0,
        timeoutSeconds: UInt64 = 300
    ) throws -> ResumeResult {
        guard txid.count == 32 else {
            throw PlatformWalletError.invalidParameter
        }

        var proofBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var proofLen: UInt = 0
        var privateKey: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                         UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        var error = PlatformWalletFFIError()

        var txidTuple: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8) =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        txid.withUnsafeBytes { buf in
            withUnsafeMutableBytes(of: &txidTuple) { dst in
                dst.copyBytes(from: buf.prefix(32))
            }
        }

        let result = asset_lock_manager_resume(
            handle, &txidTuple, vout, timeoutSeconds,
            &proofBytesPtr, &proofLen, &privateKey, &error
        )
        guard result == PLATFORM_WALLET_FFI_RESULT_SUCCESS else {
            throw PlatformWalletError(result: result, error: error)
        }
        guard let proofPtr = proofBytesPtr, proofLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but proof buffer was empty")
        }
        defer { asset_lock_manager_free_proof_bytes(proofPtr, proofLen) }

        return ResumeResult(
            proofBytes: Data(bytes: proofPtr, count: Int(proofLen)),
            privateKey: withUnsafeBytes(of: &privateKey) { Data($0) }
        )
    }
}
