import Foundation
import SwiftData
import SwiftDashSDK

/// Service to manage SwiftData operations for the app
@MainActor
final class DataManager: ObservableObject {
    private let modelContext: ModelContext
    var currentNetwork: Network
    
    init(modelContext: ModelContext, currentNetwork: Network = .testnet) {
        self.modelContext = modelContext
        self.currentNetwork = currentNetwork
    }
    
    // MARK: - Identity Operations
    
    /// Save or update an identity
    func saveIdentity(_ identity: IdentityModel) throws {
        // Check if identity already exists
        let predicate = PersistentIdentity.predicate(identityId: identity.id)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)
        
        if let existingIdentity = try modelContext.fetch(descriptor).first {
            // Update existing identity
            existingIdentity.balance = Int64(identity.balance)
            existingIdentity.alias = identity.alias
            existingIdentity.dpnsName = identity.dpnsName
            existingIdentity.mainDpnsName = identity.mainDpnsName
            existingIdentity.isLocal = identity.isLocal
            // Update public keys
            existingIdentity.publicKeys.removeAll()
            for publicKey in identity.publicKeys {
                if let persistentKey = PersistentPublicKey.from(publicKey, identityId: identity.idString) {
                    existingIdentity.addPublicKey(persistentKey)
                }
            }
            
            // Handle private keys - match them to their corresponding public keys using cryptographic validation
            for privateKeyData in identity.privateKeys {
                // Find which public key this private key corresponds to
                if let matchingPublicKey = KeyValidation.matchPrivateKeyToPublicKeys(
                    privateKeyData: privateKeyData,
                    publicKeys: identity.publicKeys,
                    isTestnet: currentNetwork == .testnet
                ) {
                    // Find the corresponding persistent public key
                    if let persistentKey = existingIdentity.publicKeys.first(where: { $0.keyId == matchingPublicKey.id }) {
                        // Store the private key for this specific public key
                        if let keychainId = KeychainManager.shared.storePrivateKey(privateKeyData, identityId: identity.id, keyIndex: persistentKey.keyId) {
                            persistentKey.privateKeyKeychainIdentifier = keychainId
                        }
                    }
                }
            }
            
            // Update special keys
            if let votingKey = identity.votingPrivateKey {
                existingIdentity.votingPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(votingKey, identityId: identity.id, keyType: .voting)
            }
            if let ownerKey = identity.ownerPrivateKey {
                existingIdentity.ownerPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(ownerKey, identityId: identity.id, keyType: .owner)
            }
            if let payoutKey = identity.payoutPrivateKey {
                existingIdentity.payoutPrivateKeyIdentifier = KeychainManager.shared.storeSpecialKey(payoutKey, identityId: identity.id, keyType: .payout)
            }
            existingIdentity.lastUpdated = Date()
        } else {
            // Create new identity
            let persistentIdentity = PersistentIdentity.from(identity, network: currentNetwork.rawValue)
            modelContext.insert(persistentIdentity)
        }
        
