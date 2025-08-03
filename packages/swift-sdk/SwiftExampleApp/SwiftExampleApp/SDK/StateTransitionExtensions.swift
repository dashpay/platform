import Foundation
import SwiftDashSDK
import DashSDKFFI

// MARK: - State Transition Extensions

extension SDK {
    
    // MARK: - Identity Handle Management
    
    /// Convert a DPPIdentity to an identity handle
    /// The returned handle must be freed with dash_sdk_identity_destroy when done
    public func identityToHandle(_ identity: DPPIdentity) throws -> OpaquePointer {
        // Convert identity ID to 32-byte array
        let idBytes = identity.id // identity.id is already Data
        guard idBytes.count == 32 else {
            throw SDKError.invalidParameter("Identity ID must be 32 bytes")
        }
        
        // Convert public keys to C structs
        let publicKeyData = identity.publicKeys.values.compactMap { key -> DashSDKPublicKeyData? in
            let keyData = key.data
            
            // Map Swift enums to C values
            let purpose: UInt8 = {
                switch key.purpose {
                case .authentication: return 0
                case .encryption: return 1
                case .decryption: return 2
                case .transfer: return 3
                case .system: return 4
                case .voting: return 5
                case .owner: return 6
                }
            }()
            
            let securityLevel: UInt8 = {
                switch key.securityLevel {
                case .master: return 0
                case .critical: return 1
                case .high: return 2
                case .medium: return 3
                }
            }()
            
            let keyType: UInt8 = {
                switch key.keyType {
                case .ecdsaSecp256k1: return 0
                case .bls12_381: return 1
                case .ecdsaHash160: return 2
                case .bip13ScriptHash: return 3
                case .eddsa25519Hash160: return 4
                }
            }()
            
            return DashSDKPublicKeyData(
                id: UInt8(key.id),
                purpose: purpose,
                security_level: securityLevel,
                key_type: keyType,
                read_only: key.readOnly,
                data: keyData.withUnsafeBytes { $0.baseAddress?.assumingMemoryBound(to: UInt8.self) } ?? nil,
                data_len: UInt(keyData.count),
                disabled_at: key.disabledAt ?? 0
            )
        }
        
        // Call the FFI function
        let result = idBytes.withUnsafeBytes { idPtr in
            publicKeyData.withUnsafeBufferPointer { keysPtr in
                dash_sdk_identity_create_from_components(
                    idPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keysPtr.baseAddress,
                    UInt(keysPtr.count),
                    identity.balance,
                    UInt64(identity.revision)
                )
            }
        }
        
        if let error = result.error {
            let errorString = String(cString: error.pointee.message)
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorString)
        }
        
        guard let handle = result.data else {
            throw SDKError.internalError("No identity handle returned")
        }
        
