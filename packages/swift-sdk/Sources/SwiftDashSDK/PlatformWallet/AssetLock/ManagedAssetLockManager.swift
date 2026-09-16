import Foundation
import DashSDKFFI

/// Asset lock lifecycle manager for building, broadcasting, and tracking asset locks.
///
/// Obtained via `ManagedPlatformWallet.assetLockManager()`.
///
/// `@unchecked Sendable` is safe here: the wrapper holds a single
/// immutable `let handle: Handle` (an `Int64`, trivially Sendable) and
/// no other state. `deinit` calls `asset_lock_manager_destroy` exactly
/// once when the last retain drops — Swift's reference counting
/// guarantees no concurrent dispatch into `deinit`, so background
/// catch-up tasks can hold their own retain on this wrapper across
/// `Task` boundaries without a lifetime race.
///
/// `final` is required because subclasses could add mutable state and
/// invalidate the Sendable claim.
public final class ManagedAssetLockManager: @unchecked Sendable {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        asset_lock_manager_destroy(handle).discard()
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

    /// Status of a tracked asset lock. Mirrors the Rust-side
    /// `AssetLockStatus` enum in `rs-platform-wallet`; any new case
    /// added here must land in lockstep with the Rust variant so the
    /// FFI round-trip stays unambiguous.
    public enum AssetLockStatus: UInt32 {
        case built = 0
        case broadcast = 1
        case instantSendLocked = 2
        case chainLocked = 3
        /// Terminal state set by `consume_asset_lock` after a
        /// successful identity registration or top-up. The persisted
        /// row is retained for historical lookup (e.g. the
        /// Transactions list mapping a funding tx back to its
        /// locked amount); a Consumed lock cannot fund another
        /// identity.
        case consumed = 4
        /// Produced by restore reconstruction or live reconciliation of an
        /// unauthenticated already-consumed report. Core-side finality is
        /// known, but Platform-side consumption is UNKNOWN — the lock
        /// may have funded an identity long ago, or be unspent
        /// stranded value. Deliberately outside both the pending
        /// window (`broadcast`…`chainLocked`) and `consumed`: UIs must
        /// render neither "in flight" nor "done" for these rows. An
        /// explicit resume may consume one; Platform rejects an
        /// already-spent outpoint with a typed error.
        case recoveredFromChain = 5
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
        /// Credit-output derivation path (string form). The signer
        /// uses this path to produce the consume-phase signature.
        public let derivationPath: String
    }

    /// Result of creating a funded asset lock proof.
    public struct FundedProofResult {
        /// Bincode-encoded AssetLockProof.
        public let proofBytes: Data
        /// Credit-output derivation path (string form).
        public let derivationPath: String
        /// 32-byte transaction ID.
        public let txid: Data
    }

    /// Result of resuming an asset lock.
    public struct ResumeResult {
        /// Bincode-encoded AssetLockProof.
        public let proofBytes: Data
        /// Credit-output derivation path (string form).
        public let derivationPath: String
    }

    // MARK: - Queries

    /// List all tracked asset locks.
    public func listTrackedLocks() throws -> [TrackedAssetLock] {
        var locksPtr: UnsafeMutablePointer<TrackedAssetLockFFI>? = nil
        var count: UInt = 0
        try asset_lock_manager_list_tracked_locks(handle, &locksPtr, &count).check()
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

    /// Build an asset lock transaction. The caller supplies a
    /// `MnemonicResolver` whose vtable backs every per-input Core
    /// ECDSA signature; the credit-output private key is never
    /// exposed across the FFI. The resolver must outlive this call.
    public func buildTransaction(
        amountDuffs: UInt64,
        accountIndex: UInt32 = 0,
        fundingType: FundingType,
        identityIndex: UInt32 = 0,
        resolver: MnemonicResolver
    ) throws -> BuildResult {
        var txBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var txLen: UInt = 0
        var pathPtr: UnsafeMutablePointer<CChar>? = nil

        try withExtendedLifetime(resolver) {
            try asset_lock_manager_build_transaction(
                handle, amountDuffs, accountIndex, fundingType.rawValue, identityIndex,
                resolver.handle, &txBytesPtr, &txLen, &pathPtr
            ).check()
        }

        guard let txPtr = txBytesPtr, txLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but transaction buffer was empty")
        }
        defer { asset_lock_manager_free_tx_bytes(txPtr, txLen) }
        defer { if let p = pathPtr { platform_wallet_string_free(p) } }

        let txData = Data(bytes: txPtr, count: Int(txLen))
        // Fail fast on a NULL or empty derivation path. The FFI
        // contract is: on `Success` the path pointer is non-NULL and
        // points to a non-empty NUL-terminated C string. Defaulting
        // to `""` would surface as opaque "key not found" / "derive
        // failed" errors at the next signing step; throwing here
        // names the actual contract violation.
        guard let pathPtr, pathPtr.pointee != 0 else {
            throw PlatformWalletError.unknown(
                "FFI returned success but derivation path was NULL or empty"
            )
        }
        let path = String(cString: pathPtr)
        return BuildResult(transaction: txData, derivationPath: path)
    }

