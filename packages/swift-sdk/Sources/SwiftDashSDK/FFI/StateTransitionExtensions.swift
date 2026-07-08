import Foundation
import DashSDKFFI

// MARK: - Sendable wrappers
private final class SendableOpaque: @unchecked Sendable { let p: OpaquePointer; init(_ p: OpaquePointer) { self.p = p } }

// MARK: - Key Selection Helpers

/// Ask the SUPPLIED signer — not `KeychainManager.shared` — whether it holds
/// signable private material for `key`. Pure marshalling: it ships the key's
/// raw public-key bytes + `KeyType` discriminant byte across the FFI and
/// returns the signer's own verdict via `dash_sdk_signer_can_sign`. The
/// availability *decision* lives entirely in Rust / the signer; Swift only
/// converts the bytes. Never pulls private-key material across the boundary.
///
/// `key.keyType.rawValue` is the dpp `KeyType` discriminant (0–4) the Rust
/// side expects; identity keys only.
private func signerCanSign(_ signer: OpaquePointer, _ key: IdentityPublicKey) -> Bool {
    key.data.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> Bool in
        dash_sdk_signer_can_sign(
            signer,
            raw.bindMemory(to: UInt8.self).baseAddress,
            UInt(key.data.count),
            key.keyType.rawValue
        )
    }
}

/// Helper to select the appropriate key for signing operations.
///
/// Key-selection *policy* (rank candidates by purpose / security level) runs
/// in Swift via `KeyManager.rankSigningCandidates`; the *availability* check
/// ("does this signer actually hold the private material?") is delegated to
/// the SUPPLIED `signer` through `dash_sdk_signer_can_sign`. This removes the
/// old coupling to `KeychainManager.shared`: a signer bound to a different
/// `KeychainManager`, a raw private-key signer (`KeyManager.createSigner`),
/// or a hardware-backed callback signer is now consulted directly, so the
/// preflight matches exactly what will happen at sign time.
///
/// When `signer == nil` (no signer available to ask) this falls back to the
/// shared-Keychain `KeyManager.findSigningKey` path — same behavior as before
/// the FFI delegation existed.
@MainActor
private func selectSigningKey(
    from identity: DPPIdentity,
    operation: String,
    signer: OpaquePointer?
) -> IdentityPublicKey? {
    print("🔑 [\(operation)] Selecting public key for identity \(identity.id.toBase58String())")

    // Contract create/update require a CRITICAL + AUTHENTICATION key;
    // document ops require any AUTHENTICATION key (prefer CRITICAL).
    let isContractOp = operation == "CONTRACT CREATE" || operation == "CONTRACT UPDATE"
    let minimumSecurityLevel: SecurityLevel? = isContractOp ? .critical : nil

    let km = KeyManager.withSharedKeychain()

    // No signer to ask → fall back to the shared-Keychain availability check.
    guard let signer = signer else {
        if let k = km.findSigningKey(
            for: identity,
            purpose: .authentication,
            minimumSecurityLevel: minimumSecurityLevel,
            preferCritical: true
        ) {
            if isContractOp {
                print("📝 [\(operation)] Selected CRITICAL AUTHENTICATION key #\(k.id)")
            } else {
                print("📝 [\(operation)] Selected AUTHENTICATION key #\(k.id) (security \(k.securityLevel.name))")
            }
            return k
        }
        if isContractOp {
            print("❌ [\(operation)] No CRITICAL AUTHENTICATION key with available private material")
        } else {
            print("❌ [\(operation)] No AUTHENTICATION key with available private material")
        }
        return nil
    }

    // Rank candidates by purpose/security in Swift, then delegate the
    // availability decision to the supplied signer. Prefer a CRITICAL auth
    // key, but skip any candidate the signer can't actually sign with, so an
    // identity whose only CRITICAL key is a TRANSFER key (or a CRITICAL auth
    // key with no stored private material) correctly falls through to its
    // HIGH auth key.
    let candidates = km.rankSigningCandidates(
        for: identity,
        purpose: .authentication,
        minimumSecurityLevel: minimumSecurityLevel,
        preferCritical: true
    )
    for k in candidates where signerCanSign(signer, k) {
        if isContractOp {
            print("📝 [\(operation)] Selected CRITICAL AUTHENTICATION key #\(k.id)")
        } else {
            print("📝 [\(operation)] Selected AUTHENTICATION key #\(k.id) (security \(k.securityLevel.name))")
        }
        return k
    }

    if isContractOp {
        print("❌ [\(operation)] No CRITICAL AUTHENTICATION key with available private material")
    } else {
        print("❌ [\(operation)] No AUTHENTICATION key with available private material")
    }
    return nil
}

