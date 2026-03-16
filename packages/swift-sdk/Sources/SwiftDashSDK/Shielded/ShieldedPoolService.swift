// ShieldedPoolService.swift
// SwiftDashSDK
//
// High-level Swift API for shielded pool (Orchard/ZK) operations.
// Provides query, transition, builder, and broadcast methods as extensions on SDK.

import Foundation
import DashSDKFFI

// MARK: - Shielded Pool Service

@MainActor
extension SDK {

    // MARK: - Internal Helpers

    /// Process a DashSDKResult and extract a string value.
    /// Mirrors the pattern from PlatformQueryExtensions.swift.
    private func shieldedProcessStringResult(_ result: DashSDKResult) throws -> String {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let string = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        return string
    }

    /// Process a DashSDKResult that returns no data (success/error only).
    private func shieldedProcessVoidResult(_ result: DashSDKResult) throws {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
    }


    // MARK: - Query Methods

    /// Fetch all anchor hashes from the shielded pool.
    ///
    /// - Returns: Array of 32-byte anchor hashes.
    /// - Throws: `SDKError` on failure.
    public func getShieldedAnchors() async throws -> [Data] {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_get_anchors(
                    UnsafePointer(sdkPtr.ptr)
                )

                do {
                    let jsonString = try self.shieldedExtractString(result)
                    guard let jsonData = jsonString.data(using: .utf8),
                          let hexArray = try? JSONSerialization.jsonObject(with: jsonData) as? [String]
                    else {
                        continuation.resume(throwing: SDKError.serializationError("Failed to parse anchors JSON"))
                        return
                    }

                    let anchors: [Data] = hexArray.compactMap { hexToData($0) }
                    continuation.resume(returning: anchors)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Fetch the most recent anchor from the shielded pool.
    ///
    /// - Returns: 32-byte anchor hash.
    /// - Throws: `SDKError` on failure.
    public func getMostRecentAnchor() async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_get_most_recent_anchor(
                    UnsafePointer(sdkPtr.ptr)
                )

                do {
                    let hexString = try self.shieldedExtractString(result)
                    guard let data = hexToData(hexString) else {
                        continuation.resume(throwing: SDKError.serializationError("Invalid hex anchor"))
                        return
                    }
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Check nullifier statuses against the shielded pool.
    ///
    /// - Parameter nullifiers: Array of 32-byte nullifier hashes.
    /// - Returns: Array of `NullifierStatus` indicating spent/unspent for each.
    /// - Throws: `SDKError` on failure.
    public func checkNullifiers(_ nullifiers: [Data]) async throws -> [NullifierStatus] {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !nullifiers.isEmpty else {
            return []
        }

        // Validate all nullifiers are 32 bytes
        for (i, nf) in nullifiers.enumerated() {
            guard nf.count == 32 else {
                throw SDKError.invalidParameter("Nullifier at index \(i) must be 32 bytes, got \(nf.count)")
            }
        }

        // Concatenate all nullifiers into a single contiguous byte array
        var concatenated = Data(capacity: nullifiers.count * 32)
        for nf in nullifiers {
            concatenated.append(nf)
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        let count = UInt32(nullifiers.count)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = concatenated.withUnsafeBytes { bytesPtr -> DashSDKResult in
                    guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return dash_sdk_shielded_get_nullifiers(
                        UnsafePointer(sdkPtr.ptr),
                        base,
                        count
                    )
                }

                do {
                    let jsonString = try self.shieldedExtractString(result)
                    guard let jsonData = jsonString.data(using: .utf8),
                          let jsonArray = try? JSONSerialization.jsonObject(with: jsonData) as? [[String: Any]]
                    else {
                        continuation.resume(throwing: SDKError.serializationError("Failed to parse nullifier statuses"))
                        return
                    }

                    let statuses: [NullifierStatus] = jsonArray.compactMap { obj in
                        guard let hexStr = obj["nullifier"] as? String,
                              let isSpent = obj["isSpent"] as? Bool,
                              let data = hexToData(hexStr)
                        else { return nil }
                        return NullifierStatus(nullifier: data, isSpent: isSpent)
                    }
                    continuation.resume(returning: statuses)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Fetch encrypted notes from the shielded pool.
    ///
    /// - Parameters:
    ///   - startIndex: Starting index (0-based) in the encrypted notes tree.
    ///   - count: Maximum number of notes to return.
    /// - Returns: Array of `EncryptedNote`.
    /// - Throws: `SDKError` on failure.
    public func getEncryptedNotes(startIndex: UInt64, count: UInt32) async throws -> [EncryptedNote] {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_get_encrypted_notes(
                    UnsafePointer(sdkPtr.ptr),
                    startIndex,
                    count
                )

                do {
                    let jsonString = try self.shieldedExtractString(result)
                    guard let jsonData = jsonString.data(using: .utf8),
                          let jsonArray = try? JSONSerialization.jsonObject(with: jsonData) as? [[String: Any]]
                    else {
                        continuation.resume(throwing: SDKError.serializationError("Failed to parse encrypted notes"))
                        return
                    }

                    let notes: [EncryptedNote] = jsonArray.compactMap { obj in
                        guard let cmxHex = obj["cmx"] as? String,
                              let nfHex = obj["nullifier"] as? String,
                              let encHex = obj["encryptedNote"] as? String,
                              let cmx = hexToData(cmxHex),
                              let nf = hexToData(nfHex),
                              let enc = hexToData(encHex)
                        else { return nil }
                        return EncryptedNote(cmx: cmx, nullifier: nf, encryptedNote: enc)
                    }
                    continuation.resume(returning: notes)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Fetch the total shielded pool balance.
    ///
    /// - Returns: Total pool balance in credits.
    /// - Throws: `SDKError` on failure.
    public func getShieldedPoolBalance() async throws -> UInt64 {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = dash_sdk_shielded_get_pool_state(
                    UnsafePointer(sdkPtr.ptr)
                )

                do {
                    let balanceStr = try self.shieldedExtractString(result)
                    guard let balance = UInt64(balanceStr) else {
                        continuation.resume(throwing: SDKError.serializationError("Failed to parse pool balance"))
                        return
                    }
                    continuation.resume(returning: balance)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Transition Methods (Build + Broadcast Combined)

    /// Shield funds from platform addresses into the shielded pool.
    ///
    /// - Parameters:
    ///   - inputs: Array of shield inputs (address + amount + private key).
    ///   - bundle: Orchard bundle parameters.
    ///   - amount: Total amount being shielded.
    ///   - feeFromInputIndex: Which input index to deduct fees from (0-based).
    /// - Throws: `SDKError` on failure.
    public func shieldFunds(
        inputs: [ShieldFundsInput],
        bundle: OrchardBundle,
        amount: UInt64,
        feeFromInputIndex: UInt16
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        let inputCount = UInt32(inputs.count)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = self.withFFIBundle(bundle) { bundlePtr in
                    self.withFFIShieldInputs(inputs) { inputsPtr in
                        dash_sdk_shielded_shield_funds(
                            UnsafePointer(sdkPtr.ptr),
                            inputsPtr,
                            inputCount,
                            bundlePtr,
                            amount,
                            feeFromInputIndex
                        )
                    }
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Transfer funds within the shielded pool (shielded-to-shielded).
    ///
    /// - Parameters:
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing out of the shielded pool.
    /// - Throws: `SDKError` on failure.
    public func shieldedTransfer(
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = self.withFFIBundle(bundle) { bundlePtr in
                    dash_sdk_shielded_transfer(
                        UnsafePointer(sdkPtr.ptr),
                        bundlePtr,
                        valueBalance
                    )
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Unshield funds from the shielded pool to a platform address.
    ///
    /// - Parameters:
    ///   - outputAddress: Platform address bytes.
    ///   - amount: Amount to unshield (in credits).
    ///   - bundle: Orchard bundle parameters.
    /// - Throws: `SDKError` on failure.
    public func unshieldFunds(
        outputAddress: Data,
        amount: UInt64,
        bundle: OrchardBundle
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !outputAddress.isEmpty else {
            throw SDKError.invalidParameter("Output address must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = outputAddress.withUnsafeBytes { addrPtr -> DashSDKResult in
                    guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        dash_sdk_shielded_unshield_funds(
                            UnsafePointer(sdkPtr.ptr),
                            addrBase,
                            outputAddress.count,
                            amount,
                            bundlePtr
                        )
                    }
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Shield funds from an L1 instant asset lock into the shielded pool.
    ///
    /// - Parameters:
    ///   - instantLock: Serialized instant lock bytes.
    ///   - transaction: Serialized funding transaction bytes.
    ///   - outputIndex: Output index in the transaction.
    ///   - privateKey: 32-byte ECDSA private key for the asset lock.
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing into the shielded pool.
    /// - Throws: `SDKError` on failure.
    public func shieldFromInstantLock(
        instantLock: Data,
        transaction: Data,
        outputIndex: UInt32,
        privateKey: Data,
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard privateKey.count == 32 else {
            throw SDKError.invalidParameter("Private key must be exactly 32 bytes")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = instantLock.withUnsafeBytes { ilPtr -> DashSDKResult in
                    guard let ilBase = ilPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return transaction.withUnsafeBytes { txPtr -> DashSDKResult in
                        guard let txBase = txPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return privateKey.withUnsafeBytes { pkPtr -> DashSDKResult in
                            guard let pkBase = pkPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                                return DashSDKResult()
                            }
                            return self.withFFIBundle(bundle) { bundlePtr in
                                dash_sdk_shielded_shield_from_instant_lock(
                                    UnsafePointer(sdkPtr.ptr),
                                    ilBase,
                                    instantLock.count,
                                    txBase,
                                    transaction.count,
                                    outputIndex,
                                    pkBase,
                                    bundlePtr,
                                    valueBalance
                                )
                            }
                        }
                    }
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Shield funds from an L1 chain asset lock into the shielded pool.
    ///
    /// - Parameters:
    ///   - coreChainLockedHeight: Core chain locked height for the asset lock.
    ///   - outPoint: 36-byte outpoint (32 txid + 4 index).
    ///   - privateKey: 32-byte ECDSA private key for the asset lock.
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing into the shielded pool.
    /// - Throws: `SDKError` on failure.
    public func shieldFromChainLock(
        coreChainLockedHeight: UInt32,
        outPoint: Data,
        privateKey: Data,
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard outPoint.count == 36 else {
            throw SDKError.invalidParameter("OutPoint must be exactly 36 bytes")
        }

        guard privateKey.count == 32 else {
            throw SDKError.invalidParameter("Private key must be exactly 32 bytes")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                var outPointTuple = dataToBytes36(outPoint)
                let result = privateKey.withUnsafeBytes { pkPtr -> DashSDKResult in
                    guard let pkBase = pkPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        withUnsafePointer(to: &outPointTuple) { opPtr in
                            dash_sdk_shielded_shield_from_chain_lock(
                                UnsafePointer(sdkPtr.ptr),
                                coreChainLockedHeight,
                                opPtr,
                                pkBase,
                                bundlePtr,
                                valueBalance
                            )
                        }
                    }
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Withdraw funds from the shielded pool to a Core (L1) address.
    ///
    /// - Parameters:
    ///   - amount: Amount to withdraw (in credits).
    ///   - bundle: Orchard bundle parameters.
    ///   - coreFeePerByte: Core fee per byte for the withdrawal transaction.
    ///   - pooling: Withdrawal pooling mode.
    ///   - outputScript: Raw Core output script bytes.
    /// - Throws: `SDKError` on failure.
    public func shieldedWithdraw(
        amount: UInt64,
        bundle: OrchardBundle,
        coreFeePerByte: UInt32,
        pooling: WithdrawalPooling,
        outputScript: Data
    ) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !outputScript.isEmpty else {
            throw SDKError.invalidParameter("Output script must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = outputScript.withUnsafeBytes { scriptPtr -> DashSDKResult in
                    guard let scriptBase = scriptPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        dash_sdk_shielded_withdraw(
                            UnsafePointer(sdkPtr.ptr),
                            amount,
                            bundlePtr,
                            coreFeePerByte,
                            pooling.rawValue,
                            scriptBase,
                            outputScript.count
                        )
                    }
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Builder Methods (Return Serialized State Transition Bytes)

    /// Build a shielded transfer state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing out of the shielded pool.
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildShieldedTransfer(
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = self.withFFIBundle(bundle) { bundlePtr in
                    dash_sdk_shielded_build_transfer(
                        UnsafePointer(sdkPtr.ptr),
                        bundlePtr,
                        valueBalance
                    )
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build an unshield state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - outputAddress: Platform address bytes.
    ///   - amount: Amount to unshield (in credits).
    ///   - bundle: Orchard bundle parameters.
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildUnshield(
        outputAddress: Data,
        amount: UInt64,
        bundle: OrchardBundle
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !outputAddress.isEmpty else {
            throw SDKError.invalidParameter("Output address must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = outputAddress.withUnsafeBytes { addrPtr -> DashSDKResult in
                    guard let addrBase = addrPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        dash_sdk_shielded_build_unshield(
                            UnsafePointer(sdkPtr.ptr),
                            addrBase,
                            outputAddress.count,
                            amount,
                            bundlePtr
                        )
                    }
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a shield state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - inputs: Array of shield inputs.
    ///   - bundle: Orchard bundle parameters.
    ///   - amount: Total amount being shielded.
    ///   - feeFromInputIndex: Which input index to deduct fees from (0-based).
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildShield(
        inputs: [ShieldFundsInput],
        bundle: OrchardBundle,
        amount: UInt64,
        feeFromInputIndex: UInt16
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)
        let inputCount = UInt32(inputs.count)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = self.withFFIBundle(bundle) { bundlePtr in
                    self.withFFIShieldInputs(inputs) { inputsPtr in
                        dash_sdk_shielded_build_shield(
                            UnsafePointer(sdkPtr.ptr),
                            inputsPtr,
                            inputCount,
                            bundlePtr,
                            amount,
                            feeFromInputIndex
                        )
                    }
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a shield-from-instant-lock state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - instantLock: Serialized instant lock bytes.
    ///   - transaction: Serialized funding transaction bytes.
    ///   - outputIndex: Output index in the transaction.
    ///   - privateKey: 32-byte ECDSA private key.
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing into the shielded pool.
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildShieldFromInstantLock(
        instantLock: Data,
        transaction: Data,
        outputIndex: UInt32,
        privateKey: Data,
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard privateKey.count == 32 else {
            throw SDKError.invalidParameter("Private key must be exactly 32 bytes")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = instantLock.withUnsafeBytes { ilPtr -> DashSDKResult in
                    guard let ilBase = ilPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return transaction.withUnsafeBytes { txPtr -> DashSDKResult in
                        guard let txBase = txPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                            return DashSDKResult()
                        }
                        return privateKey.withUnsafeBytes { pkPtr -> DashSDKResult in
                            guard let pkBase = pkPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                                return DashSDKResult()
                            }
                            return self.withFFIBundle(bundle) { bundlePtr in
                                dash_sdk_shielded_build_shield_from_instant_lock(
                                    UnsafePointer(sdkPtr.ptr),
                                    ilBase,
                                    instantLock.count,
                                    txBase,
                                    transaction.count,
                                    outputIndex,
                                    pkBase,
                                    bundlePtr,
                                    valueBalance
                                )
                            }
                        }
                    }
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a shield-from-chain-lock state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - coreChainLockedHeight: Core chain locked height.
    ///   - outPoint: 36-byte outpoint (32 txid + 4 index).
    ///   - privateKey: 32-byte ECDSA private key.
    ///   - bundle: Orchard bundle parameters.
    ///   - valueBalance: Net value flowing into the shielded pool.
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildShieldFromChainLock(
        coreChainLockedHeight: UInt32,
        outPoint: Data,
        privateKey: Data,
        bundle: OrchardBundle,
        valueBalance: UInt64
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard outPoint.count == 36 else {
            throw SDKError.invalidParameter("OutPoint must be exactly 36 bytes")
        }

        guard privateKey.count == 32 else {
            throw SDKError.invalidParameter("Private key must be exactly 32 bytes")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                var outPointTuple = dataToBytes36(outPoint)
                let result = privateKey.withUnsafeBytes { pkPtr -> DashSDKResult in
                    guard let pkBase = pkPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        withUnsafePointer(to: &outPointTuple) { opPtr in
                            dash_sdk_shielded_build_shield_from_chain_lock(
                                UnsafePointer(sdkPtr.ptr),
                                coreChainLockedHeight,
                                opPtr,
                                pkBase,
                                bundlePtr,
                                valueBalance
                            )
                        }
                    }
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a shielded withdrawal state transition without broadcasting.
    ///
    /// - Parameters:
    ///   - amount: Amount to withdraw (in credits).
    ///   - bundle: Orchard bundle parameters.
    ///   - coreFeePerByte: Core fee per byte.
    ///   - pooling: Withdrawal pooling mode.
    ///   - outputScript: Raw Core output script bytes.
    /// - Returns: Serialized state transition bytes.
    /// - Throws: `SDKError` on failure.
    public func buildShieldedWithdrawal(
        amount: UInt64,
        bundle: OrchardBundle,
        coreFeePerByte: UInt32,
        pooling: WithdrawalPooling,
        outputScript: Data
    ) async throws -> Data {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !outputScript.isEmpty else {
            throw SDKError.invalidParameter("Output script must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async {
                let result = outputScript.withUnsafeBytes { scriptPtr -> DashSDKResult in
                    guard let scriptBase = scriptPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return self.withFFIBundle(bundle) { bundlePtr in
                        dash_sdk_shielded_build_withdrawal(
                            UnsafePointer(sdkPtr.ptr),
                            amount,
                            bundlePtr,
                            coreFeePerByte,
                            pooling.rawValue,
                            scriptBase,
                            outputScript.count
                        )
                    }
                }

                do {
                    let data = try self.shieldedExtractHexData(result)
                    continuation.resume(returning: data)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Broadcast Method

    /// Broadcast a pre-built, serialized state transition.
    ///
    /// Use with the `build*` methods: first build a transition, inspect or store it,
    /// then broadcast when ready.
    ///
    /// - Parameter transitionBytes: Serialized state transition bytes (from a `build*` method).
    /// - Throws: `SDKError` on failure.
    public func broadcastShieldedTransition(_ transitionBytes: Data) async throws {
        guard let sdkHandle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !transitionBytes.isEmpty else {
            throw SDKError.invalidParameter("State transition bytes must not be empty")
        }

        let sdkPtr = SendableSdkPtr(sdkHandle)

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async {
                let result = transitionBytes.withUnsafeBytes { bytesPtr -> DashSDKResult in
                    guard let base = bytesPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                        return DashSDKResult()
                    }
                    return dash_sdk_shielded_broadcast(
                        UnsafePointer(sdkPtr.ptr),
                        base,
                        transitionBytes.count
                    )
                }

                do {
                    try self.shieldedExtractVoid(result)
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
}

// MARK: - Private Helpers

extension SDK {

    // Sendable wrapper for SDK handle pointer, used to cross @Sendable boundaries.
    final class SendableSdkPtr: @unchecked Sendable {
        let ptr: UnsafeMutablePointer<SDKHandle>
        init(_ p: UnsafeMutablePointer<SDKHandle>) { self.ptr = p }
    }

    /// Extract a string from a DashSDKResult. Thread-safe (no @MainActor).
    nonisolated private func shieldedExtractString(_ result: DashSDKResult) throws -> String {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let string = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        return string
    }

    /// Extract void (success/error) from a DashSDKResult. Thread-safe.
    nonisolated private func shieldedExtractVoid(_ result: DashSDKResult) throws {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
    }

    /// Extract hex-encoded data from a DashSDKResult (for builder functions).
    nonisolated private func shieldedExtractHexData(_ result: DashSDKResult) throws -> Data {
        let hexString = try shieldedExtractString(result)
        guard let data = hexToData(hexString) else {
            throw SDKError.serializationError("Invalid hex data returned from builder")
        }
        return data
    }

    /// Execute a closure with a temporary FFIOrchardBundleParams, ensuring all backing
    /// memory (actions, proof, encrypted notes) stays alive for the call duration.
    nonisolated func withFFIBundle<R>(
        _ bundle: OrchardBundle,
        body: (UnsafePointer<FFIOrchardBundleParams>) -> R
    ) -> R {
        // Guard against stack overflow from recursive withUnsafeBytes nesting
        precondition(bundle.actions.count <= 2048, "Bundle has too many actions (\(bundle.actions.count) > 2048)")

        // Build FFI action structs. The encrypted note data must live long enough.
        var ffiActions: [FFISerializedAction] = []
        let encryptedNoteStorage = bundle.actions.map { $0.encryptedNote }

        for (i, action) in bundle.actions.enumerated() {
            var ffiAction = FFISerializedAction(
                nullifier: dataToBytes32(action.nullifier),
                rk: dataToBytes32(action.rk),
                cmx: dataToBytes32(action.cmx),
                encrypted_note: nil,
                encrypted_note_len: action.encryptedNote.count,
                cv_net: dataToBytes32(action.cvNet),
                spend_auth_sig: dataToBytes64(action.spendAuthSig)
            )
            // We will set encrypted_note pointer below when we have buffer pointers.
            _ = i // suppress unused warning
            ffiActions.append(ffiAction)
        }

        // Now call with all pointers valid.
        return bundle.proof.withUnsafeBytes { proofPtr in
            // We need stable pointers to each encrypted note. Use a nested approach.
            func recurse(
                actionIndex: Int,
                notePointers: [UnsafePointer<UInt8>?]
            ) -> R {
                if actionIndex >= encryptedNoteStorage.count {
                    // All encrypted note pointers are now available.
                    // Update ffiActions with the correct pointers.
                    for (j, notePtr) in notePointers.enumerated() {
                        ffiActions[j].encrypted_note = notePtr
                    }

                    // Build the params struct.
                    return ffiActions.withUnsafeBufferPointer { actionsBuffer in
                        var params = FFIOrchardBundleParams(
                            actions: actionsBuffer.baseAddress,
                            actions_count: UInt32(ffiActions.count),
                            anchor: dataToBytes32(bundle.anchor),
                            proof: proofPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                            proof_len: bundle.proof.count,
                            binding_signature: dataToBytes64(bundle.bindingSignature)
                        )
                        return withUnsafePointer(to: &params) { paramsPtr in
                            body(paramsPtr)
                        }
                    }
                } else {
                    let noteData = encryptedNoteStorage[actionIndex]
                    if noteData.isEmpty {
                        return recurse(
                            actionIndex: actionIndex + 1,
                            notePointers: notePointers + [nil]
                        )
                    } else {
                        return noteData.withUnsafeBytes { noteBytes in
                            let notePtr = noteBytes.baseAddress?.assumingMemoryBound(to: UInt8.self)
                            return recurse(
                                actionIndex: actionIndex + 1,
                                notePointers: notePointers + [notePtr]
                            )
                        }
                    }
                }
            }

            return recurse(actionIndex: 0, notePointers: [])
        }
    }

    /// Execute a closure with a temporary array of FFIShieldInput structs.
    /// All backing Data (address, privateKey) stays alive for the call duration.
    nonisolated func withFFIShieldInputs<R>(
        _ inputs: [ShieldFundsInput],
        body: (UnsafePointer<FFIShieldInput>) -> R
    ) -> R {
        // We need stable pointers to each address and private key.
        func recurse(
            index: Int,
            addrPtrs: [(UnsafePointer<UInt8>?, Int)],
            pkPtrs: [UnsafePointer<UInt8>?]
        ) -> R {
            if index >= inputs.count {
                // Build FFIShieldInput array
                var ffiInputs: [FFIShieldInput] = []
                for (i, input) in inputs.enumerated() {
                    let (addrPtr, addrLen) = addrPtrs[i]
                    ffiInputs.append(FFIShieldInput(
                        address: addrPtr,
                        address_len: addrLen,
                        amount: input.amount,
                        private_key: pkPtrs[i]
                    ))
                }
                return ffiInputs.withUnsafeBufferPointer { buf in
                    body(buf.baseAddress!)
                }
            } else {
                let input = inputs[index]
                return input.address.withUnsafeBytes { addrBytes in
                    let addrPtr = addrBytes.baseAddress?.assumingMemoryBound(to: UInt8.self)
                    return input.privateKey.withUnsafeBytes { pkBytes in
                        let pkPtr = pkBytes.baseAddress?.assumingMemoryBound(to: UInt8.self)
                        return recurse(
                            index: index + 1,
                            addrPtrs: addrPtrs + [(addrPtr, input.address.count)],
                            pkPtrs: pkPtrs + [pkPtr]
                        )
                    }
                }
            }
        }

        return recurse(index: 0, addrPtrs: [], pkPtrs: [])
    }
}

