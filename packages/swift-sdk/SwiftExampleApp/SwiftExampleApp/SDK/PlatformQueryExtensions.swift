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
            DispatchQueue.global(qos: .userInitiated).async {
                print("🔵 SDK.identityGet: On background queue, calling FFI...")
                
                // Create a timeout
                let timeoutWorkItem = DispatchWorkItem {
                    print("❌ SDK.identityGet: FFI call timed out after 30 seconds")
                    continuation.resume(throwing: SDKError.timeout("Identity fetch timed out"))
                }
                DispatchQueue.global().asyncAfter(deadline: .now() + 30, execute: timeoutWorkItem)
                
                // Make the FFI call
                let result = dash_sdk_identity_fetch(handle, identityId)
                
                // Cancel timeout if we got a result
                timeoutWorkItem.cancel()
                
                print("🔵 SDK.identityGet: FFI call returned, processing result...")
                
                do {
                    let jsonResult = try self.processJSONResult(result)
                    print("✅ SDK.identityGet: Successfully processed result")
                    continuation.resume(returning: jsonResult)
                } catch {
                    print("❌ SDK.identityGet: Error processing result: \(error)")
                    continuation.resume(throwing: error)
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
        // This query might not have a direct FFI function yet
        // We'll need to implement it using available functions or create a new FFI function
        throw SDKError.notImplemented("Get identities contract keys not yet implemented in FFI")
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
    
    // MARK: - Data Contract Queries
    
    /// Get a data contract by ID
    public func dataContractGet(id: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_data_contract_fetch(handle, id)
        return try processJSONResult(result)
    }
    
    /// Get data contract history
    public func dataContractGetHistory(id: String, limit: UInt32?, offset: UInt32?, startAtMs: UInt64? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_data_contract_fetch_history(handle, id, limit ?? 100, offset ?? 0, startAtMs ?? 0)
        return try processJSONArrayResult(result)
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
        // Document queries typically require a data contract handle
        // This would need a more complex implementation
        throw SDKError.notImplemented("Document list query requires data contract handle - use SDK documents API instead")
    }
    
    /// Get a specific document
    public func documentGet(dataContractId: String, documentType: String, documentId: String) async throws -> [String: Any] {
        // Document fetch requires a data contract handle
        // For now, we need to fetch the contract first
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
        
        // Now fetch the document
        let contractHandlePtr = OpaquePointer(contractHandle)
        let result = dash_sdk_document_fetch(handle, contractHandlePtr, documentType, documentId)
        
        // Clean up contract handle
        dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        
        return try processJSONResult(result)
    }
    
    // MARK: - DPNS Queries
    
    /// Get DPNS usernames for identity
    public func dpnsGetUsername(identityId: String, limit: UInt32?) async throws -> [[String: Any]] {
        // This would require querying documents of type 'domain' where ownerId = identityId
        // Using the document query system
        throw SDKError.notImplemented("DPNS username query not yet implemented - requires document query")
    }
    
    /// Check DPNS name availability
    public func dpnsCheckAvailability(name: String) async throws -> Bool {
        // Try to resolve the name - if it fails, the name is available
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_resolve_name(handle, name)
        
        if result.error != nil {
            // If we get an error (likely not found), the name is available
            if let error = result.error {
                dash_sdk_error_free(error)
            }
            return true
        }
        
        // If we successfully resolved the name, it's not available
        if let dataPtr = result.data {
            dash_sdk_string_free(dataPtr)
        }
        
        return false
    }
    
    /// Resolve DPNS name to identity ID
    public func dpnsResolve(name: String) async throws -> String {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_resolve_name(handle, name)
        return try processStringResult(result)
    }
    
    /// Search DPNS names by prefix
    public func dpnsSearch(prefix: String, limit: UInt32? = nil) async throws -> [[String: Any]] {
        // DPNS search requires document query with prefix matching
        throw SDKError.notImplemented("DPNS search not yet implemented - requires document query")
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
        // This requires specific FFI implementation for the new parameters
        throw SDKError.notImplemented("Get contested resources with new parameters not yet implemented")
    }
    
    /// Get contested resource vote state
    public func getContestedResourceVoteState(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        resultType: String,
        allowIncludeLockedAndAbstainingVoteTally: Bool,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get contested resource vote state not yet implemented")
    }
    
    /// Get contested resource voters for identity
    public func getContestedResourceVotersForIdentity(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        contestantId: String,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get contested resource voters for identity not yet implemented")
    }
    
    /// Get contested resource identity votes
    public func getContestedResourceIdentityVotes(
        identityId: String,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get contested resource identity votes not yet implemented")
    }
    
    /// Get vote polls by end date
    public func getVotePollsByEndDate(
        startTimeMs: UInt64?,
        endTimeMs: UInt64?,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get vote polls by end date not yet implemented")
    }
    
    
    // MARK: - Protocol & Version Queries
    
    /// Get protocol version upgrade state
    public func getProtocolVersionUpgradeState() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_protocol_version_get_upgrade_state(handle)
        return try processJSONResult(result)
    }
    
    /// Get protocol version upgrade vote status
    public func getProtocolVersionUpgradeVoteStatus(startProTxHash: String?, count: UInt32?) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get protocol version upgrade vote status not yet implemented")
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
        throw SDKError.notImplemented("Get finalized epoch infos not yet implemented")
    }
    
    /// Get evonodes proposed epoch blocks by IDs
    public func getEvonodesProposedEpochBlocksByIds(epoch: UInt32, ids: [String]) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get evonodes proposed epoch blocks by IDs not yet implemented")
    }
    
    /// Get evonodes proposed epoch blocks by range
    public func getEvonodesProposedEpochBlocksByRange(
        epoch: UInt32,
        limit: UInt32?,
        startAfter: String?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get evonodes proposed epoch blocks by range not yet implemented")
    }
    
    // MARK: - Token Queries
    
    /// Get identities token balances
    public func getIdentitiesTokenBalances(identityIds: [String], tokenId: String) async throws -> [String: UInt64] {
        // Would need batch FFI function or iterate through identities
        var balances: [String: UInt64] = [:]
        
        for identityId in identityIds {
            do {
                guard let handle = handle else {
                    throw SDKError.invalidState("SDK not initialized")
                }
                
                let tokenIds = "[\"\(tokenId)\"]"
                let result = dash_sdk_token_get_identity_balances(handle, identityId, tokenIds)
                let json = try processJSONResult(result)
                
                if let balance = json[tokenId] as? UInt64 {
                    balances[identityId] = balance
                }
            } catch {
                // Skip failed fetches
                continue
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
        throw SDKError.notImplemented("Get identity token infos not yet implemented")
    }
    
    /// Get identities token infos
    public func getIdentitiesTokenInfos(identityIds: [String], tokenId: String) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get identities token infos not yet implemented")
    }
    
    /// Get token statuses
    public func getTokenStatuses(tokenIds: [String]) async throws -> [String: Any] {
        throw SDKError.notImplemented("Get token statuses not yet implemented")
    }
    
    /// Get token direct purchase prices
    public func getTokenDirectPurchasePrices(tokenIds: [String]) async throws -> [String: Any] {
        throw SDKError.notImplemented("Get token direct purchase prices not yet implemented")
    }
    
    /// Get token contract info
    public func getTokenContractInfo(dataContractId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_token_get_contract_info(handle, dataContractId)
        return try processJSONResult(result)
    }
    
    /// Get token perpetual distribution last claim
    public func getTokenPerpetualDistributionLastClaim(identityId: String, tokenId: String) async throws -> [String: Any] {
        throw SDKError.notImplemented("Get token perpetual distribution last claim not yet implemented")
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
        throw SDKError.notImplemented("Get group info not yet implemented")
    }
    
    /// Get group infos
    public func getGroupInfos(
        contractId: String,
        startAtGroupContractPosition: UInt32?,
        startGroupContractPositionIncluded: Bool,
        count: UInt32?
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get group infos not yet implemented")
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
        throw SDKError.notImplemented("Get group actions not yet implemented")
    }
    
    /// Get group action signers
    public func getGroupActionSigners(
        contractId: String,
        groupContractPosition: UInt32,
        status: String,
        actionId: String
    ) async throws -> [[String: Any]] {
        throw SDKError.notImplemented("Get group action signers not yet implemented")
    }
    
    // MARK: - System Queries
    
    /// Get platform status
    public func getStatus() async throws -> [String: Any] {
        // Platform status would typically come from a different query
        // For now, return basic status info
        return [
            "version": "1.0.0",
            "network": "testnet",
            "status": "operational"
        ]
    }
    
    /// Get total credits in platform
    public func getTotalCreditsInPlatform() async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_system_get_total_credits_in_platform(handle)
        return try processUInt64Result(result)
    }
}