        return OpaquePointer(handle)!
    }
    
    // MARK: - Identity State Transitions
    
    /// Create a new identity (returns a dictionary for now)
    public func identityCreate() async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                let result = dash_sdk_identity_create(handle)
                
                if result.error == nil {
                    if result.data_type.rawValue == 3, // ResultIdentityHandle
                       let identityHandle = result.data {
                        // Get identity info from the handle
                        let infoPtr = dash_sdk_identity_get_info(OpaquePointer(identityHandle)!)
                        
                        if let info = infoPtr {
                            // Convert the C struct to a Swift dictionary
                            let idString = String(cString: info.pointee.id)
                            let balance = info.pointee.balance
                            let revision = info.pointee.revision
                            let publicKeysCount = info.pointee.public_keys_count
                            
                            let identityDict: [String: Any] = [
                                "id": idString,
                                "balance": balance,
                                "revision": revision,
                                "publicKeysCount": publicKeysCount
                            ]
                            
                            // Free the identity info structure
                            dash_sdk_identity_info_free(info)
                            
                            // Destroy the identity handle
                            dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                            
                            continuation.resume(returning: identityDict)
                        } else {
                            // Destroy the identity handle
                            dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                            continuation.resume(throwing: SDKError.internalError("Failed to get identity info"))
                        }
                    } else {
                        continuation.resume(throwing: SDKError.internalError("Invalid result type"))
                    }
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                }
            }
        }
    }
    
    /// Top up an identity with instant lock
    public func identityTopUp(
        identity: OpaquePointer,
        instantLock: Data,
        transaction: Data,
        outputIndex: UInt32,
        privateKey: Data
    ) async throws -> UInt64 {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                guard privateKey.count == 32 else {
                    continuation.resume(throwing: SDKError.invalidParameter("Private key must be 32 bytes"))
                    return
                }
                
                let result = instantLock.withUnsafeBytes { instantLockBytes in
                    transaction.withUnsafeBytes { txBytes in
                        privateKey.withUnsafeBytes { keyBytes in
                            dash_sdk_identity_topup_with_instant_lock(
                                handle,
                                identity,
                                instantLockBytes.bindMemory(to: UInt8.self).baseAddress!,
                                UInt(instantLock.count),
                                txBytes.bindMemory(to: UInt8.self).baseAddress!,
                                UInt(transaction.count),
                                outputIndex,
                                keyBytes.bindMemory(to: UInt8.self).baseAddress!.withMemoryRebound(to: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8).self, capacity: 1) { $0 },
                                nil // Default put settings
                            )
                        }
                    }
                }
                
                
                if result.error == nil {
                    if result.data_type.rawValue == 3, // ResultIdentityHandle
                       let toppedUpIdentityHandle = result.data {
                        // Get identity info from the handle to retrieve the new balance
                        let infoPtr = dash_sdk_identity_get_info(OpaquePointer(toppedUpIdentityHandle)!)
                        
                        if let info = infoPtr {
                            let balance = info.pointee.balance
                            
                            // Free the identity info structure
                            dash_sdk_identity_info_free(info)
                            
                            // Destroy the topped up identity handle
                            dash_sdk_identity_destroy(OpaquePointer(toppedUpIdentityHandle)!)
                            
                            continuation.resume(returning: balance)
                        } else {
                            // Destroy the identity handle
                            dash_sdk_identity_destroy(OpaquePointer(toppedUpIdentityHandle)!)
                            continuation.resume(throwing: SDKError.internalError("Failed to get identity info after topup"))
                        }
                    } else {
                        continuation.resume(throwing: SDKError.internalError("Invalid result type"))
                    }
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                }
            }
        }
    }
    
    /// Transfer credits between identities
    public func identityTransferCredits(
        fromIdentity: OpaquePointer,
        toIdentityId: String,
        amount: UInt64,
        publicKey: OpaquePointer? = nil,
        signer: OpaquePointer
    ) async throws -> (senderBalance: UInt64, receiverBalance: UInt64) {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Transfer credits
                let result = toIdentityId.withCString { toIdCStr in
                    dash_sdk_identity_transfer_credits(
                        handle,
                        fromIdentity,
                        toIdCStr,
                        amount,
                        publicKey,
                        signer,
                        nil  // Default put settings
                    )
                }
                
                if result.error == nil {
                    if let transferResultPtr = result.data {
                        let transferResult = transferResultPtr.assumingMemoryBound(to: DashSDKTransferCreditsResult.self).pointee
                        let senderBalance = transferResult.sender_balance
                        let receiverBalance = transferResult.receiver_balance
                        
                        // Free the transfer result
                        dash_sdk_transfer_credits_result_free(transferResultPtr.assumingMemoryBound(to: DashSDKTransferCreditsResult.self))
                        
                        continuation.resume(returning: (senderBalance, receiverBalance))
                    } else {
                        continuation.resume(throwing: SDKError.internalError("No data returned"))
                    }
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                }
            }
        }
    }
    
    /// Withdraw credits from identity
    public func identityWithdraw(
        identity: OpaquePointer,
        amount: UInt64,
        toAddress: String,
        coreFeePerByte: UInt32 = 0,
        publicKey: OpaquePointer? = nil,
        signer: OpaquePointer
    ) async throws -> UInt64 {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Withdraw credits
                let result = toAddress.withCString { addressCStr in
                    dash_sdk_identity_withdraw(
                        handle,
                        identity,
                        addressCStr,
                        amount,
                        coreFeePerByte,
                        publicKey,
                        signer,
                        nil  // Default put settings
                    )
                }
                
                
                if result.error == nil {
                    if let dataPtr = result.data {
                        // The result is a string containing the new balance
                        let balanceString = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
                        // Free the C string
                        dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
                        
                        if let newBalance = UInt64(balanceString) {
                            continuation.resume(returning: newBalance)
                        } else {
                            continuation.resume(throwing: SDKError.serializationError("Failed to parse balance"))
                        }
                    } else {
                        continuation.resume(throwing: SDKError.internalError("No data returned"))
                    }
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                }
            }
        }
    }
    
    // MARK: - Document State Transitions
    
    /// Create a new document
    public func documentCreate(
        contractId: String,
        documentType: String,
        ownerIdentityId: String,
        properties: [String: Any]
    ) async throws -> [String: Any] {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Document creation not yet implemented")
    }
    
    /// Replace an existing document
    public func documentReplace(
        documentId: String,
        properties: [String: Any]
    ) async throws -> [String: Any] {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Document replace not yet implemented")
    }
    
    /// Delete a document
    public func documentDelete(
        documentId: String
    ) async throws {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Document delete not yet implemented")
    }
    
    // MARK: - Token State Transitions
    
    /// Transfer tokens between identities
    public func tokenTransfer(
        tokenId: String,
        fromIdentityId: String,
        toIdentityId: String,
        amount: UInt64
    ) async throws -> (senderBalance: UInt64, receiverBalance: UInt64) {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Token transfer not yet implemented")
    }
    
    /// Mint new tokens
    public func tokenMint(
        tokenId: String,
        amount: UInt64,
        recipientId: String
    ) async throws -> UInt64 {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Token mint not yet implemented")
    }
    
    /// Burn tokens
    public func tokenBurn(
        tokenId: String,
        amount: UInt64,
        ownerIdentityId: String
    ) async throws -> UInt64 {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Token burn not yet implemented")
    }
    
    // MARK: - Data Contract State Transitions
    
    /// Create a new data contract
    public func dataContractCreate(
        schema: [String: Any],
        ownerId: String
    ) async throws -> [String: Any] {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Data contract create not yet implemented")
    }
    
    /// Update an existing data contract
    public func dataContractUpdate(
        contractId: String,
        schema: [String: Any]
    ) async throws -> [String: Any] {
        // TODO: Implement when FFI binding is available
        throw SDKError.notImplemented("Data contract update not yet implemented")
    }
}