/// Helper to create a public key handle from an IdentityPublicKey
private func createPublicKeyHandle(from key: IdentityPublicKey, operation: String) -> OpaquePointer? {
    let keyData = key.data
    let keyType = key.keyType.ffiValue
    let purpose = key.purpose.ffiValue
    let securityLevel = key.securityLevel.ffiValue

    let keyResult = keyData.withUnsafeBytes { dataPtr in
        dash_sdk_identity_public_key_create_from_data(
            UInt32(key.id),
            keyType,
            purpose,
            securityLevel,
            dataPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
            UInt(keyData.count),
            key.readOnly,
            key.disabledAt ?? 0
        )
    }

    guard keyResult.error == nil else {
        let errorString = keyResult.error?.pointee.message != nil ?
            String(cString: keyResult.error!.pointee.message) : "Failed to create public key handle"
        print("❌ [\(operation)] Key handle creation failed: \(errorString)")
        dash_sdk_error_free(keyResult.error)
        return nil
    }

    guard let keyHandle = keyResult.data else {
        print("❌ [\(operation)] Invalid public key handle")
        return nil
    }

    print("✅ [\(operation)] Public key handle created from local data")
    return OpaquePointer(keyHandle)
}

// MARK: - State Transition Extensions

@MainActor
extension SDK {

    // MARK: - Helpers (nonisolated)

    /// JSON-encode a contract sub-payload off-actor for FFI hand-off.
    /// Used by `dataContractCreate` to serialize document schemas,
    /// token schemas, groups, keywords, and config — every payload
    /// the FFI accepts as a JSON string. `allowEmpty: false` returns
    /// `nil` for empty containers so callers can drop the parameter
    /// entirely (the FFI treats a missing pointer as "field absent",
    /// which lets the V1 deserializer fall back to its serde default).
    nonisolated fileprivate static func encodeContractField(
        _ value: Any,
        fieldName: String,
        allowEmpty: Bool
    ) throws -> String? {
        if !allowEmpty {
            if let dict = value as? [String: Any], dict.isEmpty { return nil }
            if let arr = value as? [Any], arr.isEmpty { return nil }
        }
        guard JSONSerialization.isValidJSONObject(value) else {
            throw SDKError.serializationError("\(fieldName) is not JSON-serializable")
        }
        guard let data = try? JSONSerialization.data(withJSONObject: value),
              let str = String(data: data, encoding: .utf8) else {
            throw SDKError.serializationError("Failed to serialize \(fieldName)")
        }
        return str
    }

    /// Run `body` with parallel C-string pointers for each input,
    /// where `nil` Swift entries map to NULL pointers. The caller
    /// receives an array `ptrs` whose `ptrs[i]` is either a valid
    /// `UnsafePointer<CChar>` for the duration of the call or
    /// `nil`. Implemented recursively over `inputs.indices` so
    /// every backing `String`'s lifetime extends through the
    /// entire body — six nested `withCString` calls in source form
    /// without the visual nesting.
    nonisolated fileprivate static func withOptionalCStrings<R>(
        _ inputs: [String?],
        _ body: ([UnsafePointer<CChar>?]) -> R
    ) -> R {
        var collected: [UnsafePointer<CChar>?] = Array(repeating: nil, count: inputs.count)
        func step(_ index: Int) -> R {
            if index == inputs.count {
                return body(collected)
            }
            switch inputs[index] {
            case .some(let s):
                return s.withCString { ptr in
                    collected[index] = ptr
                    let result = step(index + 1)
                    collected[index] = nil
                    return result
                }
            case .none:
                return step(index + 1)
            }
        }
        return step(0)
    }

    // MARK: - Identity Handle Management

