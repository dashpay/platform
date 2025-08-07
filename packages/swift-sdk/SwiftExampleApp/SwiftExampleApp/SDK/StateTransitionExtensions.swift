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
        ownerIdentity: DPPIdentity,
        properties: [String: Any],
        signer: OpaquePointer
    ) async throws -> [String: Any] {
        let startTime = Date()
        print("📝 [DOCUMENT CREATE] Starting at \(startTime)")
        print("📝 [DOCUMENT CREATE] Contract ID: \(contractId)")
        print("📝 [DOCUMENT CREATE] Document Type: \(documentType)")
        print("📝 [DOCUMENT CREATE] Owner ID: \(ownerIdentity.idString)")
        
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    print("❌ [DOCUMENT CREATE] SDK not initialized")
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // Convert properties to JSON
                print("📝 [DOCUMENT CREATE] Converting properties to JSON...")
                guard let propertiesData = try? JSONSerialization.data(withJSONObject: properties),
                      let propertiesJson = String(data: propertiesData, encoding: .utf8) else {
                    print("❌ [DOCUMENT CREATE] Failed to serialize properties")
                    continuation.resume(throwing: SDKError.invalidParameter("Failed to serialize properties to JSON"))
                    return
                }
                print("✅ [DOCUMENT CREATE] Properties JSON created: \(propertiesJson.prefix(100))...")
                
                // 1. Create document using contract from trusted context (no network fetches needed)
                print("📝 [DOCUMENT CREATE] Creating document with contract from trusted context...")
                let identityIdString = ownerIdentity.id.toBase58String()
                print("📝 [DOCUMENT CREATE] Identity ID (base58): \(identityIdString)")
                
                let createStart = Date()
                let createResult = contractId.withCString { contractIdCStr in
                    documentType.withCString { docTypeCStr in
                        identityIdString.withCString { identityIdCStr in
                            propertiesJson.withCString { propsCStr in
                                var createParams = DashSDKDocumentCreateParams(
                                    data_contract_id: contractIdCStr,
                                    document_type: docTypeCStr,
                                    owner_identity_id: identityIdCStr,
                                    properties_json: propsCStr
                                )
                                return withUnsafePointer(to: &createParams) { paramsPtr in
                                    dash_sdk_document_create(handle, paramsPtr)
                                }
                            }
                        }
                    }
                }
                let createTime = Date().timeIntervalSince(createStart)
                print("⏱️ [DOCUMENT CREATE] Document creation took \(createTime) seconds")
                
                guard createResult.error == nil else {
                    let errorString = createResult.error?.pointee.message != nil ?
                        String(cString: createResult.error!.pointee.message) : "Failed to create document"
                    print("❌ [DOCUMENT CREATE] Document creation failed: \(errorString)")
                    print("⏱️ [DOCUMENT CREATE] Total time before failure: \(Date().timeIntervalSince(startTime)) seconds")
                    dash_sdk_error_free(createResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                guard createResult.data_type == DashSDKFFI.ResultDocumentHandle,
                      let documentHandle = createResult.data else {
                    print("❌ [DOCUMENT CREATE] Invalid document result type")
                    continuation.resume(throwing: SDKError.internalError("Invalid document result type"))
                    return
                }
                print("✅ [DOCUMENT CREATE] Document handle created")
                
                defer {
                    // Clean up document handle when done
                    dash_sdk_document_handle_destroy(OpaquePointer(documentHandle))
                }
                
                // 2. Create identity public key handle directly from our local data (no network fetch)
                print("📝 [DOCUMENT CREATE] Getting public key handle...")
                let authKey = ownerIdentity.publicKeys.values.first { key in
                    key.purpose == .authentication
                } ?? ownerIdentity.publicKeys.values.first
                
                guard let keyToUse = authKey else {
                    print("❌ [DOCUMENT CREATE] No public key found for identity")
                    continuation.resume(throwing: SDKError.invalidParameter("No public key found for identity"))
                    return
                }
                print("📝 [DOCUMENT CREATE] Using key ID: \(keyToUse.id), purpose: \(keyToUse.purpose), type: \(keyToUse.keyType), security: \(keyToUse.securityLevel)")
                
                // Create public key handle directly from our local data
                let keyData = keyToUse.data
                let keyType: UInt8 = UInt8(keyToUse.keyType.rawValue)
                let purpose: UInt8 = {
                    switch keyToUse.purpose {
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
                    switch keyToUse.securityLevel {
                    case .master: return 0
                    case .critical: return 1
                    case .high: return 2
                    case .medium: return 3
                    }
                }()
                
                let keyResult = keyData.withUnsafeBytes { dataPtr in
                    dash_sdk_identity_public_key_create_from_data(
                        UInt32(keyToUse.id),
                        keyType,
                        purpose,
                        securityLevel,
                        dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        UInt(keyData.count),
                        keyToUse.readOnly,
                        keyToUse.disabledAt ?? 0
                    )
                }
                
                guard keyResult.error == nil else {
                    let errorString = keyResult.error?.pointee.message != nil ?
                        String(cString: keyResult.error!.pointee.message) : "Failed to create public key handle"
                    print("❌ [DOCUMENT CREATE] Key handle creation failed: \(errorString)")
                    print("⏱️ [DOCUMENT CREATE] Total time before failure: \(Date().timeIntervalSince(startTime)) seconds")
                    dash_sdk_error_free(keyResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                guard let keyHandle = keyResult.data else {
                    print("❌ [DOCUMENT CREATE] Invalid public key handle")
                    continuation.resume(throwing: SDKError.internalError("Invalid public key handle"))
                    return
                }
                print("✅ [DOCUMENT CREATE] Public key handle created from local data (no network fetch!)")
                
                defer {
                    // Clean up key handle  
                    dash_sdk_identity_public_key_destroy(OpaquePointer(keyHandle))
                }
                
                // 4. Create put settings (null for defaults)
                let putSettings: UnsafePointer<DashSDKPutSettings>? = nil
                let tokenPaymentInfo: UnsafePointer<DashSDKTokenPaymentInfo>? = nil
                let stateTransitionOptions: UnsafePointer<DashSDKStateTransitionCreationOptions>? = nil
                
                // Generate entropy for document ID
                var entropy = (
                    UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                    UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                    UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                    UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0)
                )
                withUnsafeMutableBytes(of: &entropy) { entropyBytes in
                    _ = SecRandomCopyBytes(kSecRandomDefault, 32, entropyBytes.baseAddress!)
                }
                
                // 5. Put document to platform and wait (using contract ID from trusted context)
                print("🚀 [DOCUMENT CREATE] Submitting document to platform...")
                print("🚀 [DOCUMENT CREATE] This is the NETWORK CALL - using contract from trusted context...")
                let putStart = Date()
                let putResult = withUnsafePointer(to: &entropy) { entropyPtr in
                    contractId.withCString { contractIdCStr in
                        documentType.withCString { docTypeCStr in
                            dash_sdk_document_put_to_platform_and_wait(
                                handle,
                                OpaquePointer(documentHandle),
                                contractIdCStr,
                                docTypeCStr,
                                entropyPtr,
                                OpaquePointer(keyHandle),
                                signer,
                                tokenPaymentInfo,
                                putSettings,
                                stateTransitionOptions
                            )
                        }
                    }
                }
                let putTime = Date().timeIntervalSince(putStart)
                print("⏱️ [DOCUMENT CREATE] Platform submission took \(putTime) seconds")
                print("✅ [DOCUMENT CREATE] Received response from platform (no timeout!)")
                
                if let error = putResult.error {
                    let errorString = error.pointee.message != nil ?
                        String(cString: error.pointee.message) : "Failed to put document to platform"
                    print("❌ [DOCUMENT CREATE] Platform submission failed: \(errorString)")
                    print("⏱️ [DOCUMENT CREATE] Total operation time: \(Date().timeIntervalSince(startTime)) seconds")
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                } else if putResult.data_type == DashSDKFFI.String,
                          let jsonData = putResult.data {
                    // Parse the returned JSON
                    let jsonString = String(cString: UnsafePointer<CChar>(OpaquePointer(jsonData)))
                    dash_sdk_string_free(UnsafeMutablePointer<CChar>(mutating: UnsafePointer<CChar>(OpaquePointer(jsonData))))
                    
                    print("✅ [DOCUMENT CREATE] Success! Total operation time: \(Date().timeIntervalSince(startTime)) seconds")
                    print("📝 [DOCUMENT CREATE] Response: \(jsonString.prefix(200))...")
                    
                    if let data = jsonString.data(using: .utf8),
                       let jsonObject = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                        continuation.resume(returning: jsonObject)
                    } else {
                        continuation.resume(returning: ["status": "success", "raw": jsonString])
                    }
                } else {
                    print("✅ [DOCUMENT CREATE] Success! Total operation time: \(Date().timeIntervalSince(startTime)) seconds")
                    continuation.resume(returning: ["status": "success", "message": "Document created successfully"])
                }
            }
        }
    }
    
    /// Replace an existing document
    public func documentReplace(
        contractId: String,
        documentType: String,
        documentId: String,
        ownerIdentity: DPPIdentity,
        properties: [String: Any],
        signer: OpaquePointer
    ) async throws -> [String: Any] {
        let startTime = Date()
        print("📝 [DOCUMENT REPLACE] Starting at \(startTime)")
        print("📝 [DOCUMENT REPLACE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // 1. Fetch the document first
                print("📝 [DOCUMENT REPLACE] Fetching existing document...")
                let docFetchStart = Date()
                let documentResult = documentId.withCString { docIdCStr in
                    contractId.withCString { contractIdCStr in
                        documentType.withCString { docTypeCStr in
                            dash_sdk_document_fetch(handle, nil, docTypeCStr, docIdCStr)
                        }
                    }
                }
                
                let docFetchTime = Date().timeIntervalSince(docFetchStart)
                print("⏱️ [DOCUMENT REPLACE] Document fetch took \(docFetchTime) seconds")
                
                guard documentResult.error == nil else {
                    print("❌ [DOCUMENT REPLACE] Failed to fetch document after \(docFetchTime) seconds")
                    let errorString = documentResult.error?.pointee.message != nil ?
                        String(cString: documentResult.error!.pointee.message) : "Failed to fetch document"
                    dash_sdk_error_free(documentResult.error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                guard documentResult.data_type == DashSDKFFI.ResultDocumentHandle,
                      let documentHandle = documentResult.data else {
                    continuation.resume(throwing: SDKError.internalError("Invalid document result type"))
                    return
                }
                
                defer {
                    dash_sdk_document_handle_destroy(OpaquePointer(documentHandle))
                }
                
                print("✅ [DOCUMENT REPLACE] Document fetched successfully")
                
                // 2. Fetch the data contract handle
                print("📝 [DOCUMENT REPLACE] Fetching data contract...")
                let contractFetchStart = Date()
                let contractResult = contractId.withCString { contractIdCStr in
                    dash_sdk_data_contract_fetch(handle, contractIdCStr)
                }
                
                let contractFetchTime = Date().timeIntervalSince(contractFetchStart)
                print("⏱️ [DOCUMENT REPLACE] Contract fetch took \(contractFetchTime) seconds")
                
                guard contractResult.error == nil,
                      contractResult.data_type == DashSDKFFI.ResultDataContractHandle,
                      let contractHandle = contractResult.data else {
                    if contractResult.error != nil {
                        let errorString = String(cString: contractResult.error!.pointee.message)
                        dash_sdk_error_free(contractResult.error)
                        continuation.resume(throwing: SDKError.internalError(errorString))
                    } else {
                        continuation.resume(throwing: SDKError.internalError("Failed to fetch contract"))
                    }
                    return
                }
                
                defer {
                    dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
                }
                
                print("✅ [DOCUMENT REPLACE] Contract fetched successfully")
                
                // 3. Fetch the identity handle
                print("📝 [DOCUMENT REPLACE] Fetching identity handle...")
                let identityFetchStart = Date()
                let identityIdString = ownerIdentity.id.toBase58String()
                let identityResult = identityIdString.withCString { identityIdCStr in
                    dash_sdk_identity_fetch_handle(handle, identityIdCStr)
                }
                
                let identityFetchTime = Date().timeIntervalSince(identityFetchStart)
                print("⏱️ [DOCUMENT REPLACE] Identity fetch took \(identityFetchTime) seconds")
                
                guard identityResult.error == nil,
                      identityResult.data_type == DashSDKFFI.ResultIdentityHandle,
                      let identityHandle = identityResult.data else {
                    if identityResult.error != nil {
                        let errorString = String(cString: identityResult.error!.pointee.message)
                        dash_sdk_error_free(identityResult.error)
                        continuation.resume(throwing: SDKError.internalError(errorString))
                    } else {
                        continuation.resume(throwing: SDKError.internalError("Failed to fetch identity"))
                    }
                    return
                }
                
                defer {
                    dash_sdk_identity_destroy(OpaquePointer(identityHandle))
                }
                
                print("✅ [DOCUMENT REPLACE] Identity fetched successfully")
                
                // 4. Get public key handle
                print("📝 [DOCUMENT REPLACE] Getting public key handle...")
                let keyFetchStart = Date()
                let authKey = ownerIdentity.publicKeys.values.first { key in
                    key.purpose == .authentication
                } ?? ownerIdentity.publicKeys.values.first
                
                guard let keyToUse = authKey else {
                    continuation.resume(throwing: SDKError.invalidParameter("No public key found"))
                    return
                }
                
                let keyResult = dash_sdk_identity_get_public_key_by_id(
                    OpaquePointer(identityHandle),
                    UInt8(keyToUse.id)
                )
                
                let keyFetchTime = Date().timeIntervalSince(keyFetchStart)
                print("⏱️ [DOCUMENT REPLACE] Key fetch took \(keyFetchTime) seconds")
                
                guard keyResult.error == nil,
                      keyResult.data_type == DashSDKFFI.ResultPublicKeyHandle,
                      let keyHandle = keyResult.data else {
                    if keyResult.error != nil {
                        let errorString = String(cString: keyResult.error!.pointee.message)
                        dash_sdk_error_free(keyResult.error)
                        continuation.resume(throwing: SDKError.internalError(errorString))
                    } else {
                        continuation.resume(throwing: SDKError.internalError("Failed to get public key"))
                    }
                    return
                }
                
                defer {
                    dash_sdk_identity_public_key_destroy(OpaquePointer(keyHandle))
                }
                
                print("✅ [DOCUMENT REPLACE] Public key fetched successfully")
                
                // 5. Replace document on platform
                print("🚀 [DOCUMENT REPLACE] This is the NETWORK CALL - monitoring for timeout...")
                let replaceStart = Date()
                let putResult = documentType.withCString { docTypeCStr in
                    dash_sdk_document_replace_on_platform_and_wait(
                        handle,
                        OpaquePointer(documentHandle),
                        OpaquePointer(contractHandle),
                        docTypeCStr,
                        OpaquePointer(keyHandle),
                        signer,
                        nil, // token payment info
                        nil, // put settings
                        nil  // state transition options
                    )
                }
                
                let replaceTime = Date().timeIntervalSince(replaceStart)
                print("⏱️ [DOCUMENT REPLACE] Platform submission took \(replaceTime) seconds")
                
                if let error = putResult.error {
                    print("❌ [DOCUMENT REPLACE] Network call failed after \(replaceTime) seconds")
                    let errorString = String(cString: error.pointee.message)
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                } else if putResult.data_type == DashSDKFFI.String,
                          let jsonData = putResult.data {
                    let jsonString = String(cString: UnsafePointer<CChar>(OpaquePointer(jsonData)))
                    dash_sdk_string_free(UnsafeMutablePointer<CChar>(mutating: UnsafePointer<CChar>(OpaquePointer(jsonData))))
                    
                    if let data = jsonString.data(using: .utf8),
                       let jsonObject = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                        let totalTime = Date().timeIntervalSince(startTime)
                        print("✅ [DOCUMENT REPLACE] Response received - document replaced successfully")
                        print("✅ [DOCUMENT REPLACE] Total operation time: \(totalTime) seconds")
                        continuation.resume(returning: jsonObject)
                    } else {
                        let totalTime = Date().timeIntervalSince(startTime)
                        print("✅ [DOCUMENT REPLACE] Response received - document replaced successfully")
                        print("✅ [DOCUMENT REPLACE] Total operation time: \(totalTime) seconds")
                        continuation.resume(returning: ["status": "success", "raw": jsonString])
                    }
                } else {
                    let totalTime = Date().timeIntervalSince(startTime)
                    print("✅ [DOCUMENT REPLACE] Document replaced successfully")
                    print("✅ [DOCUMENT REPLACE] Total operation time: \(totalTime) seconds")
                    continuation.resume(returning: ["status": "success", "message": "Document replaced successfully"])
                }
            }
        }
    }
    
    /// Delete a document
    public func documentDelete(
        contractId: String,
        documentType: String,
        documentId: String,
        ownerIdentity: DPPIdentity,
        signer: OpaquePointer
    ) async throws {
        let startTime = Date()
        print("🗑️ [DOCUMENT DELETE] Starting at \(startTime)")
        print("🗑️ [DOCUMENT DELETE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        
        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // TODO: Implement full document delete with logging similar to documentReplace:
                // 1. Fetch document with timing
                // 2. Fetch contract with timing
                // 3. Fetch identity with timing
                // 4. Get public key with timing
                // 5. Call dash_sdk_document_delete_and_wait with network timing
                
                print("⚠️ [DOCUMENT DELETE] Not fully implemented yet")
                let totalTime = Date().timeIntervalSince(startTime)
                print("⚠️ [DOCUMENT DELETE] Total time: \(totalTime) seconds")
                
                continuation.resume(throwing: SDKError.notImplemented(
                    "Document delete implementation similar to replace - handles are available"
                ))
            }
        }
    }
    
    /// Transfer a document to another identity
    public func documentTransfer(
        contractId: String,
        documentType: String,
        documentId: String,
        fromIdentity: DPPIdentity,
        toIdentityId: String,
        signer: OpaquePointer
    ) async throws -> [String: Any] {
        let startTime = Date()
        print("🔁 [DOCUMENT TRANSFER] Starting at \(startTime)")
        print("🔁 [DOCUMENT TRANSFER] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        print("🔁 [DOCUMENT TRANSFER] From: \(fromIdentity.id.toBase58String()), To: \(toIdentityId)")
        
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // TODO: Implement full document transfer with logging:
                // 1. Fetch document with timing
                // 2. Fetch contract with timing
                // 3. Fetch from identity with timing
                // 4. Fetch to identity with timing
                // 5. Get public key with timing
                // 6. Call dash_sdk_document_transfer_to_identity_and_wait with network timing
                
                print("⚠️ [DOCUMENT TRANSFER] Not fully implemented yet")
                let totalTime = Date().timeIntervalSince(startTime)
                print("⚠️ [DOCUMENT TRANSFER] Total time: \(totalTime) seconds")
                
                continuation.resume(throwing: SDKError.notImplemented(
                    "Document transfer implementation similar to replace - handles are available"
                ))
            }
        }
    }
    
    /// Purchase a document
    public func documentPurchase(
        contractId: String,
        documentType: String,
        documentId: String,
        purchaserIdentity: DPPIdentity,
        price: UInt64,
        signer: OpaquePointer
    ) async throws -> [String: Any] {
        let startTime = Date()
        print("🛍️ [DOCUMENT PURCHASE] Starting at \(startTime)")
        print("🛍️ [DOCUMENT PURCHASE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        print("🛍️ [DOCUMENT PURCHASE] Purchaser: \(purchaserIdentity.id.toBase58String()), Price: \(price)")
        
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // TODO: Implement full document purchase with logging:
                // 1. Fetch document with timing
                // 2. Fetch contract with timing
                // 3. Fetch purchaser identity with timing
                // 4. Get public key with timing
                // 5. Call dash_sdk_document_purchase_and_wait with network timing
                
                print("⚠️ [DOCUMENT PURCHASE] Not fully implemented yet")
                let totalTime = Date().timeIntervalSince(startTime)
                print("⚠️ [DOCUMENT PURCHASE] Total time: \(totalTime) seconds")
                
                continuation.resume(throwing: SDKError.notImplemented(
                    "Document purchase implementation similar to replace - handles are available"
                ))
            }
        }
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
                                    
                                    print("🚀 [TOKEN MINT] Submitting to platform WITH note...")
                                    print("🚀 [TOKEN MINT] This is the NETWORK CALL - monitoring for timeout...")
                                    let mintStart = Date()
                                    let result = dash_sdk_token_mint(
                                        handle,
                                        ownerIdBytes.bindMemory(to: UInt8.self).baseAddress!,
                                        &params,
                                        publicKeyHandle,
                                        signer,
                                        nil,  // Default put settings
                                        nil   // Default state transition options
                                    )
                                    let mintTime = Date().timeIntervalSince(mintStart)
                                    print("⏱️ [TOKEN MINT] Network call took \(mintTime) seconds")
                                    print("✅ [TOKEN MINT] Received response from platform (no timeout!)")
                                    return result
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