// MARK: - Helper Types

// For now, we'll use the existing SDK types and create type aliases when needed

// MARK: - Convenience Methods with DPPIdentity

extension SDK {
    /// Transfer credits between identities (convenience method with DPPIdentity)
    public func transferCredits(
        from identity: DPPIdentity,
        toIdentityId: String,
        amount: UInt64,
        signer: OpaquePointer
    ) async throws -> (senderBalance: UInt64, receiverBalance: UInt64) {
        // Convert DPPIdentity to handle
        let identityHandle = try identityToHandle(identity)
        defer {
            // Clean up the handle when done
            dash_sdk_identity_destroy(identityHandle)
        }
        
        // Call the lower-level method
        return try await identityTransferCredits(
            fromIdentity: identityHandle,
            toIdentityId: toIdentityId,
            amount: amount,
            publicKey: nil, // Auto-select key
            signer: signer
        )
    }
    
    /// Top up identity with instant lock (convenience method with DPPIdentity)
    public func topUpIdentity(
        _ identity: DPPIdentity,
        instantLock: Data,
        transaction: Data,
        outputIndex: UInt32,
        privateKey: Data
    ) async throws -> UInt64 {
        // Convert DPPIdentity to handle
        let identityHandle = try identityToHandle(identity)
        defer {
            // Clean up the handle when done
            dash_sdk_identity_destroy(identityHandle)
        }
        
        // Call the lower-level method
        return try await identityTopUp(
            identity: identityHandle,
            instantLock: instantLock,
            transaction: transaction,
            outputIndex: outputIndex,
            privateKey: privateKey
        )
    }
    
    /// Withdraw credits from identity (convenience method with DPPIdentity)
    public func withdrawFromIdentity(
        _ identity: DPPIdentity,
        amount: UInt64,
        toAddress: String,
        coreFeePerByte: UInt32 = 0,
        signer: OpaquePointer
    ) async throws -> UInt64 {
        // Convert DPPIdentity to handle
        let identityHandle = try identityToHandle(identity)
        defer {
            // Clean up the handle when done
            dash_sdk_identity_destroy(identityHandle)
        }
        
        // Call the lower-level method
        return try await identityWithdraw(
            identity: identityHandle,
            amount: amount,
            toAddress: toAddress,
            coreFeePerByte: coreFeePerByte,
            publicKey: nil, // Auto-select key
            signer: signer
        )
    }
}