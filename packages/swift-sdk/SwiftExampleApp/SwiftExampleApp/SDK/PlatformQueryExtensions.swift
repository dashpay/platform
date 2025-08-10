import Foundation
import SwiftDashSDK
import DashSDKFFI

// MARK: - Platform Query Extensions for SDK
extension SDK {
    
    // MARK: - Helper Functions
    
    /// Process DashSDKResult and extract JSON
    private func processJSONResult(_ result: DashSDKResult) throws -> [String: Any] {
        print("🔵 processJSONResult: Processing result...")
        
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            print("❌ processJSONResult: FFI returned error: \(errorMessage)")
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            print("❌ processJSONResult: No data returned from FFI")
            throw SDKError.notFound("No data returned")
        }
        
        // Check if the pointer is null (identity not found)
        if dataPtr == UnsafeMutableRawPointer(bitPattern: 0) {
            print("🔵 processJSONResult: Null pointer returned (identity not found)")
            throw SDKError.notFound("Identity not found")
        }
        
        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        print("🔵 processJSONResult: JSON string: \(jsonString)")
        dash_sdk_string_free(dataPtr)
        
        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            print("❌ processJSONResult: Failed to parse JSON")
            throw SDKError.serializationError("Failed to parse JSON data")
        }
        
        print("✅ processJSONResult: Successfully parsed JSON")
        return json
    }
    
    /// Process DashSDKResult and extract JSON array
    private func processJSONArrayResult(_ result: DashSDKResult) throws -> [[String: Any]] {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            return [] // Empty array
        }
        
        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        
        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            throw SDKError.serializationError("Failed to parse JSON array")
        }
        
        return json
    }
    
    /// Process DashSDKResult and extract string
    private func processStringResult(_ result: DashSDKResult) throws -> String {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }
        
        let string: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        
        return string
    }
    
    /// Process DashSDKResult and extract UInt64
    private func processUInt64Result(_ result: DashSDKResult) throws -> UInt64 {
        let string = try processStringResult(result)
        guard let value = UInt64(string) else {
            throw SDKError.serializationError("Failed to parse UInt64 value")
        }
        return value
    }
    
    // MARK: - Identity Queries
    
    /// Get an identity by ID
    public func identityGet(identityId: String) async throws -> [String: Any] {
        print("🔵 SDK.identityGet: Called with ID: \(identityId)")
        
        guard let handle = handle else {
            print("❌ SDK.identityGet: SDK handle is nil")
            throw SDKError.invalidState("SDK not initialized")
        }
        
        print("🔵 SDK.identityGet: SDK handle exists: \(handle)")
        print("🔵 SDK.identityGet: About to call dash_sdk_identity_fetch with handle: \(handle) and ID: \(identityId)")
        
        // Call the FFI function on a background queue with timeout
        return try await withCheckedThrowingContinuation { continuation in
            // Use a flag to ensure continuation is only resumed once
            let continuationResumed = NSLock()
            var isResumed = false
            
            DispatchQueue.global(qos: .userInitiated).async {
                print("🔵 SDK.identityGet: On background queue, calling FFI...")
                
                // Create a timeout
                let timeoutWorkItem = DispatchWorkItem {
                    continuationResumed.lock()
                    defer { continuationResumed.unlock() }
                    
                    if !isResumed {
                        isResumed = true
                        print("❌ SDK.identityGet: FFI call timed out after 30 seconds")
                        continuation.resume(throwing: SDKError.timeout("Identity fetch timed out"))
                    }
                }
                DispatchQueue.global().asyncAfter(deadline: .now() + 30, execute: timeoutWorkItem)
                
                // Make the FFI call
                let result = dash_sdk_identity_fetch(handle, identityId)
                
                // Cancel timeout if we got a result
                timeoutWorkItem.cancel()
                
                print("🔵 SDK.identityGet: FFI call returned, processing result...")
                
                // Try to resume with the result
                continuationResumed.lock()
                defer { continuationResumed.unlock() }
                
                if !isResumed {
                    isResumed = true
                    do {
                        let jsonResult = try self.processJSONResult(result)
                        print("✅ SDK.identityGet: Successfully processed result")
                        continuation.resume(returning: jsonResult)
                    } catch {
                        print("❌ SDK.identityGet: Error processing result: \(error)")
                        continuation.resume(throwing: error)
                    }
                } else {
                    print("⚠️ SDK.identityGet: Continuation already resumed (likely from timeout), ignoring FFI result")
                }
            }
        }
    }
    
    /// Get identity keys
    public func identityGetKeys(
        identityId: String,
        keyRequestType: String? = nil,
        specificKeyIds: [String]? = nil,
        searchPurposeMap: String? = nil,
        limit: UInt32? = nil,
        offset: UInt32? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // For now, use the simple fetch - would need to implement complex key fetching in FFI
        let result = dash_sdk_identity_fetch_public_keys(handle, identityId)
        return try processJSONResult(result)
    }
    
    /// Get identities contract keys
    public func identityGetContractKeys(
        identityIds: [String],
        contractId: String,
        documentType: String?,
        purposes: [String]? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")
        
        // Convert purposes to comma-separated string (default to all purposes if not specified)
        let purposesStr = purposes?.joined(separator: ",") ?? "0,1,2,3"
        
        let result = dash_sdk_identities_fetch_contract_keys(
            handle,
            identityIdsStr,
            contractId,
            documentType,
            purposesStr
        )
        
        return try processJSONResult(result)
    }
    
    /// Get identity nonce
    public func identityGetNonce(identityId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_nonce(handle, identityId)
        return try processUInt64Result(result)
    }
    
    /// Get identity contract nonce
    public func identityGetContractNonce(identityId: String, contractId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_contract_nonce(handle, identityId, contractId)
        return try processUInt64Result(result)
    }
    
    /// Get identity balance
    public func identityGetBalance(identityId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_balance(handle, identityId)
        return try processUInt64Result(result)
    }
    
    /// Get identities balances
    public func identityGetBalances(identityIds: [String]) async throws -> [String: UInt64] {
        // This would need to call dash_sdk_identity_fetch_balance for each ID
        // or we need a batch FFI function
        var balances: [String: UInt64] = [:]
        
        for identityId in identityIds {
            do {
                let balance = try await identityGetBalance(identityId: identityId)
                balances[identityId] = balance
            } catch {
                // Skip failed fetches
                continue
            }
        }
        
        return balances
    }
    
    /// Get identity balance and revision
    public func identityGetBalanceAndRevision(identityId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_balance_and_revision(handle, identityId)
        return try processJSONResult(result)
    }
    
    /// Get identity by public key hash
    public func identityGetByPublicKeyHash(publicKeyHash: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_by_public_key_hash(handle, publicKeyHash)
        return try processJSONResult(result)
    }
    
    /// Get identities by non-unique public key hash
    public func identityGetByNonUniquePublicKeyHash(publicKeyHash: String, startAfter: String? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_by_non_unique_public_key_hash(handle, publicKeyHash, startAfter)
        return try processJSONArrayResult(result)
    }
    
    // MARK: - Trusted Context Management
    
    /// Add a data contract to the trusted context provider cache
    /// This allows the SDK to use the contract without fetching it from the network
    public func addContractToContext(contractId: String, binaryData: Data) throws {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // The Rust FFI expects comma-separated contract IDs and binary serialized contract data
        let contracts = [(id: contractId, data: binaryData)]
        
        // Use the existing loadKnownContracts function which properly handles binary data
        try loadKnownContracts(contracts)
        
        print("✅ Added contract \(contractId) to trusted context")
    }
    
    // MARK: - Data Contract Queries
    
    /// Get a data contract by ID
    public func dataContractGet(id: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Use the new unified function with return_json = true, return_serialized = false
        let result = id.withCString { idCStr in
            dash_sdk_data_contract_fetch_with_serialization(handle, idCStr, true, false)
        }
        
        // Check for error
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }
        
        // Get the JSON string
        guard result.json_string != nil else {
            throw SDKError.internalError("No JSON data returned from contract fetch")
        }
        
        let jsonString = String(cString: result.json_string!)
        
        // Free the result
        var mutableResult = result
        dash_sdk_data_contract_fetch_result_free(&mutableResult)
        
        // Parse the JSON
        guard let jsonData = jsonString.data(using: .utf8),
              let jsonObject = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse contract JSON")
        }
        
        return jsonObject
    }
    
    /// Get data contract history
    public func dataContractGetHistory(id: String, limit: UInt32?, offset: UInt32?, startAtMs: UInt64? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_data_contract_fetch_history(handle, id, limit ?? 100, offset ?? 0, startAtMs ?? 0)
        
        // The result is a JSON object with an "entries" field containing the array
        let jsonObject = try processJSONResult(result)
        
        // Extract the entries array
        guard let entries = jsonObject["entries"] as? [[String: Any]] else {
            throw SDKError.serializationError("Expected 'entries' array in data contract history response")
        }
        
        return entries
    }
    
    /// Get multiple data contracts
    public func dataContractGetMultiple(ids: [String]) async throws -> [[String: Any]] {
        // Call fetch for each contract ID
        var contracts: [[String: Any]] = []
        
        for id in ids {
            do {
                let contract = try await dataContractGet(id: id)
                contracts.append(contract)
            } catch {
                // Skip failed fetches
                continue
            }
        }
        
        return contracts
    }
    
    // MARK: - Document Queries
    
    /// List documents
    public func documentList(
        dataContractId: String,
        documentType: String,
        whereClause: String? = nil,
        orderByClause: String? = nil,
        limit: UInt32? = nil,
        startAfter: String? = nil,
        startAt: String? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // First fetch the data contract
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }
        
        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }
        
        defer {
            // Clean up contract handle when done
            dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        }
        
        // Create search parameters struct with proper string handling
        let documentTypeCString = documentType.cString(using: .utf8)!
        let whereClauseCString = whereClause?.cString(using: .utf8)
        let orderByClauseCString = orderByClause?.cString(using: .utf8)
        
        let result = documentTypeCString.withUnsafeBufferPointer { documentTypePtr in
            if let whereClause = whereClauseCString {
                return whereClause.withUnsafeBufferPointer { wherePtr in
                    if let orderByClause = orderByClauseCString {
                        return orderByClause.withUnsafeBufferPointer { orderByPtr in
                            var searchParams = DashSDKDocumentSearchParams()
                            searchParams.data_contract_handle = OpaquePointer(contractHandle)
                            searchParams.document_type = documentTypePtr.baseAddress
                            searchParams.where_json = wherePtr.baseAddress
                            searchParams.order_by_json = orderByPtr.baseAddress
                            searchParams.limit = limit ?? 100
                            // Handle pagination - startAt takes precedence over startAfter
                            if let startAt = startAt {
                                // startAt is inclusive - start from this exact position
                                searchParams.start_at = UInt32(startAt) ?? 0
                            } else if let startAfter = startAfter {
                                // startAfter is exclusive - start from the next position
                                searchParams.start_at = (UInt32(startAfter) ?? 0) + 1
                            } else {
                                searchParams.start_at = 0
                            }
                            
                            return dash_sdk_document_search(handle, &searchParams)
                        }
                    } else {
                        var searchParams = DashSDKDocumentSearchParams()
                        searchParams.data_contract_handle = OpaquePointer(contractHandle)
                        searchParams.document_type = documentTypePtr.baseAddress
                        searchParams.where_json = wherePtr.baseAddress
                        searchParams.order_by_json = nil
                        searchParams.limit = limit ?? 100
                        // Handle pagination - startAt takes precedence over startAfter
                        if let startAt = startAt {
                            // startAt is inclusive - start from this exact position
                            searchParams.start_at = UInt32(startAt) ?? 0
                        } else if let startAfter = startAfter {
                            // startAfter is exclusive - start from the next position
                            searchParams.start_at = (UInt32(startAfter) ?? 0) + 1
                        } else {
                            searchParams.start_at = 0
                        }
                        
                        return dash_sdk_document_search(handle, &searchParams)
                    }
                }
            } else {
                var searchParams = DashSDKDocumentSearchParams()
                searchParams.data_contract_handle = OpaquePointer(contractHandle)
                searchParams.document_type = documentTypePtr.baseAddress
                searchParams.where_json = nil
                searchParams.order_by_json = nil
                searchParams.limit = limit ?? 100
                searchParams.start_at = 0
                
                return dash_sdk_document_search(handle, &searchParams)
            }
        }
        
        return try processJSONResult(result)
    }
    
    /// Get a specific document
    public func documentGet(dataContractId: String, documentType: String, documentId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // First fetch the data contract
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }
        
        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }
        
        defer {
            // Clean up contract handle when done
            dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        }
        
        // Now fetch the document
        let documentResult = dash_sdk_document_fetch(handle, OpaquePointer(contractHandle), documentType, documentId)
        
        if let error = documentResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch document: \(errorMessage)")
        }
        
        guard let documentHandle = documentResult.data else {
            throw SDKError.notFound("Document not found")
        }
        
        defer {
            // Clean up document handle
            dash_sdk_document_destroy(handle, OpaquePointer(documentHandle))
        }
        
        // Get document info to convert to JSON
        let info = dash_sdk_document_get_info(OpaquePointer(documentHandle))
        defer {
            if let info = info {
                dash_sdk_document_info_free(info)
            }
        }
        
        guard let infoPtr = info else {
            throw SDKError.internalError("Failed to get document info")
        }
        
        // Convert document info to dictionary
        let documentInfo = infoPtr.pointee
        
        // Build JSON representation from document info fields
        let json: [String: Any] = [
            "id": documentInfo.id != nil ? String(cString: documentInfo.id!) : "",
            "ownerId": documentInfo.owner_id != nil ? String(cString: documentInfo.owner_id!) : "",
            "dataContractId": documentInfo.data_contract_id != nil ? String(cString: documentInfo.data_contract_id!) : "",
            "documentType": documentInfo.document_type != nil ? String(cString: documentInfo.document_type!) : "",
            "revision": documentInfo.revision,
            "createdAt": documentInfo.created_at,
            "updatedAt": documentInfo.updated_at
        ]
        
        return json
    }
    
    // MARK: - DPNS Queries
    
    /// Get DPNS usernames for identity
    public func dpnsGetUsername(identityId: String, limit: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function with identity ID as string
        let result = dash_sdk_dpns_get_usernames(handle, identityId, limit ?? 10)
        
        return try processJSONArrayResult(result)
    }
    
    /// Check DPNS name availability
    public func dpnsCheckAvailability(name: String) async throws -> Bool {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function
        let result = dash_sdk_dpns_check_availability(handle, name)
        
        // Process the result to get the availability info
        let json = try processJSONResult(result)
        
        // Extract the "available" boolean from the result
        guard let isAvailable = json["available"] as? Bool else {
            throw SDKError.serializationError("Failed to parse availability result")
        }
        
        return isAvailable
    }
    
    /// Get non-resolved DPNS contests for a specific identity
    public func dpnsGetNonResolvedContestsForIdentity(identityId: String, limit: UInt32?) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function which now returns a pointer to DashSDKContestedNamesList
        guard let contestedNamesListPtr = dash_sdk_dpns_get_non_resolved_contests_for_identity(handle, identityId, limit ?? 20) else {
            throw SDKError.internalError("Failed to get contested names")
        }
        
        defer {
            // Free the C structure when done
            dash_sdk_contested_names_list_free(contestedNamesListPtr)
        }
        
        // Convert C structure to Swift dictionary
        let contestedNamesList = contestedNamesListPtr.pointee
        var result: [String: Any] = [:]
        
        if contestedNamesList.count > 0 && contestedNamesList.names != nil {
            for i in 0..<contestedNamesList.count {
                let contestedName = contestedNamesList.names![Int(i)]
                
                // Get the name
                guard let namePtr = contestedName.name else { continue }
                let name = String(cString: namePtr)
                
                // Parse contest info
                var contestInfo: [String: Any] = [:]
                let contest = contestedName.contest_info
                
                // Add end time
                contestInfo["endTime"] = contest.end_time
                contestInfo["hasWinner"] = contest.has_winner
                
                // Add vote tallies
                contestInfo["abstainVotes"] = contest.abstain_votes
                contestInfo["lockVotes"] = contest.lock_votes
                
                // Parse contenders
                var contenders: [[String: Any]] = []
                if contest.contender_count > 0 && contest.contenders != nil {
                    for j in 0..<contest.contender_count {
                        let contender = contest.contenders![Int(j)]
                        
                        var contenderDict: [String: Any] = [:]
                        if let idPtr = contender.identity_id {
                            contenderDict["identifier"] = String(cString: idPtr)
                        }
                        contenderDict["votes"] = "ResourceVote { vote_choice: TowardsIdentity, strength: \(contender.vote_count) }"
                        
                        contenders.append(contenderDict)
                    }
                }
                contestInfo["contenders"] = contenders
                
                result[name] = contestInfo
            }
        }
        
        return result
    }
    
    /// Get current DPNS contests (active vote polls)
    public func dpnsGetCurrentContests(startTime: UInt64 = 0, endTime: UInt64 = 0, limit: UInt16 = 100) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function which returns a pointer to DashSDKNameTimestampList
        guard let nameTimestampListPtr = dash_sdk_dpns_get_current_contests(handle, startTime, endTime, limit) else {
            throw SDKError.internalError("Failed to get current contests")
        }
        
        defer {
            // Free the C structure when done
            dash_sdk_name_timestamp_list_free(nameTimestampListPtr)
        }
        
        // Convert C structure to Swift dictionary
        let nameTimestampList = nameTimestampListPtr.pointee
        var result: [String: UInt64] = [:]
        
        if nameTimestampList.count > 0 && nameTimestampList.entries != nil {
            for i in 0..<nameTimestampList.count {
                let entry = nameTimestampList.entries![Int(i)]
                
                guard let namePtr = entry.name else { continue }
                let name = String(cString: namePtr)
                result[name] = entry.end_time
            }
        }
        
        return result
    }
    
    /// Get the vote state for a contested DPNS username
    public func dpnsGetContestedVoteState(name: String, limit: UInt32 = 100) async throws -> [String: Any] {
        guard let handle = self.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = await withCheckedContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                let result = name.withCString { namePtr in
                    dash_sdk_dpns_get_contested_vote_state(handle, namePtr, limit)
                }
                continuation.resume(returning: result)
            }
        }
        
        // Check for error
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        // Parse the JSON result
        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }
        
        let jsonString = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
        
        guard let jsonData = jsonString.data(using: .utf8),
              let voteState = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse vote state JSON")
        }
        
        return voteState
    }
    
    /// Get contested DPNS usernames that are not yet resolved
    public func dpnsGetContestedNonResolvedUsernames(limit: UInt32 = 100) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function which returns a pointer to DashSDKContestedNamesList
        guard let contestedNamesListPtr = dash_sdk_dpns_get_contested_non_resolved_usernames(handle, limit) else {
            throw SDKError.internalError("Failed to get contested names")
        }
        
        defer {
            // Free the C structure when done
            dash_sdk_contested_names_list_free(contestedNamesListPtr)
        }
        
        // Convert C structure to Swift dictionary
        let contestedNamesList = contestedNamesListPtr.pointee
        var result: [String: Any] = [:]
        
        if contestedNamesList.count > 0 && contestedNamesList.names != nil {
            for i in 0..<contestedNamesList.count {
                let contestedName = contestedNamesList.names![Int(i)]
                
                // Get the name
                guard let namePtr = contestedName.name else { continue }
                let name = String(cString: namePtr)
                
                // Parse contest info
                var contestInfo: [String: Any] = [:]
                let contest = contestedName.contest_info
                
                // Add end time
                contestInfo["endTime"] = contest.end_time
                contestInfo["hasWinner"] = contest.has_winner
                
                // Add vote tallies
                contestInfo["abstainVotes"] = contest.abstain_votes
                contestInfo["lockVotes"] = contest.lock_votes
                
                // Parse contenders
                var contenders: [[String: Any]] = []
                if contest.contender_count > 0 && contest.contenders != nil {
                    for j in 0..<contest.contender_count {
                        let contender = contest.contenders![Int(j)]
                        
                        var contenderDict: [String: Any] = [:]
                        if let idPtr = contender.identity_id {
                            contenderDict["identifier"] = String(cString: idPtr)
                        }
                        contenderDict["votes"] = "ResourceVote { vote_choice: TowardsIdentity, strength: \(contender.vote_count) }"
                        
                        contenders.append(contenderDict)
                    }
                }
                contestInfo["contenders"] = contenders
                
                result[name] = contestInfo
            }
        }
        
        return result
    }
    
    /// Resolve DPNS name to identity ID
    public func dpnsResolve(name: String) async throws -> String {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_resolve_name(handle, name)
        
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.notFound("Name not found")
        }
        
        // Cast to DashSDKBinaryData to get the binary identity ID
        let binaryData = dataPtr.assumingMemoryBound(to: DashSDKBinaryData.self).pointee
        
        // Convert the 32-byte identity ID to hex string
        let identityIdData = Data(bytes: binaryData.data, count: Int(binaryData.len))
        let identityIdHex = identityIdData.toHexString()
        
        // Free the binary data
        dash_sdk_binary_data_free(dataPtr.assumingMemoryBound(to: DashSDKBinaryData.self))
        
        return identityIdHex
    }
    
    /// Search DPNS names by prefix
    public func dpnsSearch(prefix: String, limit: UInt32? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Call native FFI function
        let result = dash_sdk_dpns_search(handle, prefix, limit ?? 10)
        
        return try processJSONArrayResult(result)
    }
    
    // MARK: - Voting & Contested Resources Queries
    
    /// Get contested resources
    public func getContestedResources(
        documentTypeName: String,
        dataContractId: String,
        indexName: String,
        resultType: String,
        allowIncludeLockedAndAbstainingVoteTally: Bool,
        startAtValue: String?,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_contested_resource_get_resources(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            startAtValue,
            nil, // end_index_values_json
            limit ?? 100,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get contested resource vote state
    public func getContestedResourceVoteState(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: [String]? = nil,
        resultType: String,
        allowIncludeLockedAndAbstainingVoteTally: Bool,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Convert result type to integer
        let resultTypeInt: UInt8 = switch resultType {
        case "contenders": 0
        case "abstainers": 1
        case "locked": 2
        default: 0
        }
        
        // Create index values JSON array
        let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues ?? [])
        let indexValuesJson = String(data: indexValuesData, encoding: .utf8) ?? "[]"
        
        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            indexValuesJson,
            resultTypeInt,
            allowIncludeLockedAndAbstainingVoteTally,
            count ?? 100
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get contested resource voters for identity
    public func getContestedResourceVotersForIdentity(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: [String]? = nil,
        contestantId: String,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Create index values JSON array
        let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues ?? [])
        let indexValuesJson = String(data: indexValuesData, encoding: .utf8) ?? "[]"
        
        let result = dash_sdk_contested_resource_get_voters_for_identity(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            indexValuesJson,
            contestantId,
            count ?? 100,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get contested resource identity votes
    public func getContestedResourceIdentityVotes(
        identityId: String,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_contested_resource_get_identity_votes(
            handle,
            identityId,
            limit ?? 100,
            offset ?? 0,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get vote polls by end date
    public func getVotePollsByEndDate(
        startTimeMs: UInt64?,
        endTimeMs: UInt64?,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_voting_get_vote_polls_by_end_date(
            handle,
            startTimeMs ?? 0,
            true, // start_time_included
            endTimeMs ?? UInt64.max,
            true, // end_time_included
            limit ?? 100,
            offset ?? 0,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }
    
    
    // MARK: - Protocol & Version Queries
    
    /// Get protocol version upgrade state
    public func getProtocolVersionUpgradeState() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_protocol_version_get_upgrade_state(handle)
        
        // Special handling for protocol version upgrade state which returns an array
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        // If no data, return empty result
        guard let dataPtr = result.data else {
            return ["upgrades": []]
        }
        
        let jsonArray = try processJSONArrayResult(result)
        return ["upgrades": jsonArray]
    }
    
    /// Get protocol version upgrade vote status
    public func getProtocolVersionUpgradeVoteStatus(startProTxHash: String?, count: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_protocol_version_get_upgrade_vote_status(handle, startProTxHash, count ?? 100)
        return try processJSONArrayResult(result)
    }
    
    // MARK: - Epoch & Block Queries
    
    /// Get epochs info
    public func getEpochsInfo(startEpoch: UInt32?, count: UInt32?, ascending: Bool) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let startEpochString = startEpoch.map { String($0) }
        let result = dash_sdk_system_get_epochs_info(handle, startEpochString, count ?? 100, ascending)
        return try processJSONArrayResult(result)
    }
    
    /// Get current epoch
    public func getCurrentEpoch() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Get current epoch info by passing nil as start_epoch to get the latest
        let result = dash_sdk_system_get_epochs_info(handle, nil, 1, true)
        let epochs = try processJSONArrayResult(result)
        
        guard let currentEpoch = epochs.first else {
            throw SDKError.notFound("Current epoch not found")
        }
        
        return currentEpoch
    }
    
    /// Get finalized epoch infos
    public func getFinalizedEpochInfos(startEpoch: UInt32?, count: UInt32?, ascending: Bool) async throws -> [[String: Any]] {
        // For now, use getEpochsInfo as they might be the same
        // The FFI might need a separate function for finalized epochs only
        return try await getEpochsInfo(startEpoch: startEpoch, count: count, ascending: ascending)
    }
    
    /// Get evonodes proposed epoch blocks by IDs
    public func getEvonodesProposedEpochBlocksByIds(epoch: UInt32, ids: [String]) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Convert IDs array to JSON
        let idsData = try JSONSerialization.data(withJSONObject: ids)
        let idsStr = String(data: idsData, encoding: .utf8) ?? "[]"
        
        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(handle, epoch, idsStr)
        return try processJSONArrayResult(result)
    }
    
    /// Get evonodes proposed epoch blocks by range
    public func getEvonodesProposedEpochBlocksByRange(
        epoch: UInt32,
        limit: UInt32?,
        startAfter: String?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
            handle,
            epoch,
            UInt32(limit ?? 100),
            startAfter,
            nil  // start_at parameter - not used in this implementation
        )
        return try processJSONArrayResult(result)
    }
    
    // MARK: - Token Queries
    
    /// Get identity token balances - get balances for multiple tokens for a single identity
    public func getIdentityTokenBalances(identityId: String, tokenIds: [String]) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")
        
        let result = dash_sdk_token_get_identity_balances(handle, identityId, tokenIdsStr)
        let json = try processJSONResult(result)
        
        // Convert JSON object to [String: UInt64]
        var balances: [String: UInt64] = [:]
        if let dict = json as? [String: Any] {
            for (tokenId, balance) in dict {
                if let balanceNum = balance as? NSNumber {
                    balances[tokenId] = balanceNum.uint64Value
                }
            }
        }
        
        return balances
    }
    
    /// Get identities token balances
    public func getIdentitiesTokenBalances(identityIds: [String], tokenId: String) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")
        
        let result = dash_sdk_identities_fetch_token_balances(handle, identityIdsStr, tokenId)
        let json = try processJSONResult(result)
        
        // Convert the result to [String: UInt64]
        var balances: [String: UInt64] = [:]
        for (key, value) in json {
            if let balance = value as? UInt64 {
                balances[key] = balance
            }
        }
        
        return balances
    }
    
    /// Get identity token infos
    public func getIdentityTokenInfos(
        identityId: String,
        tokenIds: [String]?,
        limit: UInt32?,
        offset: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Convert token IDs to comma-separated string or nil
        let tokenIdsStr = tokenIds?.joined(separator: ",")
        
        let result = dash_sdk_identity_fetch_token_infos(handle, identityId, tokenIdsStr)
        return try processJSONArrayResult(result)
    }
    
    /// Get identities token infos
    public func getIdentitiesTokenInfos(identityIds: [String], tokenId: String) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")
        
        let result = dash_sdk_identities_fetch_token_infos(handle, identityIdsStr, tokenId)
        return try processJSONArrayResult(result)
    }
    
    /// Get token statuses
    public func getTokenStatuses(tokenIds: [String]) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")
        
        let result = dash_sdk_token_get_statuses(handle, tokenIdsStr)
        return try processJSONResult(result)
    }
    
    /// Get token direct purchase prices
    public func getTokenDirectPurchasePrices(tokenIds: [String]) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")
        
        let result = dash_sdk_token_get_direct_purchase_prices(handle, tokenIdsStr)
        return try processJSONResult(result)
    }
    
    /// Get token contract info
    public func getTokenContractInfo(tokenId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_token_get_contract_info(handle, tokenId)
        return try processJSONResult(result)
    }
    
    /// Get token perpetual distribution last claim
    public func getTokenPerpetualDistributionLastClaim(identityId: String, tokenId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_token_get_perpetual_distribution_last_claim(handle, tokenId, identityId)
        
        // Special handling for this query - null means no claim found
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            // No claim found - return empty dictionary
            return [:]
        }
        
        // Check if the pointer is null (no claim found)
        if dataPtr == UnsafeMutableRawPointer(bitPattern: 0) {
            return [:]
        }
        
        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        
        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse JSON data")
        }
        
        return json
    }
    
    /// Get token total supply
    public func getTokenTotalSupply(tokenId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_token_get_total_supply(handle, tokenId)
        return try processUInt64Result(result)
    }
    
    // MARK: - Group Queries
    
    /// Get group info
    public func getGroupInfo(contractId: String, groupContractPosition: UInt32) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_group_get_info(handle, contractId, UInt16(groupContractPosition))
        return try processJSONResult(result)
    }
    
    /// Get group infos
    public func getGroupInfos(
        contractId: String,
        startAtGroupContractPosition: UInt32?,
        startGroupContractPositionIncluded: Bool,
        count: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_group_get_infos(
            handle,
            startAtGroupContractPosition.map { String($0) },  // Convert UInt32 to String
            UInt32(count ?? 100)
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get group actions
    public func getGroupActions(
        contractId: String,
        groupContractPosition: UInt32,
        status: String,
        startActionId: String?,
        startActionIdIncluded: Bool,
        count: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Convert status string to enum value
        let statusValue: UInt8 = status == "ACTIVE" ? 0 : 1
        
        let result = dash_sdk_group_get_actions(
            handle,
            contractId,
            UInt16(groupContractPosition),
            statusValue,
            startActionId,  // Pass the string directly
            UInt16(count ?? 100)
        )
        return try processJSONArrayResult(result)
    }
    
    /// Get group action signers
    public func getGroupActionSigners(
        contractId: String,
        groupContractPosition: UInt32,
        status: String,
        actionId: String
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Convert status string to enum value
        let statusValue: UInt8 = status == "ACTIVE" ? 0 : 1
        
        let result = dash_sdk_group_get_action_signers(
            handle,
            contractId,
            UInt16(groupContractPosition),
            statusValue,
            actionId
        )
        return try processJSONArrayResult(result)
    }
    
    // MARK: - System Queries
    
    /// Get platform status
    public func getStatus() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_get_platform_status(handle)
        return try processJSONResult(result)
    }
    
    /// Get total credits in platform
    public func getTotalCreditsInPlatform() async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_system_get_total_credits_in_platform(handle)
        return try processUInt64Result(result)
    }
    
    /// Get current quorums info
    public func getCurrentQuorumsInfo() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_system_get_current_quorums_info(handle)
        return try processJSONResult(result)
    }
    
    /// Get prefunded specialized balance
    public func getPrefundedSpecializedBalance(id: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_system_get_prefunded_specialized_balance(handle, id)
        return try processUInt64Result(result)
    }
}