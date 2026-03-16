// NullifierSyncService.swift
// SwiftDashSDK
//
// High-level Swift API for privacy-preserving nullifier BLAST sync.
// Discovers which nullifiers have been spent in the shielded pool using
// trunk/branch chunk queries to hide the specific nullifiers being checked.

import Foundation
import DashSDKFFI

// MARK: - Nullifier Sync Service

@MainActor
extension SDK {

    /// Synchronize nullifier statuses using privacy-preserving BLAST sync.
    ///
    /// Discovers which of the supplied nullifiers have been spent in the shielded pool.
    /// Uses trunk/branch chunk queries for privacy (hides which specific nullifiers
    /// the wallet cares about).
    ///
    /// - Parameters:
    ///   - nullifiers: Array of 32-byte nullifier hashes to check.
    ///   - config: Sync configuration. Pass nil to use default values.
    ///   - lastSyncHeight: Height from previous sync (0 for full scan).
    ///   - lastSyncTimestamp: Timestamp from previous sync (0 for full scan).
    /// - Returns: `NullifierSyncResult` containing found/absent nullifiers and sync checkpoint.
    /// - Throws: `SDKError` on failure.
    public func syncNullifiers(
        nullifiers: [Data],
        config: NullifierSyncConfig? = nil,
        lastSyncHeight: UInt64 = 0,
        lastSyncTimestamp: UInt64 = 0
    ) async throws -> NullifierSyncResult {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !nullifiers.isEmpty else {
            throw SDKError.invalidParameter("Nullifiers array must not be empty")
        }

        // Validate all nullifiers are 32 bytes
        for (i, nf) in nullifiers.enumerated() {
            guard nf.count == 32 else {
                throw SDKError.invalidParameter(
                    "Nullifier at index \(i) must be 32 bytes, got \(nf.count)"
                )
            }
        }

        // Concatenate all nullifiers into a single contiguous byte array
        var concatenated = Data(capacity: nullifiers.count * 32)
        for nf in nullifiers {
            concatenated.append(nf)
        }

        let sdkPtr = NullifierSendableSdkPtr(sdkHandle)
        let count = UInt32(nullifiers.count)

        // Prepare config — use let so it can be captured by @Sendable closure
        let ffiConfig: FFINullifierSyncConfig? = if let cfg = config {
            try cfg.toFFI()
        } else {
            nil
        }
        let nullifierData = concatenated  // immutable copy for capture

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                // Use the _with_result variant for better error handling via DashSDKResult.
                let result: DashSDKResult

                if var cfg = ffiConfig {
                    result = nullifierData.withUnsafeBytes { bytesPtr -> DashSDKResult in
                        guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return withUnsafePointer(to: &cfg) { cfgPtr in
                            dash_sdk_sync_nullifiers_with_result(
                                UnsafePointer(sdkPtr.ptr),
                                base,
                                count,
                                cfgPtr,
                                lastSyncHeight,
                                lastSyncTimestamp
                            )
                        }
                    }
                } else {
                    result = nullifierData.withUnsafeBytes { bytesPtr -> DashSDKResult in
                        guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return dash_sdk_sync_nullifiers_with_result(
                            UnsafePointer(sdkPtr.ptr),
                            base,
                            count,
                            nil,
                            lastSyncHeight,
                            lastSyncTimestamp
                        )
                    }
                }

                // Check for error
                if let error = result.error {
                    let errorMessage = error.pointee.message != nil
                        ? String(cString: error.pointee.message!)
                        : "Unknown error"
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError(errorMessage))
                    return
                }

                guard let dataPtr = result.data else {
                    continuation.resume(throwing: SDKError.internalError("No sync result returned"))
                    return
                }

                // The data pointer is a FFINullifierSyncResult allocated by Box::into_raw.
                let ffiResultPtr = dataPtr.assumingMemoryBound(to: FFINullifierSyncResult.self)
                let ffiResult = ffiResultPtr.pointee