    /// Build, broadcast, and wait for an asset lock proof. Signing
    /// happens through the supplied `MnemonicResolver` — the
    /// credit-output private key never crosses the FFI boundary.
    public func createFundedProof(
        amountDuffs: UInt64,
        accountIndex: UInt32 = 0,
        fundingType: FundingType,
        identityIndex: UInt32 = 0,
        resolver: MnemonicResolver
    ) throws -> FundedProofResult {
        var proofBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var proofLen: UInt = 0
        var pathPtr: UnsafeMutablePointer<CChar>? = nil
        var txid: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)

        try withExtendedLifetime(resolver) {
            try asset_lock_manager_create_funded_proof(
                handle, amountDuffs, accountIndex, fundingType.rawValue, identityIndex,
                resolver.handle, &proofBytesPtr, &proofLen, &pathPtr, &txid
            ).check()
        }

        guard let proofPtr = proofBytesPtr, proofLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but proof buffer was empty")
        }
        defer { asset_lock_manager_free_proof_bytes(proofPtr, proofLen) }
        defer { if let p = pathPtr { platform_wallet_string_free(p) } }

        // Fail fast on a NULL or empty derivation path. The FFI
        // contract is: on `Success` the path pointer is non-NULL and
        // points to a non-empty NUL-terminated C string. Defaulting
        // to `""` would surface as opaque "key not found" / "derive
        // failed" errors at the next signing step; throwing here
        // names the actual contract violation.
        guard let pathPtr, pathPtr.pointee != 0 else {
            throw PlatformWalletError.unknown(
                "FFI returned success but derivation path was NULL or empty"
            )
        }
        let path = String(cString: pathPtr)
        return FundedProofResult(
            proofBytes: Data(bytes: proofPtr, count: Int(proofLen)),
            derivationPath: path,
            txid: withUnsafeBytes(of: &txid) { Data($0) }
        )
    }

    // MARK: - Resume

    /// Resume a tracked asset lock from whatever stage it's at.
    /// Resume only re-derives the proof + credit-output path; signing
    /// the consume transaction happens in the next FFI call, which
    /// is where the resolver plugs in. No signer parameter needed.
    public func resume(
        txid: Data,
        vout: UInt32 = 0,
        timeoutSeconds: UInt64 = 300
    ) throws -> ResumeResult {
        guard txid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "txid must be 32 bytes, got \(txid.count)"
            )
        }

        var proofBytesPtr: UnsafeMutablePointer<UInt8>? = nil
        var proofLen: UInt = 0
        var pathPtr: UnsafeMutablePointer<CChar>? = nil

        var txidTuple: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        txid.withUnsafeBytes { buf in
            withUnsafeMutableBytes(of: &txidTuple) { dst in
                dst.copyBytes(from: buf.prefix(32))
            }
        }

        try asset_lock_manager_resume(
            handle, &txidTuple, vout, timeoutSeconds,
            &proofBytesPtr, &proofLen, &pathPtr
        ).check()

        guard let proofPtr = proofBytesPtr, proofLen > 0 else {
            throw PlatformWalletError.unknown("FFI returned success but proof buffer was empty")
        }
        defer { asset_lock_manager_free_proof_bytes(proofPtr, proofLen) }
        defer { if let p = pathPtr { platform_wallet_string_free(p) } }

        // Fail fast on a NULL or empty derivation path. The FFI
        // contract is: on `Success` the path pointer is non-NULL and
        // points to a non-empty NUL-terminated C string. Defaulting
        // to `""` would surface as opaque "key not found" / "derive
        // failed" errors at the next signing step; throwing here
        // names the actual contract violation.
        guard let pathPtr, pathPtr.pointee != 0 else {
            throw PlatformWalletError.unknown(
                "FFI returned success but derivation path was NULL or empty"
            )
        }
        let path = String(cString: pathPtr)
        return ResumeResult(
            proofBytes: Data(bytes: proofPtr, count: Int(proofLen)),
            derivationPath: path
        )
    }

    /// Fire-and-forget catch-up for a tracked asset lock — the
    /// app-launch / app-foreground variant of [`resume`].
    ///
    /// Drives the same `resume_asset_lock` path as [`resume`] but
    /// discards the returned proof + derivation-path tuple. The proof
    /// reaches the UI via the `AssetLockChangeSet` that
    /// `advance_asset_lock_status` queues — `statusRaw = 3 +
    /// proofBytes` lands on the `PersistentAssetLock` row, the UI
    /// observes it through `@Query`, no Swift-side state to plumb.
    ///
    /// Blocking: the Rust `runtime().block_on(...)` parks the calling
    /// thread for up to `timeoutSeconds`. Callers MUST schedule this
    /// on a background queue / `Task.detached`.
    public func catchUpBlocking(
        txid: Data,
        vout: UInt32 = 0,
        timeoutSeconds: UInt64 = 300
    ) throws {
        guard txid.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "txid must be 32 bytes, got \(txid.count)"
            )
        }

        var txidTuple: FFIByteTuple32 =
            (0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0)
        txid.withUnsafeBytes { buf in
            withUnsafeMutableBytes(of: &txidTuple) { dst in
                dst.copyBytes(from: buf.prefix(32))
            }
        }

        try asset_lock_manager_catch_up_blocking(
            handle, &txidTuple, vout, timeoutSeconds
        ).check()
    }
}
