import Foundation
import SwiftDashSDK
import DashSDKFFI

// MARK: - Platform Query Extensions for SDK
extension SDK {
    
    // MARK: - Helper Functions
    
    /// Process DashSDKResult and extract JSON
    private func processJSONResult(_ result: DashSDKResult) throws -> [String: Any] {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }
        
        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        
        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse JSON data")
        }
        
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
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch(handle, identityId)
        return try processJSONResult(result)
    }
    
    /// Get identity keys
    public func identityGetKeys(identityId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_public_keys(handle, identityId)
        return try processJSONResult(result)
    }
    
    /// Get identities contract keys
    public func identityGetContractKeys(identityIds: [String], contractId: String, documentType: String?) async throws -> [String: Any] {
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
    public func identityGetByNonUniquePublicKeyHash(publicKeyHash: String) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_identity_fetch_by_non_unique_public_key_hash(handle, publicKeyHash, nil)
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
    public func dataContractGetHistory(id: String, limit: UInt32?, offset: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_data_contract_fetch_history(handle, id, limit ?? 100, offset ?? 0, 0)
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
        limit: UInt32? = nil
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
    
    // MARK: - Voting & Contested Resources Queries
    
    /// Get contested resources
    public func getContestedResources(resourceType: String, limit: UInt32?, offset: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Contested resources are typically tied to a specific contract (like DPNS)
        // For now, we'll use the DPNS contract ID
        let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        
        let result = dash_sdk_contested_resource_get_resources(handle, dpnsContractId, resourceType, nil, nil, nil, limit ?? 100, true)
        return try processJSONArrayResult(result)
    }
    
    /// Get contested resource votes
    public func getContestedResourceVotes(resourceId: String) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // For contested resource votes, we need the contract ID and resource type
        let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
        let resourceType = "domain" // Assuming DPNS domains
        
        // Create JSON array with resourceId as index value
        let indexValues = "[\"\(resourceId)\"]"
        let result = dash_sdk_contested_resource_get_vote_state(handle, dpnsContractId, resourceType, nil, indexValues, 1, true, 100)
        return try processJSONArrayResult(result)
    }
    
    /// Get masternode votes
    public func getMasternodeVotes(masternodeId: String) async throws -> [[String: Any]] {
        // Masternode votes would typically be retrieved via contested resource votes
        // where the voter is the masternode
        throw SDKError.notImplemented("Masternode votes query not yet implemented")
    }
    
    /// Get active proposals
    public func getActiveProposals() async throws -> [[String: Any]] {
        // Proposals would be a specific type of document or contested resource
        throw SDKError.notImplemented("Active proposals query not yet implemented")
    }
    
    /// Get proposal by ID
    public func getProposal(proposalId: String) async throws -> [String: Any] {
        // Proposal would be fetched as a document or contested resource
        throw SDKError.notImplemented("Get proposal query not yet implemented")
    }
    
    // MARK: - Protocol & Version Queries
    
    /// Get protocol version
    public func getProtocolVersion() async throws -> [String: Any] {
        // Protocol version is typically part of the network status
        // For now return static values
        return [
            "version": 1,
            "minVersion": 1
        ]
    }
    
    /// Get version upgrade state
    public func getVersionUpgradeState() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_protocol_version_get_upgrade_state(handle)
        return try processJSONResult(result)
    }
    
    // MARK: - Epoch & Block Queries
    
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
    
    /// Get epoch by index
    public func getEpoch(epochIndex: UInt32) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let epochString = String(epochIndex)
        let result = dash_sdk_system_get_epochs_info(handle, epochString, 1, true)
        let epochs = try processJSONArrayResult(result)
        
        guard let epoch = epochs.first else {
            throw SDKError.notFound("Epoch not found")
        }
        
        return epoch
    }
    
    /// Get best block height
    public func getBestBlockHeight() async throws -> UInt64 {
        // This would typically come from Core chain info, not Platform
        // For now, return a placeholder
        throw SDKError.notImplemented("Best block height requires Core chain query")
    }
    
    /// Get block by height
    public func getBlock(height: UInt64) async throws -> [String: Any] {
        // Block queries are Core chain queries, not Platform
        throw SDKError.notImplemented("Block queries require Core chain access")
    }
    
    /// Get block by hash
    public func getBlockByHash(hash: String) async throws -> [String: Any] {
        // Block queries are Core chain queries, not Platform
        throw SDKError.notImplemented("Block queries require Core chain access")
    }
    
    // MARK: - Token Queries
    
    /// Get identity token balance
    public func getIdentityTokenBalance(identityId: String, tokenId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let tokenIds = "[\"\(tokenId)\"]"
        let result = dash_sdk_token_get_identity_balances(handle, identityId, tokenIds)
        let json = try processJSONResult(result)
        
        guard let balance = json[tokenId] as? UInt64 else {
            throw SDKError.serializationError("Failed to parse token balance")
        }
        
        return balance
    }
    
    /// Get all token balances for identity
    public func getIdentityTokenBalances(identityId: String) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Pass nil to get all token balances
        let result = dash_sdk_identity_fetch_token_balances(handle, identityId, nil)
        let json = try processJSONResult(result)
        
        guard let balances = json as? [String: UInt64] else {
            throw SDKError.serializationError("Failed to parse token balances")
        }
        
        return balances
    }
    
    /// Get token info
    public func getTokenInfo(tokenId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        // Get token contract info first
        let result = dash_sdk_token_get_contract_info(handle, tokenId)
        return try processJSONResult(result)
    }
    
    /// Get token holders
    public func getTokenHolders(tokenId: String, limit: UInt32?, offset: UInt32?) async throws -> [[String: Any]] {
        // Token holders would require querying token balance documents
        throw SDKError.notImplemented("Token holders query not yet implemented")
    }
    
    /// Get total token supply
    public func getTotalTokenSupply(tokenId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_token_get_total_supply(handle, tokenId)
        return try processUInt64Result(result)
    }
    
    // MARK: - Group Queries
    
    /// Get group members
    public func getGroupMembers(groupId: String) async throws -> [[String: Any]] {
        // Groups would be implemented as documents or data contracts
        throw SDKError.notImplemented("Group members query not yet implemented")
    }
    
    /// Get groups for identity
    public func getIdentityGroups(identityId: String) async throws -> [[String: Any]] {
        // Groups would be implemented as documents where member includes identityId
        throw SDKError.notImplemented("Identity groups query not yet implemented")
    }
    
    /// Get group info
    public func getGroupInfo(groupId: String) async throws -> [String: Any] {
        // Group info would be fetched as a document
        throw SDKError.notImplemented("Group info query not yet implemented")
    }
    
    /// Check group membership
    public func checkGroupMembership(groupId: String, identityId: String) async throws -> Bool {
        // Would check if identity is in group members document
        throw SDKError.notImplemented("Group membership check not yet implemented")
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