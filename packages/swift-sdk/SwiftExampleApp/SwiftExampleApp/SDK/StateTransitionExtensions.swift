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
        publicKeyId: UInt32 = 0,
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
                        publicKeyId,
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
        publicKeyId: UInt32 = 0,
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
                        publicKeyId,
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
        contractId: String,
        recipientId: String?,
        amount: UInt64,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        print("🟦 TOKEN MINT: Starting token mint operation")
        print("🟦 TOKEN MINT: Contract ID: \(contractId)")
        print("🟦 TOKEN MINT: Recipient ID: \(recipientId ?? "owner (default)")")
        print("🟦 TOKEN MINT: Amount: \(amount)")
        print("🟦 TOKEN MINT: Owner Identity ID: \(ownerIdentity.idString)")
        print("🟦 TOKEN MINT: Note: \(note ?? "none")")
        
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    print("❌ TOKEN MINT: SDK not initialized")
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                print("🟦 TOKEN MINT: Converting owner identity to handle")
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                    print("✅ TOKEN MINT: Successfully converted identity to handle")
                } catch {
                    print("❌ TOKEN MINT: Failed to convert identity to handle: \(error)")
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    print("🟦 TOKEN MINT: Cleaning up identity handle")
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                print("🟦 TOKEN MINT: Owner ID (hex): \(ownerId.toHexString())")
                
                // Convert recipient ID to bytes (or use owner ID if not specified)
                let recipientIdData: Data
                if let recipientId = recipientId {
                    // Normalize the recipient identity ID to base58
                    let normalizedRecipientId = self.normalizeIdentityId(recipientId)
                    print("🟦 TOKEN MINT: Normalized recipient ID: \(normalizedRecipientId)")
                    
                    print("🟦 TOKEN MINT: Converting recipient ID from base58 to bytes")
                    guard let data = Data.identifier(fromBase58: normalizedRecipientId),
                          data.count == 32 else {
                        print("❌ TOKEN MINT: Invalid recipient identity ID - failed to convert from base58 or wrong size")
                        continuation.resume(throwing: SDKError.invalidParameter("Invalid recipient identity ID"))
                        return
                    }
                    recipientIdData = data
                    print("✅ TOKEN MINT: Recipient ID converted to bytes (hex): \(recipientIdData.toHexString())")
                } else {
                    // Use owner ID as recipient if not specified
                    recipientIdData = ownerId
                    print("🟦 TOKEN MINT: No recipient specified, using owner ID as recipient")
                }
                
                // TODO: We need to get the minting key from the owner identity
                // Use the specified key ID
                print("🟦 TOKEN MINT: Using specified minting key ID: \(keyId)")
                
                // Get the public key handle for the minting key
                print("🟦 TOKEN MINT: Getting public key handle for key ID: \(keyId)")
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(keyId)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    print("❌ TOKEN MINT: Failed to get public key handle: \(errorString)")
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                print("✅ TOKEN MINT: Successfully got public key handle")
                defer {
                    print("🟦 TOKEN MINT: Cleaning up public key handle")
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                print("🟦 TOKEN MINT: Preparing to call FFI function dash_sdk_token_mint")
                let result = contractId.withCString { contractIdCStr in
                    recipientIdData.withUnsafeBytes { recipientIdBytes in
                        ownerId.withUnsafeBytes { ownerIdBytes in
                            var params = DashSDKTokenMintParams()
                            params.token_contract_id = contractIdCStr
                            params.serialized_contract = nil
                            params.serialized_contract_len = 0
                            params.token_position = 0 // Default position
                            params.recipient_id = recipientIdBytes.bindMemory(to: UInt8.self).baseAddress
                            params.amount = amount
                            
                            print("🟦 TOKEN MINT: Parameters prepared:")
                            print("  - Contract ID C String: \(String(cString: contractIdCStr))")
                            print("  - Token position: 0")
                            print("  - Amount: \(amount)")
                            print("  - Recipient ID bytes: \(recipientIdData.toHexString())")
                            print("  - Owner ID bytes: \(ownerId.toHexString())")
                            
                            // Handle note
                            if let note = note {
                                print("🟦 TOKEN MINT: Adding note: \(note)")
                                return note.withCString { noteCStr in
                                    params.public_note = noteCStr
                                    
                                    print("🟦 TOKEN MINT: Calling dash_sdk_token_mint WITH note")
                                    return dash_sdk_token_mint(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                }
                            } else {
                                params.public_note = nil
                                
                                print("🟦 TOKEN MINT: Calling dash_sdk_token_mint WITHOUT note")
                                return dash_sdk_token_mint(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        }
                    }
                }
                
                print("🟦 TOKEN MINT: FFI call completed, checking result")
                if result.error == nil {
                    print("✅ TOKEN MINT: Success! Token minted successfully")
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Token minted successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    let errorCode = result.error?.pointee.code.rawValue ?? 0
                    print("❌ TOKEN MINT: Failed with error code \(errorCode): \(errorString)")
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token mint failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Freeze tokens for a target identity
    public func tokenFreeze(
        contractId: String,
        targetIdentityId: String,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Normalize the target identity ID to base58
                let normalizedTargetId = self.normalizeIdentityId(targetIdentityId)
                
                // Convert target ID to bytes
                guard let targetIdData = Data.identifier(fromBase58: normalizedTargetId),
                      targetIdData.count == 32 else {
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid target identity ID"))
                    return
                }
                
                // TODO: We need to get the freezing key from the owner identity
                // For now, we'll assume the first key is the freezing key
                guard let freezingKey = ownerIdentity.publicKeys.values.first else {
                    continuation.resume(throwing: SDKError.invalidParameter("No public keys found in owner identity"))
                    return
                }
                
                // Get the public key handle for the freezing key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(freezingKey.id)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    targetIdData.withUnsafeBytes { targetIdBytes in
                        ownerId.withUnsafeBytes { ownerIdBytes in
                            var params = DashSDKTokenFreezeParams()
                            params.token_contract_id = contractIdCStr
                            params.serialized_contract = nil
                            params.serialized_contract_len = 0
                            params.token_position = 0 // Default position
                            params.target_identity_id = targetIdBytes.bindMemory(to: UInt8.self).baseAddress
                            
                            // Handle note
                            if let note = note {
                                return note.withCString { noteCStr in
                                    params.public_note = noteCStr
                                    
                                    return dash_sdk_token_freeze(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                }
                            } else {
                                params.public_note = nil
                                
                                return dash_sdk_token_freeze(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Token frozen successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token freeze failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Unfreeze tokens for a target identity
    public func tokenUnfreeze(
        contractId: String,
        targetIdentityId: String,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Normalize the target identity ID to base58
                let normalizedTargetId = self.normalizeIdentityId(targetIdentityId)
                
                // Convert target ID to bytes
                guard let targetIdData = Data.identifier(fromBase58: normalizedTargetId),
                      targetIdData.count == 32 else {
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid target identity ID"))
                    return
                }
                
                // TODO: We need to get the unfreezing key from the owner identity
                // For now, we'll assume the first key is the unfreezing key
                guard let unfreezingKey = ownerIdentity.publicKeys.values.first else {
                    continuation.resume(throwing: SDKError.invalidParameter("No public keys found in owner identity"))
                    return
                }
                
                // Get the public key handle for the unfreezing key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(unfreezingKey.id)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    targetIdData.withUnsafeBytes { targetIdBytes in
                        ownerId.withUnsafeBytes { ownerIdBytes in
                            var params = DashSDKTokenFreezeParams()
                            params.token_contract_id = contractIdCStr
                            params.serialized_contract = nil
                            params.serialized_contract_len = 0
                            params.token_position = 0 // Default position
                            params.target_identity_id = targetIdBytes.bindMemory(to: UInt8.self).baseAddress
                            
                            // Handle note
                            if let note = note {
                                return note.withCString { noteCStr in
                                    params.public_note = noteCStr
                                    
                                    return dash_sdk_token_unfreeze(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                }
                            } else {
                                params.public_note = nil
                                
                                return dash_sdk_token_unfreeze(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Token unfrozen successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token unfreeze failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Burn tokens
    public func tokenBurn(
        contractId: String,
        amount: UInt64,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // TODO: We need to get the burning key from the owner identity
                // For now, we'll assume the first key is the burning key
                guard let burningKey = ownerIdentity.publicKeys.values.first else {
                    continuation.resume(throwing: SDKError.invalidParameter("No public keys found in owner identity"))
                    return
                }
                
                // Get the public key handle for the burning key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(burningKey.id)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    ownerId.withUnsafeBytes { ownerIdBytes in
                        var params = DashSDKTokenBurnParams()
                        params.token_contract_id = contractIdCStr
                        params.serialized_contract = nil
                        params.serialized_contract_len = 0
                        params.token_position = 0 // Default position
                        params.amount = amount
                        
                        // Handle note
                        if let note = note {
                            return note.withCString { noteCStr in
                                params.public_note = noteCStr
                                
                                return dash_sdk_token_burn(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        } else {
                            params.public_note = nil
                            
                            return dash_sdk_token_burn(
                                handle,
                                ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                &params,
                                publicKeyHandle,
                                signer,
                                nil,  // Default put settings
                                nil   // Default state transition options
                            )
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Tokens burned successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token burn failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Destroy frozen funds for a frozen identity
    public func tokenDestroyFrozenFunds(
        contractId: String,
        frozenIdentityId: String,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Normalize the frozen identity ID to base58
                let normalizedFrozenId = self.normalizeIdentityId(frozenIdentityId)
                
                // Convert frozen ID to bytes
                guard let frozenIdData = Data.identifier(fromBase58: normalizedFrozenId),
                      frozenIdData.count == 32 else {
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid frozen identity ID"))
                    return
                }
                
                // TODO: We need to get the destroy frozen funds key from the owner identity
                // For now, we'll assume the first key is the destroy frozen funds key
                guard let destroyKey = ownerIdentity.publicKeys.values.first else {
                    continuation.resume(throwing: SDKError.invalidParameter("No public keys found in owner identity"))
                    return
                }
                
                // Get the public key handle for the destroy key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(destroyKey.id)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    frozenIdData.withUnsafeBytes { frozenIdBytes in
                        ownerId.withUnsafeBytes { ownerIdBytes in
                            var params = DashSDKTokenDestroyFrozenFundsParams()
                            params.token_contract_id = contractIdCStr
                            params.serialized_contract = nil
                            params.serialized_contract_len = 0
                            params.token_position = 0 // Default position
                            params.frozen_identity_id = frozenIdBytes.bindMemory(to: UInt8.self).baseAddress
                            
                            // Handle note
                            if let note = note {
                                return note.withCString { noteCStr in
                                    params.public_note = noteCStr
                                    
                                    return dash_sdk_token_destroy_frozen_funds(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                }
                            } else {
                                params.public_note = nil
                                
                                return dash_sdk_token_destroy_frozen_funds(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Frozen funds destroyed successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token destroy frozen funds failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Claim tokens from a distribution
    public func tokenClaim(
        contractId: String,
        distributionType: String,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Get the public key handle for the claiming key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(keyId)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Map distribution type string to enum
                let distributionTypeEnum: DashSDKTokenDistributionType
                switch distributionType.lowercased() {
                case "perpetual":
                    distributionTypeEnum = DashSDKTokenDistributionType(1) // Perpetual = 1
                case "preprogrammed":
                    distributionTypeEnum = DashSDKTokenDistributionType(0) // PreProgrammed = 0
                default:
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid distribution type: \(distributionType)"))
                    return
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    ownerId.withUnsafeBytes { ownerIdBytes in
                        var params = DashSDKTokenClaimParams()
                        params.token_contract_id = contractIdCStr
                        params.serialized_contract = nil
                        params.serialized_contract_len = 0
                        params.token_position = 0 // Default position
                        params.distribution_type = distributionTypeEnum
                        
                        // Handle note
                        if let note = note {
                            return note.withCString { noteCStr in
                                params.public_note = noteCStr
                                
                                return dash_sdk_token_claim(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        } else {
                            params.public_note = nil
                            
                            return dash_sdk_token_claim(
                                handle,
                                ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                &params,
                                publicKeyHandle,
                                signer,
                                nil,  // Default put settings
                                nil   // Default state transition options
                            )
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Tokens claimed successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token claim failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Transfer tokens to another identity
    public func tokenTransfer(
        contractId: String,
        recipientId: String,
        amount: UInt64,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Normalize the recipient identity ID to base58
                let normalizedRecipientId = self.normalizeIdentityId(recipientId)
                
                // Convert recipient ID to bytes
                guard let recipientIdData = Data.identifier(fromBase58: normalizedRecipientId),
                      recipientIdData.count == 32 else {
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid recipient identity ID"))
                    return
                }
                
                // Get the public key handle for the transfer key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(keyId)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    recipientIdData.withUnsafeBytes { recipientIdBytes in
                        ownerId.withUnsafeBytes { ownerIdBytes in
                            var params = DashSDKTokenTransferParams()
                            params.token_contract_id = contractIdCStr
                            params.serialized_contract = nil
                            params.serialized_contract_len = 0
                            params.token_position = 0 // Default position
                            params.recipient_id = recipientIdBytes.bindMemory(to: UInt8.self).baseAddress
                            params.amount = amount
                            params.private_encrypted_note = nil
                            params.shared_encrypted_note = nil
                            
                            // Handle note
                            if let note = note {
                                return note.withCString { noteCStr in
                                    params.public_note = noteCStr
                                    
                                    return dash_sdk_token_transfer(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                }
                            } else {
                                params.public_note = nil
                                
                                return dash_sdk_token_transfer(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Tokens transferred successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token transfer failed: \(errorString)"))
                }
            }
        }
    }
    
    /// Set token price for direct purchase
    public func tokenSetPrice(
        contractId: String,
        pricingType: String,
        priceData: String?,
        ownerIdentity: DPPIdentity,
        keyId: KeyID,
        signer: OpaquePointer,
        note: String? = nil
    ) async throws -> [String: Any] {
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert owner identity to handle
                let ownerIdentityHandle: OpaquePointer
                do {
                    ownerIdentityHandle = try self.identityToHandle(ownerIdentity)
                } catch {
                    continuation.resume(throwing: error)
                    return
                }
                
                defer {
                    // Clean up the identity handle when done
                    dash_sdk_identity_destroy(ownerIdentityHandle)
                }
                
                // Get the owner ID from the identity
                let ownerId = ownerIdentity.id
                
                // Get the public key handle for the pricing key
                let keyHandleResult = dash_sdk_identity_get_public_key_by_id(
                    ownerIdentityHandle,
                    UInt8(keyId)
                )
                
                guard keyHandleResult.error == nil,
                      let keyHandleData = keyHandleResult.data else {
                    let errorString = keyHandleResult.error?.pointee.message != nil ?
                        String(cString: keyHandleResult.error!.pointee.message) : "Failed to get public key"
                    dash_sdk_error_free(keyHandleResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let publicKeyHandle = OpaquePointer(keyHandleData)!
                defer {
                    // Clean up the public key handle when done
                    dash_sdk_identity_public_key_destroy(publicKeyHandle)
                }
                
                // Map pricing type string to enum
                let pricingTypeEnum: DashSDKTokenPricingType
                switch pricingType.lowercased() {
                case "single":
                    pricingTypeEnum = DashSDKTokenPricingType(0) // SinglePrice = 0
                case "tiered":
                    pricingTypeEnum = DashSDKTokenPricingType(1) // SetPrices = 1
                default:
                    continuation.resume(throwing: SDKError.invalidParameter("Invalid pricing type: \(pricingType)"))
                    return
                }
                
                // Call the FFI function with proper parameters
                let result = contractId.withCString { contractIdCStr in
                    ownerId.withUnsafeBytes { ownerIdBytes in
                        var params = DashSDKTokenSetPriceParams()
                        params.token_contract_id = contractIdCStr
                        params.serialized_contract = nil
                        params.serialized_contract_len = 0
                        params.token_position = 0 // Default position
                        params.pricing_type = pricingTypeEnum
                        params.price_entries = nil
                        params.price_entries_count = 0
                        
                        // Handle pricing data based on type
                        if pricingTypeEnum.rawValue == 0 { // SinglePrice
                            if let priceData = priceData, !priceData.isEmpty {
                                params.single_price = UInt64(priceData) ?? 0
                            } else {
                                params.single_price = 0 // Remove pricing
                            }
                        } else { // SetPrices - for now, we'll leave this as TODO
                            params.single_price = 0
                            // TODO: Parse price data as JSON for tiered pricing
                        }
                        
                        // Handle note
                        if let note = note {
                            return note.withCString { noteCStr in
                                params.public_note = noteCStr
                                
                                return dash_sdk_token_set_price(
                                    handle,
                                    ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                    &params,
                                    publicKeyHandle,
                                    signer,
                                    nil,  // Default put settings
                                    nil   // Default state transition options
                                )
                            }
                        } else {
                            params.public_note = nil
                            
                            return dash_sdk_token_set_price(
                                handle,
                                ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                &params,
                                publicKeyHandle,
                                signer,
                                nil,  // Default put settings
                                nil   // Default state transition options
                            )
                        }
                    }
                }
                
                if result.error == nil {
                    // Parse the result
                    // TODO: Parse actual result structure
                    continuation.resume(returning: [
                        "success": true,
                        "message": "Token price set successfully"
                    ])
                } else {
                    let errorString = result.error?.pointee.message != nil ?
                        String(cString: result.error!.pointee.message) : "Unknown error"
                    dash_sdk_error_free(result.error)
                    continuation.resume(throwing: SDKError.internalError("Token set price failed: \(errorString)"))
                }
            }
        }
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
            publicKeyId: 0, // Auto-select TRANSFER key
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
            publicKeyId: 0, // Auto-select TRANSFER key
            signer: signer
        )
    }
    
    // MARK: - Helper Methods
    
    private func normalizeIdentityId(_ identityId: String) -> String {
        // Remove any prefix
        let cleanId = identityId
            .replacingOccurrences(of: "id:", with: "")
            .replacingOccurrences(of: "0x", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        
        // If it's hex (64 chars), convert to base58
        if cleanId.count == 64, let data = Data(hexString: cleanId) {
            return data.toBase58String()
        }
        
        // Otherwise assume it's already base58
        return cleanId
    }
}