                // Copy data out into Swift types before freeing.
                let syncResult = NullifierSyncResult(ffi: ffiResult)

                // Free the FFI result (this also frees the found/absent arrays).
                dash_sdk_nullifier_sync_result_free(
                    UnsafeMutablePointer(mutating: ffiResultPtr)
                )

                continuation.resume(returning: syncResult)
            }
        }
    }

    /// Convenience: Synchronize nullifiers using the raw pointer API.
    /// Prefer `syncNullifiers(nullifiers:config:lastSyncHeight:lastSyncTimestamp:)` instead,
    /// which uses the DashSDKResult-based variant for better error handling.
    ///
    /// - Parameters:
    ///   - nullifiers: Array of 32-byte nullifier hashes.
    ///   - config: Sync configuration. Pass nil to use defaults.
    ///   - lastSyncHeight: Height from previous sync (0 for full scan).
    ///   - lastSyncTimestamp: Timestamp from previous sync (0 for full scan).
    /// - Returns: `NullifierSyncResult` or nil if the sync failed.
    public func syncNullifiersRaw(
        nullifiers: [Data],
        config: NullifierSyncConfig? = nil,
        lastSyncHeight: UInt64 = 0,
        lastSyncTimestamp: UInt64 = 0
    ) async throws -> NullifierSyncResult {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !nullifiers.isEmpty else {
            throw SDKError.invalidParameter("Nullifiers array must not be empty")
        }

        for (i, nf) in nullifiers.enumerated() {
            guard nf.count == 32 else {
                throw SDKError.invalidParameter(
                    "Nullifier at index \(i) must be 32 bytes, got \(nf.count)"
                )
            }
        }

        var concatenated = Data(capacity: nullifiers.count * 32)
        for nf in nullifiers {
            concatenated.append(nf)
        }

        let sdkPtr = NullifierSendableSdkPtr(sdkHandle)
        let count = UInt32(nullifiers.count)

        var ffiConfig: FFINullifierSyncConfig?
        if let cfg = config {
            ffiConfig = try cfg.toFFI()
        }

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let resultPtr: UnsafeMutablePointer<FFINullifierSyncResult>?

                if var cfg = ffiConfig {
                    resultPtr = concatenated.withUnsafeBytes { bytesPtr -> UnsafeMutablePointer<FFINullifierSyncResult>? in
                        guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return nil
                        }
                        return withUnsafePointer(to: &cfg) { cfgPtr in
                            dash_sdk_sync_nullifiers(
                                UnsafePointer(sdkPtr.ptr),
                                base,
                                count,
                                cfgPtr,
                                lastSyncHeight,
                                lastSyncTimestamp
                            )
                        }
                    }
                } else {
                    resultPtr = concatenated.withUnsafeBytes { bytesPtr -> UnsafeMutablePointer<FFINullifierSyncResult>? in
                        guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return nil
                        }
                        return dash_sdk_sync_nullifiers(
                            UnsafePointer(sdkPtr.ptr),
                            base,
                            count,
                            nil,
                            lastSyncHeight,
                            lastSyncTimestamp
                        )
                    }
                }

                guard let ffiResultPtr = resultPtr else {
                    continuation.resume(
                        throwing: SDKError.internalError("Nullifier sync returned null (check SDK logs for details)")
                    )
                    return
                }

                let ffiResult = ffiResultPtr.pointee
                let syncResult = NullifierSyncResult(ffi: ffiResult)
                dash_sdk_nullifier_sync_result_free(ffiResultPtr)

                continuation.resume(returning: syncResult)
            }
        }
    }
}

// MARK: - Private Sendable Wrapper

/// Sendable wrapper for the SDK handle pointer, for use in nullifier sync closures.
private final class NullifierSendableSdkPtr: @unchecked Sendable {
    let ptr: UnsafeMutablePointer<SDKHandle>
    init(_ p: UnsafeMutablePointer<SDKHandle>) { self.ptr = p }
}
