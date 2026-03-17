// AddressSyncService.swift
// SwiftDashSDK
//
// High-level Swift API for privacy-preserving BLAST address sync.
// Discovers which of a wallet's platform addresses have balances using
// trunk/branch Merkle tree queries for privacy.
//
// Uses the batch FFI function (dash_sdk_sync_addresses_batch_with_result)
// which takes flat address arrays — no callbacks needed.

import Foundation
import DashSDKFFI

// MARK: - Address Sync Service

@MainActor
extension SDK {

    /// Synchronize address balances using privacy-preserving BLAST sync.
    ///
    /// Passes all addresses as flat arrays to the Rust side which runs the
    /// full trunk/branch tree scan and incremental catch-up internally.
    ///
    /// - Parameters:
    ///   - addresses: Array of (derivation index, address key bytes) tuples.
    ///   - gapLimit: HD wallet gap limit (default 20).
    ///   - knownBalances: Previously found addresses from the last sync result.
    ///     Pass these back so incremental-only mode can skip the tree scan.
    ///   - config: Sync configuration. Pass nil for defaults.
    ///   - lastSyncHeight: Height from the previous sync result (0 for first sync).
    ///   - lastSyncTimestamp: Timestamp from previous sync (0 for full scan).
    /// - Returns: AddressSyncResult with found/absent addresses and sync checkpoint.
    /// - Throws: SDKError on failure.
    public func syncAddressBalances(
        addresses: [(index: UInt32, key: Data)],
        gapLimit: UInt32 = 20,
        knownBalances: [FoundAddress] = [],
        config: AddressSyncConfig? = nil,
        lastSyncHeight: UInt64 = 0,
        lastSyncTimestamp: UInt64 = 0
    ) async throws -> AddressSyncResult {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !addresses.isEmpty else {
            throw SDKError.invalidParameter("No addresses provided for sync")
        }

        // All keys must be the same size
        let keySize = addresses[0].key.count
        guard keySize > 0 else {
            throw SDKError.invalidParameter("Address key size must be > 0")
        }
        for (i, addr) in addresses.enumerated() {
            guard addr.key.count == keySize else {
                throw SDKError.invalidParameter(
                    "Address at index \(i) has key size \(addr.key.count), expected \(keySize)"
                )
            }
        }

        let sdkPtr = AddressSyncSendableSdkPtr(sdkHandle)
        let ffiConfig: FFIAddressSyncConfig? = config?.toFFI()

        // Build flat arrays for addresses
        var concatenatedKeys = Data(capacity: addresses.count * keySize)
        for addr in addresses {
            concatenatedKeys.append(addr.key)
        }
        let indices: [UInt32] = addresses.map { $0.index }

        // Build flat arrays for known balances
        var tmpKbKeys = Data()
        var tmpKbIndices: [UInt32] = []
        var tmpKbNonces: [UInt32] = []
        var tmpKbAmounts: [UInt64] = []
        for found in knownBalances {
            guard found.key.count == keySize else { continue }
            tmpKbKeys.append(found.key)
            tmpKbIndices.append(found.index)
            tmpKbNonces.append(found.nonce)
            tmpKbAmounts.append(found.balance)
        }

        let count = UInt32(addresses.count)
        let keySizeU32 = UInt32(keySize)
        let kbCount = UInt32(tmpKbIndices.count)
        let keysData = concatenatedKeys
        let kbKeysData = tmpKbKeys
        let kbIndices = tmpKbIndices
        let kbNonces = tmpKbNonces
        let kbAmounts = tmpKbAmounts
        let syncHeight = lastSyncHeight
        let syncTimestamp = lastSyncTimestamp

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result: DashSDKResult = keysData.withUnsafeBytes { keysBuffer in
                    guard let keysBase = keysBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }

                    return indices.withUnsafeBufferPointer { indicesBuffer in
                        guard let indicesBase = indicesBuffer.baseAddress else {
                            return DashSDKResult()
                        }

                        // Known balance pointers (may be empty)
                        let callFFI = { (kbKeysPtr: UnsafePointer<UInt8>?, kbIndPtr: UnsafePointer<UInt32>?, kbNonPtr: UnsafePointer<UInt32>?, kbAmtPtr: UnsafePointer<UInt64>?) -> DashSDKResult in
                            if var cfg = ffiConfig {
                                return withUnsafePointer(to: &cfg) { cfgPtr in
                                    dash_sdk_sync_addresses_batch_with_result(
                                        UnsafePointer(sdkPtr.ptr),
                                        keysBase, indicesBase, count, keySizeU32, gapLimit,
                                        kbKeysPtr, kbIndPtr, kbNonPtr, kbAmtPtr, kbCount,
                                        cfgPtr, syncHeight, syncTimestamp
                                    )
                                }
                            } else {
                                return dash_sdk_sync_addresses_batch_with_result(
                                    UnsafePointer(sdkPtr.ptr),
                                    keysBase, indicesBase, count, keySizeU32, gapLimit,
                                    kbKeysPtr, kbIndPtr, kbNonPtr, kbAmtPtr, kbCount,
                                    nil, syncHeight, syncTimestamp
                                )
                            }
                        }

                        if kbCount > 0 {
                            return kbKeysData.withUnsafeBytes { kbKeysBuffer in
                                let kbKeysBase = kbKeysBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
                                return kbIndices.withUnsafeBufferPointer { kbIndBuf in
                                    return kbNonces.withUnsafeBufferPointer { kbNonBuf in
                                        return kbAmounts.withUnsafeBufferPointer { kbAmtBuf in
                                            callFFI(kbKeysBase, kbIndBuf.baseAddress, kbNonBuf.baseAddress, kbAmtBuf.baseAddress)
                                        }
                                    }
                                }
                            }
                        } else {
                            return callFFI(nil, nil, nil, nil)
                        }
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
                    continuation.resume(
                        throwing: SDKError.internalError("No address sync result returned")
                    )
                    return
                }

                let ffiResultPtr = dataPtr.assumingMemoryBound(to: FFIAddressSyncResult.self)
                let ffiResult = ffiResultPtr.pointee
                let syncResult = AddressSyncResult(ffi: ffiResult)
                dash_sdk_address_sync_result_free(UnsafeMutablePointer(mutating: ffiResultPtr))
                continuation.resume(returning: syncResult)
            }
        }
    }
}

// MARK: - Private Sendable Wrapper

private final class AddressSyncSendableSdkPtr: @unchecked Sendable {
    let ptr: UnsafeMutablePointer<SDKHandle>
    init(_ p: UnsafeMutablePointer<SDKHandle>) { self.ptr = p }
}
