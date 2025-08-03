import Foundation
import SwiftDashSDK

// MARK: - State Transition Type
public enum StateTransitionType: UInt32 {
    case identityUpdate = 0
    case identityTopUp = 1
    case identityCreditTransfer = 2
    case identityCreditWithdrawal = 3
    case documentsBatch = 4
    case dataContractCreate = 5
    case dataContractUpdate = 6
}

// MARK: - State Transition Extensions

extension SDK {
    
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
        identityId: String,
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
                
                // First fetch the identity to get its handle
                let fetchResult = identityId.withCString { idCStr in
                    dash_sdk_identity_fetch(handle, idCStr)
                }
                
                guard fetchResult.error == nil,
                      let identityHandle = fetchResult.data else {
                    let errorString = fetchResult.error?.pointee.message != nil ?
                        String(cString: fetchResult.error!.pointee.message) : "Failed to fetch identity"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                let result = instantLock.withUnsafeBytes { instantLockBytes in
                    transaction.withUnsafeBytes { txBytes in
                        privateKey.withUnsafeBytes { keyBytes in
                            dash_sdk_identity_topup_with_instant_lock(
                                handle,
                                OpaquePointer(identityHandle)!,
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
                
                // Clean up the identity handle
                dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                
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
        fromIdentityId: String,
        toIdentityId: String,
        amount: UInt64,
        signerPrivateKey: Data? = nil
    ) async throws -> (senderBalance: UInt64, receiverBalance: UInt64) {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // First fetch the from identity to get its handle
                let fetchResult = fromIdentityId.withCString { idCStr in
                    dash_sdk_identity_fetch(handle, idCStr)
                }
                
                guard fetchResult.error == nil,
                      let fromIdentityHandle = fetchResult.data else {
                    let errorString = fetchResult.error?.pointee.message != nil ?
                        String(cString: fetchResult.error!.pointee.message) : "Failed to fetch from identity"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                var signerHandle: UnsafeMutableRawPointer?
                var selectedKeyHandle: UnsafeMutableRawPointer?
                
                if let signerPrivateKey = signerPrivateKey {
                    // Use provided private key
                    guard signerPrivateKey.count == 32 else {
                        dash_sdk_identity_destroy(OpaquePointer(fromIdentityHandle)!)
                        continuation.resume(throwing: SDKError.invalidParameter("Signer private key must be 32 bytes"))
                        return
                    }
                    
                    // Create a signer from the private key
                    let signerResult = signerPrivateKey.withUnsafeBytes { keyBytes in
                        dash_sdk_signer_create_from_private_key(
                            keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                            UInt(signerPrivateKey.count)
                        )
                    }
                    
                    guard signerResult.error == nil,
                          let signer = signerResult.data else {
                        dash_sdk_identity_destroy(OpaquePointer(fromIdentityHandle)!)
                        let errorString = signerResult.error?.pointee.message != nil ?
                            String(cString: signerResult.error!.pointee.message) : "Failed to create signer"
                        continuation.resume(throwing: SDKError.internalError(errorString))
                        return
                    }
                    signerHandle = signer
                } else {
                    // Auto-select signing key
                    let keyResult = dash_sdk_identity_get_signing_key_for_transition(
                        OpaquePointer(fromIdentityHandle)!,
                        DashUnifiedSDK.StateTransitionType(rawValue: StateTransitionType.identityCreditTransfer.rawValue)
                    )
                    
                    guard keyResult.error == nil,
                          let keyHandle = keyResult.data else {
                        dash_sdk_identity_destroy(OpaquePointer(fromIdentityHandle)!)
                        let errorString = keyResult.error?.pointee.message != nil ?
                            String(cString: keyResult.error!.pointee.message) : "Failed to get signing key"
                        continuation.resume(throwing: SDKError.internalError(errorString))
                        return
                    }
                    selectedKeyHandle = keyHandle
                    
                    // TODO: In a real implementation, we would get the private key for this public key
                    // from the wallet/key storage. For now, we'll return an error.
                    dash_sdk_identity_public_key_destroy(OpaquePointer(keyHandle)!)
                    dash_sdk_identity_destroy(OpaquePointer(fromIdentityHandle)!)
                    continuation.resume(throwing: SDKError.internalError("Automatic key selection requires wallet integration"))
                    return
                }
                
                // Transfer credits
                let result = toIdentityId.withCString { toIdCStr in
                    dash_sdk_identity_transfer_credits(
                        handle,
                        OpaquePointer(fromIdentityHandle)!,
                        toIdCStr,
                        amount,
                        selectedKeyHandle != nil ? OpaquePointer(selectedKeyHandle!) : nil,
                        OpaquePointer(signerHandle!)!,
                        nil  // Default put settings
                    )
                }
                
                // Clean up handles
                dash_sdk_identity_destroy(OpaquePointer(fromIdentityHandle)!)
                dash_sdk_signer_destroy(OpaquePointer(signerHandle)!)
                
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
        identityId: String,
        amount: UInt64,
        toAddress: String,
        coreFeePerByte: UInt32 = 0,
        signerPrivateKey: Data? = nil
    ) async throws -> UInt64 {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }
                
                // First fetch the identity to get its handle
                let fetchResult = identityId.withCString { idCStr in
                    dash_sdk_identity_fetch(handle, idCStr)
                }
                
                guard fetchResult.error == nil,
                      let identityHandle = fetchResult.data else {
                    let errorString = fetchResult.error?.pointee.message != nil ?
                        String(cString: fetchResult.error!.pointee.message) : "Failed to fetch identity"
                    continuation.resume(throwing: SDKError.internalError(errorString))
                    return
                }
                
                var signerHandle: UnsafeMutableRawPointer?
                var selectedKeyHandle: UnsafeMutableRawPointer?
                
                if let signerPrivateKey = signerPrivateKey {
                    // Use provided private key
                    guard signerPrivateKey.count == 32 else {
                        dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                        continuation.resume(throwing: SDKError.invalidParameter("Signer private key must be 32 bytes"))
                        return
                    }
                    
                    // Create a signer from the private key
                    let signerResult = signerPrivateKey.withUnsafeBytes { keyBytes in
                        dash_sdk_signer_create_from_private_key(
                            keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                            UInt(signerPrivateKey.count)
                        )
                    }
                    
                    guard signerResult.error == nil,
                          let signer = signerResult.data else {
                        dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                        let errorString = signerResult.error?.pointee.message != nil ?
                            String(cString: signerResult.error!.pointee.message) : "Failed to create signer"
                        continuation.resume(throwing: SDKError.internalError(errorString))
                        return
                    }
                    signerHandle = signer
                } else {
                    // Auto-select signing key
                    let keyResult = dash_sdk_identity_get_signing_key_for_transition(
                        OpaquePointer(identityHandle)!,
                        DashUnifiedSDK.StateTransitionType(rawValue: StateTransitionType.identityCreditWithdrawal.rawValue)
                    )
                    
                    guard keyResult.error == nil,
                          let keyHandle = keyResult.data else {
                        dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                        let errorString = keyResult.error?.pointee.message != nil ?
                            String(cString: keyResult.error!.pointee.message) : "Failed to get signing key"
                        continuation.resume(throwing: SDKError.internalError(errorString))
                        return
                    }
                    selectedKeyHandle = keyHandle
                    
                    // TODO: In a real implementation, we would get the private key for this public key
                    // from the wallet/key storage. For now, we'll return an error.
                    dash_sdk_identity_public_key_destroy(OpaquePointer(keyHandle)!)
                    dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                    continuation.resume(throwing: SDKError.internalError("Automatic key selection requires wallet integration"))
                    return
                }
                
                // Withdraw credits
                let result = toAddress.withCString { addressCStr in
                    dash_sdk_identity_withdraw(
                        handle,
                        OpaquePointer(identityHandle)!,
                        addressCStr,
                        amount,
                        coreFeePerByte,
                        selectedKeyHandle != nil ? OpaquePointer(selectedKeyHandle!) : nil,
                        OpaquePointer(signerHandle!)!,
                        nil  // Default put settings
                    )
                }
                
                // Clean up handles
                dash_sdk_identity_destroy(OpaquePointer(identityHandle)!)
                dash_sdk_signer_destroy(OpaquePointer(signerHandle)!)
                
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