    /// Convert a DPPIdentity to an identity handle
    /// The returned handle must be freed with dash_sdk_identity_destroy when done
    nonisolated public func identityToHandle(_ identity: DPPIdentity) throws -> OpaquePointer {
        // Convert identity ID to 32-byte array
        let idBytes = identity.id // identity.id is already Data
        guard idBytes.count == 32 else {
            throw SDKError.invalidParameter("Identity ID must be 32 bytes")
        }

        // Convert public keys to C structs
        let publicKeyData = identity.publicKeys.values.compactMap { key -> DashSDKPublicKeyData? in
            let keyData = key.data

            // Map Swift enums to C values
            let purpose = key.purpose.ffiValue
            let securityLevel = key.securityLevel.ffiValue
            let keyType = key.keyType.ffiValue

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
                        let idPtr = OpaquePointer(identityHandle)
                        let infoPtr = dash_sdk_identity_get_info(idPtr)

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
                            dash_sdk_identity_destroy(idPtr)

                            continuation.resume(returning: identityDict)
                        } else {
                            // Destroy the identity handle
                            dash_sdk_identity_destroy(idPtr)
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
        let idBox = SendableOpaque(identity)
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
                                idBox.p,
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
                        let idPtr = OpaquePointer(toppedUpIdentityHandle)
                        let infoPtr = dash_sdk_identity_get_info(idPtr)

                        if let info = infoPtr {
                            let balance = info.pointee.balance

                            // Free the identity info structure
                            dash_sdk_identity_info_free(info)

                            // Destroy the topped up identity handle
                            dash_sdk_identity_destroy(idPtr)

                            continuation.resume(returning: balance)
                        } else {
                            // Destroy the identity handle
                            dash_sdk_identity_destroy(idPtr)
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
        let fromBox = SendableOpaque(fromIdentity)
        let signerBox = SendableOpaque(signer)
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
                        fromBox.p,
                        toIdCStr,
                        amount,
                        publicKeyId,
                        signerBox.p,
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
        let idBox = SendableOpaque(identity)
        let signerBox = SendableOpaque(signer)
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
                        idBox.p,
                        addressCStr,
                        amount,
                        coreFeePerByte,
                        publicKeyId,
                        signerBox.p,
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
        let signerBox = SendableOpaque(signer)
        let startTime = Date()
        print("📝 [DOCUMENT CREATE] Starting at \(startTime)")
        print("📝 [DOCUMENT CREATE] Contract ID: \(contractId)")
        print("📝 [DOCUMENT CREATE] Document Type: \(documentType)")
        print("📝 [DOCUMENT CREATE] Owner ID: \(ownerIdentity.idString)")
        // Pre-serialize properties to avoid capturing non-Sendable dictionary in concurrent closure
        guard let propertiesDataPre = try? JSONSerialization.data(withJSONObject: properties),
              let propertiesJsonPre = String(data: propertiesDataPre, encoding: .utf8) else {
            throw SDKError.invalidParameter("Failed to serialize properties to JSON")
        }

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before dispatching the FFI work off-actor.
        guard let signingKey = selectSigningKey(from: ownerIdentity, operation: "DOCUMENT CREATE", signer: signer) else {
            throw SDKError.invalidParameter("No public key found for identity")
        }

        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    print("❌ [DOCUMENT CREATE] SDK not initialized")
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }

                print("📝 [DOCUMENT CREATE] Converting properties to JSON...")
                let propertiesJson = propertiesJsonPre
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

                // Extract the document handle and entropy from the result
                guard let resultData = createResult.data else {
                    print("❌ [DOCUMENT CREATE] Invalid document result type")
                    continuation.resume(throwing: SDKError.internalError("Invalid document result type"))
                    return
                }

                // Cast the result data to DashSDKDocumentCreateResult pointer
                let createResultPtr = resultData.assumingMemoryBound(to: DashSDKDocumentCreateResult.self)
                let createResultStruct = createResultPtr.pointee
                let documentHandle = createResultStruct.document_handle
                let entropy = createResultStruct.entropy

                // Free the create result structure (but keep the document handle)
                dash_sdk_document_create_result_free(createResultPtr)

                print("✅ [DOCUMENT CREATE] Document handle created with entropy")

                defer {
                    // Clean up document handle when done
                    dash_sdk_document_handle_destroy(documentHandle)
                }

                // 2. Create identity public key handle directly from our local data (no network fetch)
                print("📝 [DOCUMENT CREATE] Getting public key handle...")

                // Key selection happened on the MainActor before dispatch.
                let keyToUse = signingKey

                // Create public key handle
                guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "DOCUMENT CREATE") else {
                    print("⏱️ [DOCUMENT CREATE] Total time before failure: \(Date().timeIntervalSince(startTime)) seconds")
                    continuation.resume(throwing: SDKError.internalError("Failed to create public key handle"))
                    return
                }

                defer {
                    // Clean up key handle
                    dash_sdk_identity_public_key_destroy(keyHandle)
                }

                // 4. Create put settings (null for defaults)
                let putSettings: UnsafePointer<DashSDKPutSettings>? = nil
                let tokenPaymentInfo: UnsafePointer<DashSDKTokenPaymentInfo>? = nil
                let stateTransitionOptions: UnsafePointer<DashSDKStateTransitionCreationOptions>? = nil

                // Use the entropy from document creation (already generated)

                // 5. Put document to platform and wait (using contract ID from trusted context)
                print("🚀 [DOCUMENT CREATE] Submitting document to platform...")
                print("🚀 [DOCUMENT CREATE] This is the NETWORK CALL - using contract from trusted context...")
                let putStart = Date()
                var mutableEntropy = entropy  // Create mutable copy for withUnsafePointer
                let putResult = withUnsafePointer(to: &mutableEntropy) { entropyPtr in
                    contractId.withCString { contractIdCStr in
                        documentType.withCString { docTypeCStr in
                            dash_sdk_document_put_to_platform_and_wait(
                                handle,
                                documentHandle,
                                contractIdCStr,
                                docTypeCStr,
                                entropyPtr,
                                keyHandle,
                                signerBox.p,
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
                    let jsonPtr = jsonData.assumingMemoryBound(to: CChar.self)
                    let jsonString = String(cString: jsonPtr)
                    dash_sdk_string_free(jsonPtr)

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
        let signerBox = SendableOpaque(signer)
        // Pre-serialize properties to avoid capturing non-Sendable dictionary in concurrent closure
        guard let propertiesDataPre = try? JSONSerialization.data(withJSONObject: properties),
              let propertiesJsonPre = String(data: propertiesDataPre, encoding: .utf8) else {
            throw SDKError.invalidParameter("Failed to serialize properties to JSON")
        }
        let startTime = Date()
        print("📝 [DOCUMENT REPLACE] Starting at \(startTime)")
        print("📝 [DOCUMENT REPLACE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before dispatching the FFI work off-actor.
        guard let signingKey = selectSigningKey(from: ownerIdentity, operation: "DOCUMENT REPLACE", signer: signer) else {
            throw SDKError.invalidParameter("No public key found")
        }

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }

                // MARK: - Document Replace
                print("📝 [DOCUMENT REPLACE] Starting at \(Date())...")
                let startTime = Date()

                // 1. Fetch the existing document using the new function
                print("📝 [DOCUMENT REPLACE] Fetching existing document...")
                let fetchStart = Date()

                // First fetch the data contract
                let contractResult = contractId.withCString { contractIdCStr in
                    dash_sdk_data_contract_fetch(handle, contractIdCStr)
                }

                guard contractResult.error == nil,
                      let contractHandle = contractResult.data else {
                    if let error = contractResult.error {
                        let errorMsg = String(cString: error.pointee.message)
                        print("❌ [DOCUMENT REPLACE] Failed to fetch contract: \(errorMsg)")
                        continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    } else {
                        continuation.resume(throwing: SDKError.notFound("Contract not found"))
                    }
                    return
                }

                defer {
                    let dcPtr = OpaquePointer(contractHandle)
                    dash_sdk_data_contract_destroy(dcPtr)
                }

                // Now fetch the document using the contract handle
                let fetchResult = documentType.withCString { docTypeCStr in
                    documentId.withCString { docIdCStr in
                        dash_sdk_document_fetch(
                            handle,
                            OpaquePointer(contractHandle),
                            docTypeCStr,
                            docIdCStr
                        )
                    }
                }

                let fetchTime = Date().timeIntervalSince(fetchStart)
                print("⏱️ [DOCUMENT REPLACE] Document fetch took \(fetchTime) seconds")

                guard fetchResult.error == nil else {
                    let errorString = fetchResult.error?.pointee.message != nil ?
                        String(cString: fetchResult.error!.pointee.message) : "Failed to fetch document"
                    dash_sdk_error_free(fetchResult.error)
                    print("❌ [DOCUMENT REPLACE] Failed to fetch document: \(errorString)")
                    continuation.resume(throwing: SDKError.internalError("Failed to fetch document: \(errorString)"))
                    return
                }

                guard let documentHandle = fetchResult.data else {
                    print("❌ [DOCUMENT REPLACE] Document not found")
                    continuation.resume(throwing: SDKError.notFound("Document not found"))
                    return
                }

                defer {
                    dash_sdk_document_free(OpaquePointer(documentHandle))
                }

                print("✅ [DOCUMENT REPLACE] Document fetched successfully")

                // 2. Update the document properties
                // Use pre-serialized JSON to avoid capturing non-Sendable value types
                let propertiesJson = propertiesJsonPre
                _ = propertiesJson.withCString { propsCStr in
                    dash_sdk_document_set_properties(OpaquePointer(documentHandle), propsCStr)
                }

                // 3. Get appropriate key for signing
                print("📝 [DOCUMENT REPLACE] Getting public key handle...")

                // Key selection happened on the MainActor before dispatch.
                let keyToUse = signingKey

                // Create public key handle
                guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "DOCUMENT REPLACE") else {
                    continuation.resume(throwing: SDKError.internalError("Failed to create public key handle"))
                    return
                }

                defer {
                    dash_sdk_identity_public_key_destroy(keyHandle)
                }

                // 5. Replace document on platform
                print("🚀 [DOCUMENT REPLACE] Submitting document replace to platform...")
                let replaceStart = Date()

                let replaceResult = contractId.withCString { contractIdCStr in
                    documentType.withCString { docTypeCStr in
                            dash_sdk_document_replace_on_platform_and_wait(
                                handle,
                                OpaquePointer(documentHandle),
                                contractIdCStr,
                                docTypeCStr,
                                keyHandle,
                                signerBox.p,
                                nil, // token payment info
                                nil, // put settings
                                nil  // state transition options
                            )
                    }
                }

                let replaceTime = Date().timeIntervalSince(replaceStart)
                print("⏱️ [DOCUMENT REPLACE] Platform submission took \(replaceTime) seconds")

                if let error = replaceResult.error {
                    print("❌ [DOCUMENT REPLACE] Replace failed after \(replaceTime) seconds")
                    let errorString = String(cString: error.pointee.message)
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError(errorString))
                } else if replaceResult.data_type == DashSDKFFI.ResultDocumentHandle,
                          let resultHandle = replaceResult.data {
                    // Document was successfully replaced
                    dash_sdk_document_free(OpaquePointer(resultHandle))

                    let totalTime = Date().timeIntervalSince(startTime)
                    print("✅ [DOCUMENT REPLACE] Document replaced successfully")
                    print("✅ [DOCUMENT REPLACE] Total operation time: \(totalTime) seconds")
                    continuation.resume(returning: ["status": "success", "message": "Document replaced successfully"])
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
        let signerBox = SendableOpaque(signer)
        let startTime = Date()
        print("🗑️ [DOCUMENT DELETE] Starting at \(startTime)")
        print("🗑️ [DOCUMENT DELETE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before dispatching the FFI work off-actor.
        guard let signingKey = selectSigningKey(from: ownerIdentity, operation: "DOCUMENT DELETE", signer: signer) else {
            throw SDKError.protocolError("No suitable key found for signing")
        }

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }

                do {
                    // Prepare C strings
                    guard let documentIdCString = documentId.cString(using: .utf8),
                          let ownerIdCString = ownerIdentity.id.toBase58String().cString(using: .utf8),
                          let contractIdCString = contractId.cString(using: .utf8),
                          let documentTypeCString = documentType.cString(using: .utf8) else {
                        throw SDKError.serializationError("Failed to encode strings to C strings")
                    }

                    // Key selection happened on the MainActor before dispatch.
                    let keyToUse = signingKey

                    // Create public key handle
                    guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "DOCUMENT DELETE") else {
                        throw SDKError.protocolError("Failed to create public key handle")
                    }

                    defer {
                        dash_sdk_identity_public_key_destroy(keyHandle)
                    }

                    // Call the FFI function with network timing
                    let networkStartTime = Date()
                    print("🗑️ [DOCUMENT DELETE] Calling dash_sdk_document_delete_and_wait...")
                    print("🗑️ [DOCUMENT DELETE] Document ID: \(documentId)")
                    print("🗑️ [DOCUMENT DELETE] Owner ID: \(ownerIdentity.id.toBase58String())")

                    let result = dash_sdk_document_delete_and_wait(
                        handle,
                        documentIdCString,
                        ownerIdCString,
                        contractIdCString,
                        documentTypeCString,
                        keyHandle,
                        signerBox.p,
                        nil,  // token_payment_info
                        nil,  // put_settings
                        nil   // state_transition_creation_options
                    )

                    let networkTime = Date().timeIntervalSince(networkStartTime)
                    print("🗑️ [DOCUMENT DELETE] Network call completed in \(networkTime) seconds")

                    if let error = result.error {
                        let errorMessage = String(cString: error.pointee.message)
                        dash_sdk_error_free(error)
                        throw SDKError.protocolError(errorMessage)
                    }

                    let totalTime = Date().timeIntervalSince(startTime)
                    print("✅ [DOCUMENT DELETE] Success! Total time: \(totalTime) seconds")

                    continuation.resume()
                } catch {
                    let totalTime = Date().timeIntervalSince(startTime)
                    print("❌ [DOCUMENT DELETE] Failed after \(totalTime) seconds: \(error)")
                    continuation.resume(throwing: error)
                }
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
        let signerBox = SendableOpaque(signer)
        let startTime = Date()
        print("🔁 [DOCUMENT TRANSFER] Starting at \(startTime)")
        print("🔁 [DOCUMENT TRANSFER] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        print("🔁 [DOCUMENT TRANSFER] From: \(fromIdentity.id.toBase58String()), To: \(toIdentityId)")

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before dispatching the FFI work off-actor.
        guard let signingKey = selectSigningKey(from: fromIdentity, operation: "DOCUMENT TRANSFER", signer: signer) else {
            throw SDKError.invalidParameter("No suitable key found for signing")
        }

        return try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<[String: Any], Error>) in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }

                // Convert strings to C strings
                guard let contractIdCString = contractId.cString(using: .utf8),
                      let documentTypeCString = documentType.cString(using: .utf8),
                      let documentIdCString = documentId.cString(using: .utf8),
                      let toIdentityCString = toIdentityId.cString(using: .utf8) else {
                    continuation.resume(throwing: SDKError.serializationError("Failed to convert strings to C strings"))
                    return
                }

                // Key selection happened on the MainActor before dispatch.
                let keyToUse = signingKey

                // Create public key handle
                guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "DOCUMENT TRANSFER") else {
                    continuation.resume(throwing: SDKError.internalError("Failed to create key handle"))
                    return
                }

                defer {
                    dash_sdk_identity_public_key_destroy(keyHandle)
                }

                print("📝 [DOCUMENT TRANSFER] Step 1: Fetching contract...")
                let contractFetchStartTime = Date()

                // First fetch the data contract
                let contractResult = dash_sdk_data_contract_fetch(handle, contractIdCString)

                guard contractResult.error == nil,
                      let contractHandle = contractResult.data else {
                    if let error = contractResult.error {
                        let errorMsg = String(cString: error.pointee.message)
                        print("❌ [DOCUMENT TRANSFER] Failed to fetch contract: \(errorMsg)")
                        continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    } else {
                        continuation.resume(throwing: SDKError.notFound("Contract not found"))
                    }
                    return
                }

                defer {
                    let dcPtr2 = OpaquePointer(contractHandle)
                    dash_sdk_data_contract_destroy(dcPtr2)
                }

                let contractFetchTime = Date().timeIntervalSince(contractFetchStartTime)
                print("✅ [DOCUMENT TRANSFER] Contract fetched in \(contractFetchTime) seconds")

                print("📝 [DOCUMENT TRANSFER] Step 2: Fetching document...")
                let docFetchStartTime = Date()

                // Now fetch the document using the contract handle
                let fetchResult = dash_sdk_document_fetch(
                    handle,
                    OpaquePointer(contractHandle),
                    documentTypeCString,
                    documentIdCString
                )

                let docFetchTime = Date().timeIntervalSince(docFetchStartTime)
                print("📝 [DOCUMENT TRANSFER] Document fetch took \(docFetchTime) seconds")

                guard fetchResult.error == nil,
                      let documentHandle = fetchResult.data else {
                    let error = fetchResult.error.pointee
                    let errorMsg = String(cString: error.message)
                    print("❌ [DOCUMENT TRANSFER] Failed to fetch document: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }

                defer {
                    dash_sdk_document_destroy(handle, OpaquePointer(documentHandle))
                }

                print("✅ [DOCUMENT TRANSFER] Document fetched successfully")
                print("🔄 [DOCUMENT TRANSFER] Step 3: Creating transfer transition...")

                let transferStartTime = Date()

                // First, try to create the state transition without waiting
                print("🔄 [DOCUMENT TRANSFER] Creating state transition...")
                let transitionResult = dash_sdk_document_transfer_to_identity(
                    handle,
                    OpaquePointer(documentHandle),
                    toIdentityCString,
                    contractIdCString,
                    documentTypeCString,
                    keyHandle,
                    signerBox.p,
                    nil,  // token_payment_info
                    nil,  // put_settings
                    nil   // state_transition_creation_options
                )

                guard transitionResult.error == nil else {
                    let error = transitionResult.error.pointee
                    let errorMsg = String(cString: error.message)
                    print("❌ [DOCUMENT TRANSFER] Failed to create transition: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }


                // Now try the _and_wait version which handles broadcasting internally
                print("🔄 [DOCUMENT TRANSFER] Broadcasting and waiting for confirmation...")
                let result = dash_sdk_document_transfer_to_identity_and_wait(
                    handle,
                    OpaquePointer(documentHandle),
                    toIdentityCString,
                    contractIdCString,
                    documentTypeCString,
                    keyHandle,
                    signerBox.p,
                    nil,  // token_payment_info
                    nil,  // put_settings
                    nil   // state_transition_creation_options
                )

                let transferTime = Date().timeIntervalSince(transferStartTime)
                print("🔄 [DOCUMENT TRANSFER] Transfer operation took \(transferTime) seconds")

                if result.error != nil {
                    let error = result.error.pointee
                    let errorMsg = String(cString: error.message)

                    // Check if it's the "already in chain" error
                    if errorMsg.contains("already in chain") || errorMsg.contains("AlreadyExists") {
                        print("⚠️ [DOCUMENT TRANSFER] State transition already in chain - treating as success")
                        let totalTime = Date().timeIntervalSince(startTime)
                        print("✅ [DOCUMENT TRANSFER] Successfully transferred in \(totalTime) seconds")

                        continuation.resume(returning: [
                            "success": true,
                            "message": "Document transfer already processed",
                            "documentId": documentId,
                            "toIdentity": toIdentityId
                        ])
                        return
                    }

                    print("❌ [DOCUMENT TRANSFER] Broadcast failed: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }

                // Document transfer was successful
                let totalTime = Date().timeIntervalSince(startTime)
                print("✅ [DOCUMENT TRANSFER] Successfully transferred in \(totalTime) seconds")

                // Return a success message
                continuation.resume(returning: [
                    "success": true,
                    "message": "Document successfully transferred",
                    "documentId": documentId,
                    "toIdentity": toIdentityId
                ])
            }
        }
    }

    /// Update document price
    public func documentUpdatePrice(
        contractId: String,
        documentType: String,
        documentId: String,
        newPrice: UInt64,
        ownerIdentity: DPPIdentity,
        signer: OpaquePointer
    ) async throws -> [String: Any] {
        let signerBox = SendableOpaque(signer)
        let startTime = Date()
        print("💰 [DOCUMENT UPDATE PRICE] Starting...")
        print("💰 [DOCUMENT UPDATE PRICE] Contract: \(contractId), Type: \(documentType)")
        print("💰 [DOCUMENT UPDATE PRICE] Document: \(documentId), New Price: \(newPrice)")

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before dispatching the FFI work off-actor.
        guard let signingKey = selectSigningKey(from: ownerIdentity, operation: "UPDATE_PRICE", signer: signer) else {
            throw SDKError.invalidParameter("No suitable signing key found")
        }

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global().async { [weak self] in
                guard let self = self, let handle = self.handle else {
                    continuation.resume(throwing: SDKError.invalidState("SDK not initialized"))
                    return
                }

                // Step 1: Fetch the contract
                print("💰 [DOCUMENT UPDATE PRICE] Step 1: Fetching contract...")
                let contractResult = contractId.withCString { contractIdCStr in
                    dash_sdk_data_contract_fetch(handle, contractIdCStr)
                }

                guard contractResult.error == nil else {
                    let error = contractResult.error.pointee
                    let errorMsg = String(cString: error.message)
                    print("❌ [DOCUMENT UPDATE PRICE] Failed to fetch contract: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }

                guard let contractHandle = contractResult.data else {
                    print("❌ [DOCUMENT UPDATE PRICE] No contract handle returned")
                    continuation.resume(throwing: SDKError.protocolError("No contract handle returned"))
                    return
                }

                defer {
                    let dcPtr3 = OpaquePointer(contractHandle)
                    dash_sdk_data_contract_destroy(dcPtr3)
                }

                // Step 2: Fetch the document
                print("💰 [DOCUMENT UPDATE PRICE] Step 2: Fetching document...")
                let fetchResult = documentType.withCString { docTypeCStr in
                    documentId.withCString { docIdCStr in
                        dash_sdk_document_fetch(
                            handle,
                            OpaquePointer(contractHandle),
                            docTypeCStr,
                            docIdCStr
                        )
                    }
                }

                guard fetchResult.error == nil else {
                    let error = fetchResult.error.pointee
                    let errorMsg = String(cString: error.message)
                    print("❌ [DOCUMENT UPDATE PRICE] Failed to fetch document: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }

                guard let documentHandle = fetchResult.data else {
                    print("❌ [DOCUMENT UPDATE PRICE] No document handle returned")
                    continuation.resume(throwing: SDKError.protocolError("No document handle returned"))
                    return
                }

                defer {
                    dash_sdk_document_destroy(handle, OpaquePointer(documentHandle))
                }

                print("✅ [DOCUMENT UPDATE PRICE] Document fetched successfully")

                // Step 3: Key selection happened on the MainActor before dispatch.
                let keyToUse = signingKey

                guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "UPDATE_PRICE") else {
                    continuation.resume(throwing: SDKError.serializationError("Failed to create key handle"))
                    return
                }

                defer {
                    dash_sdk_identity_public_key_destroy(keyHandle)
                }

                // Step 4: Update price and wait
                print("💰 [DOCUMENT UPDATE PRICE] Step 4: Updating price...")
                let updateResult = contractId.withCString { contractIdCStr in
                    documentType.withCString { documentTypeCStr in
                        dash_sdk_document_update_price_of_document_and_wait(
                            handle,
                            OpaquePointer(documentHandle),
                            contractIdCStr,
                            documentTypeCStr,
                            newPrice,
                            keyHandle,
                            signerBox.p,
                            nil,  // token_payment_info
                            nil,  // put_settings
                            nil   // state_transition_creation_options
                        )
                    }
                }

                if updateResult.error != nil {
                    let error = updateResult.error.pointee
                    let errorMsg = String(cString: error.message)
                    print("❌ [DOCUMENT UPDATE PRICE] Failed: \(errorMsg)")
                    continuation.resume(throwing: SDKError.protocolError(errorMsg))
                    return
                }

                let totalTime = Date().timeIntervalSince(startTime)
                print("✅ [DOCUMENT UPDATE PRICE] Successfully updated in \(totalTime) seconds")

                continuation.resume(returning: [
                    "success": true,
                    "message": "Document price updated successfully",
                    "documentId": documentId,
                    "newPrice": newPrice
                ])
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
        let signerBox = SendableOpaque(signer)
        let startTime = Date()
        print("🛍️ [DOCUMENT PURCHASE] Starting at \(startTime)")
        print("🛍️ [DOCUMENT PURCHASE] Contract: \(contractId), Type: \(documentType), Doc: \(documentId)")
        print("🛍️ [DOCUMENT PURCHASE] Purchaser: \(purchaserIdentity.id.toBase58String()), Price: \(price)")

        guard let handle = self.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Select the signing key on the MainActor (KeyManager is @MainActor)
        // before entering the continuation, for consistency with the other ops.
        guard let signingKey = selectSigningKey(from: purchaserIdentity, operation: "DOCUMENT PURCHASE", signer: signer) else {
            throw SDKError.invalidParameter("No suitable key found for signing")
        }

        return try await withCheckedThrowingContinuation { continuation in
                // Convert strings to C strings
                guard let contractIdCString = contractId.cString(using: .utf8),
                      let documentTypeCString = documentType.cString(using: .utf8),
                      let documentIdCString = documentId.cString(using: .utf8),
                      let purchaserIdCString = purchaserIdentity.id.toBase58String().cString(using: .utf8) else {
                    continuation.resume(throwing: SDKError.serializationError("Failed to convert strings to C strings"))
                    return
                }

                // Key selection happened on the MainActor before the continuation.
                let keyToUse = signingKey

                // Create public key handle
                guard let keyHandle = createPublicKeyHandle(from: keyToUse, operation: "DOCUMENT PURCHASE") else {
                    continuation.resume(throwing: SDKError.internalError("Failed to create key handle"))
                    return
                }

                defer {
                    dash_sdk_identity_public_key_destroy(keyHandle)
                }

                print("📝 [DOCUMENT PURCHASE] Step 1: Fetching contract...")
                let contractFetchStartTime = Date()

                // First fetch the data contract
                let contractResult = dash_sdk_data_contract_fetch(handle, contractIdCString)

                if let error = contractResult.error {
                    let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError("Failed to fetch contract: \(errorMessage)"))
                    return
                }

                guard let contractHandle = contractResult.data else {
                    continuation.resume(throwing: SDKError.notFound("Data contract not found"))
                    return
                }

                defer {
                    dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
                }

                print("📝 [DOCUMENT PURCHASE] Contract fetched in \(Date().timeIntervalSince(contractFetchStartTime)) seconds")

                // Fetch the document to purchase
                print("📝 [DOCUMENT PURCHASE] Step 2: Fetching document...")
                let documentFetchStart = Date()

                let documentResult = dash_sdk_document_fetch(handle, OpaquePointer(contractHandle), documentTypeCString, documentIdCString)

                if let error = documentResult.error {
                    let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
                    dash_sdk_error_free(error)
                    continuation.resume(throwing: SDKError.internalError("Failed to fetch document: \(errorMessage)"))
                    return
                }

                guard let documentHandle = documentResult.data else {
                    continuation.resume(throwing: SDKError.notFound("Document not found"))
                    return
                }

                defer {
                    dash_sdk_document_destroy(handle, OpaquePointer(documentHandle))
                }

                print("📝 [DOCUMENT PURCHASE] Document fetched in \(Date().timeIntervalSince(documentFetchStart)) seconds")

                // Call the document purchase function and broadcast
                print("📝 [DOCUMENT PURCHASE] Step 3: Executing purchase and broadcasting...")
                print("🚀 [DOCUMENT PURCHASE] This will broadcast the state transition to the network")
                let purchaseStartTime = Date()

                let result = dash_sdk_document_purchase_and_wait(
                    handle,
                    OpaquePointer(documentHandle),
                    contractIdCString,
                    documentTypeCString,
                    price,
                    purchaserIdCString,
                    keyHandle,
                    signerBox.p,
                    nil,  // token_payment_info - null for now
                    nil,  // put_settings - null for now
                    nil   // state_transition_creation_options - null for now
                )

                print("📝 [DOCUMENT PURCHASE] Purchase executed in \(Date().timeIntervalSince(purchaseStartTime)) seconds")
                print("📝 [DOCUMENT PURCHASE] Result data type: \(result.data_type)")

                if let error = result.error {
                    let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
                    dash_sdk_error_free(error)

                    print("❌ [DOCUMENT PURCHASE] Failed: \(errorMessage)")
                    let totalTime = Date().timeIntervalSince(startTime)
                    print("❌ [DOCUMENT PURCHASE] Total time: \(totalTime) seconds")

                    continuation.resume(throwing: SDKError.internalError("Document purchase failed: \(errorMessage)"))
                    return
                }

                // The result should contain the purchased document
                if let documentData = result.data {
                    // We received the purchased document back
                    let purchasedDocHandle = OpaquePointer(documentData)

                    // Get info about the purchased document (extract Sendable primitives)
                    var purchasedId: String? = nil
                    var purchasedOwner: String? = nil
                    var purchasedRevision: UInt64 = 0
                    if let info = dash_sdk_document_get_info(purchasedDocHandle) {
                        let docInfo = info.pointee
                        if let idPtr = docInfo.id { purchasedId = String(cString: idPtr) }
                        if let ownerPtr = docInfo.owner_id { purchasedOwner = String(cString: ownerPtr) }
                        purchasedRevision = docInfo.revision
                        dash_sdk_document_info_free(info)
                    }

                    // Clean up the purchased document handle
                    dash_sdk_document_destroy(handle, purchasedDocHandle)

                    let totalTime = Date().timeIntervalSince(startTime)
                    print("✅ [DOCUMENT PURCHASE] Purchase completed and confirmed in \(totalTime) seconds")
                    print("📦 [DOCUMENT PURCHASE] Document successfully purchased and ownership transferred")
                    print("📄 [DOCUMENT PURCHASE] New owner: \(purchasedOwner ?? "unknown")")

                    // Return success with the purchased document info on main actor to avoid sending non-Sendable values
                    DispatchQueue.main.async {
                        let purchasedDocInfo: [String: Any] = [
                            "id": purchasedId as Any,
                            "owner_id": purchasedOwner as Any,
                            "revision": purchasedRevision
                        ]
                        continuation.resume(returning: [
                            "success": true,
                            "message": "Document purchased successfully",
                            "transitionType": "documentPurchase",
                            "contractId": contractId,
                            "documentType": documentType,
                            "documentId": documentId,
                            "price": price,
                            "purchasedDocument": purchasedDocInfo
                        ])
                    }
                } else {
                    print("❌ [DOCUMENT PURCHASE] No data returned from purchase")
                    continuation.resume(throwing: SDKError.internalError("No data returned from document purchase"))
                    return
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                        signerBox.p,
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
                                    signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                        signerBox.p,
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
                                    signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                        signerBox.p,
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
                                    signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                    signerBox.p,
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
                                signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                        signerBox.p,
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
                                    signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                    signerBox.p,
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
                                signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                        signerBox.p,
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
                                    signerBox.p,
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
        let signerBox = SendableOpaque(signer)
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

                let publicKeyHandle = OpaquePointer(keyHandleData)
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
                                    signerBox.p,
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
                                signerBox.p,
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

}

// MARK: - Helper Types

// For now, we'll use the existing SDK types and create type aliases when needed

// MARK: - Convenience Methods with DPPIdentity

@MainActor
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

    nonisolated private func normalizeIdentityId(_ identityId: String) -> String {
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