        try modelContext.save()
    }
    
    /// Fetch all identities for current network
    func fetchIdentities() throws -> [IdentityModel] {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: PersistentIdentity.predicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }
    
    /// Fetch local identities only
    func fetchLocalIdentities() throws -> [IdentityModel] {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: PersistentIdentity.localIdentitiesPredicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }
    
    /// Delete an identity
    func deleteIdentity(withId identityId: Data) throws {
        let predicate = PersistentIdentity.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)
        
        if let identity = try modelContext.fetch(descriptor).first {
            modelContext.delete(identity)
            try modelContext.save()
        }
    }
    
    // MARK: - Document Operations
    
    /// Save or update a document
    func saveDocument(_ document: DocumentModel) throws {
        let predicate = PersistentDocument.predicate(documentId: document.id)
        let descriptor = FetchDescriptor<PersistentDocument>(predicate: predicate)
        
        if let existingDocument = try modelContext.fetch(descriptor).first {
            // Update existing document
            let dataToStore = (try? JSONSerialization.data(withJSONObject: document.data, options: [])) ?? Data()
            existingDocument.updateProperties(dataToStore)
            existingDocument.updateRevision(Int64(document.revision))
        } else {
            // Create new document
            let persistentDocument = PersistentDocument.from(document)
            modelContext.insert(persistentDocument)
            
            // Link to local identity if the owner is local
            persistentDocument.linkToLocalIdentityIfNeeded(in: modelContext)
        }
        
        try modelContext.save()
    }
    
    /// Fetch documents for a contract
    func fetchDocuments(contractId: String) throws -> [DocumentModel] {
        let predicate = PersistentDocument.predicate(contractId: contractId, network: currentNetwork.rawValue)
        let descriptor = FetchDescriptor<PersistentDocument>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentDocuments = try modelContext.fetch(descriptor)
        return persistentDocuments.map { $0.toDocumentModel() }
    }
    
    /// Fetch documents owned by an identity
    func fetchDocuments(ownerId: Data) throws -> [DocumentModel] {
        let predicate = PersistentDocument.predicate(ownerId: ownerId)
        let descriptor = FetchDescriptor<PersistentDocument>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentDocuments = try modelContext.fetch(descriptor)
        return persistentDocuments.map { $0.toDocumentModel() }
    }
    
    /// Delete a document
    func deleteDocument(withId documentId: String) throws {
        let predicate = PersistentDocument.predicate(documentId: documentId)
        let descriptor = FetchDescriptor<PersistentDocument>(predicate: predicate)
        
        if let document = try modelContext.fetch(descriptor).first {
            document.markAsDeleted()
            try modelContext.save()
        }
    }
    
    // MARK: - Contract Operations
    
    /// Save or update a contract
    func saveContract(_ contract: ContractModel) throws {
        let predicate = PersistentDataContract.predicate(contractId: contract.id)
        let descriptor = FetchDescriptor<PersistentDataContract>(predicate: predicate)
        
        if let existingContract = try modelContext.fetch(descriptor).first {
            // Update existing contract
            existingContract.name = contract.name
            existingContract.updateVersion(contract.version)
            existingContract.schema = contract.schema
            existingContract.documentTypesList = contract.documentTypes
            // Update keywords by recreating relations
            existingContract.keywordRelations = contract.keywords.map { 
                PersistentKeyword(keyword: $0, contractId: existingContract.idBase58) 
            }
            existingContract.contractDescription = contract.description
        } else {
            // Create new contract
            let persistentContract = PersistentDataContract.from(contract)
            modelContext.insert(persistentContract)
        }
        
        try modelContext.save()
    }
    
    /// Fetch all contracts for current network
    func fetchContracts() throws -> [ContractModel] {
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: PersistentDataContract.predicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentContracts = try modelContext.fetch(descriptor)
        return persistentContracts.map { $0.toContractModel() }
    }
    
    /// Fetch contracts with tokens
    func fetchContractsWithTokens() throws -> [ContractModel] {
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: PersistentDataContract.contractsWithTokensPredicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentContracts = try modelContext.fetch(descriptor)
        return persistentContracts.map { $0.toContractModel() }
    }
    
    // MARK: - Token Balance Operations
    
    /// Save or update a token balance
    func saveTokenBalance(tokenId: String, identityId: Data, balance: UInt64, frozen: Bool = false, tokenInfo: (name: String, symbol: String, decimals: Int32)? = nil) throws {
        let predicate = PersistentTokenBalance.predicate(tokenId: tokenId, identityId: identityId)
        let descriptor = FetchDescriptor<PersistentTokenBalance>(predicate: predicate)
        
        if let existingBalance = try modelContext.fetch(descriptor).first {
            // Update existing balance
            existingBalance.updateBalance(Int64(balance))
            if frozen != existingBalance.frozen {
                if frozen {
                    existingBalance.freeze()
                } else {
                    existingBalance.unfreeze()
                }
            }
            if let info = tokenInfo {
                existingBalance.updateTokenInfo(name: info.name, symbol: info.symbol, decimals: info.decimals)
            }
        } else {
            // Create new balance
            let persistentBalance = PersistentTokenBalance(
                tokenId: tokenId,
                identityId: identityId,
                balance: Int64(balance),
                frozen: frozen,
                tokenName: tokenInfo?.name,
                tokenSymbol: tokenInfo?.symbol,
                tokenDecimals: tokenInfo?.decimals
            )
            modelContext.insert(persistentBalance)
        }
        
        try modelContext.save()
    }
    
    /// Fetch token balances for an identity
    func fetchTokenBalances(identityId: Data) throws -> [(tokenId: String, balance: UInt64, frozen: Bool)] {
        let predicate = PersistentTokenBalance.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentTokenBalance>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.balance, order: .reverse)]
        )
        let persistentBalances = try modelContext.fetch(descriptor)
        return persistentBalances.map { $0.toTokenBalance() }
    }
    
    // MARK: - Sync Operations
    
    /// Mark an identity as synced
    func markIdentityAsSynced(identityId: Data) throws {
        let predicate = PersistentIdentity.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)
        
        if let identity = try modelContext.fetch(descriptor).first {
            identity.markAsSynced()
            try modelContext.save()
        }
    }
    
    /// Get identities that need syncing
    func fetchIdentitiesNeedingSync(olderThan hours: Int = 1) throws -> [IdentityModel] {
        let date = Date().addingTimeInterval(-Double(hours) * 3600)
        let predicate = PersistentIdentity.needsSyncPredicate(olderThan: date)
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.lastSyncedAt)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }
    
    // MARK: - Utility Operations
    
    /// Clear all data (for testing or reset)
    func clearAllData() throws {
        // Delete all identities
        try modelContext.delete(model: PersistentIdentity.self)
        
        // Delete all documents
        try modelContext.delete(model: PersistentDocument.self)
        
        // Delete all contracts
        try modelContext.delete(model: PersistentDataContract.self)
        
        // Delete all public keys
        try modelContext.delete(model: PersistentPublicKey.self)
        
        // Delete all token balances
        try modelContext.delete(model: PersistentTokenBalance.self)
        
        try modelContext.save()
    }
    
    /// Get statistics about stored data
    func getDataStatistics() throws -> (identities: Int, documents: Int, contracts: Int, tokenBalances: Int) {
        let identityCount = try modelContext.fetchCount(FetchDescriptor<PersistentIdentity>())
        let documentCount = try modelContext.fetchCount(FetchDescriptor<PersistentDocument>())
        let contractCount = try modelContext.fetchCount(FetchDescriptor<PersistentDataContract>())
        let tokenBalanceCount = try modelContext.fetchCount(FetchDescriptor<PersistentTokenBalance>())
        
        return (identities: identityCount, documents: documentCount, contracts: contractCount, tokenBalances: tokenBalanceCount)
    }
    
    /// Remove private key reference from a public key
    func removePrivateKeyReference(identityId: Data, keyId: Int32) throws {
        let predicate = PersistentIdentity.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)
        
        if let identity = try modelContext.fetch(descriptor).first,
           let publicKey = identity.publicKeys.first(where: { $0.keyId == keyId }) {
            publicKey.privateKeyKeychainIdentifier = nil
            try modelContext.save()
        }
    }
}
