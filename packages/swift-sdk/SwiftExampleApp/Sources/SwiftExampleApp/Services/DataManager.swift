import Foundation
import SwiftData

/// Service to manage SwiftData operations for the app
@MainActor
final class DataManager: ObservableObject {
    private let modelContext: ModelContext
    
    init(modelContext: ModelContext) {
        self.modelContext = modelContext
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
            existingIdentity.isLocal = identity.isLocal
            existingIdentity.privateKeys = identity.privateKeys
            existingIdentity.votingPrivateKey = identity.votingPrivateKey
            existingIdentity.ownerPrivateKey = identity.ownerPrivateKey
            existingIdentity.payoutPrivateKey = identity.payoutPrivateKey
            existingIdentity.lastUpdated = Date()
            
            // Update public keys
            existingIdentity.publicKeys.removeAll()
            for publicKey in identity.publicKeys {
                if let persistentKey = PersistentPublicKey.from(publicKey, identityId: identity.id) {
                    existingIdentity.addPublicKey(persistentKey)
                }
            }
        } else {
            // Create new identity
            let persistentIdentity = PersistentIdentity.from(identity)
            modelContext.insert(persistentIdentity)
        }
        
        try modelContext.save()
    }
    
    /// Fetch all identities
    func fetchIdentities() throws -> [IdentityModel] {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }
    
    /// Fetch local identities only
    func fetchLocalIdentities() throws -> [IdentityModel] {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: PersistentIdentity.localIdentitiesPredicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }
    
    /// Delete an identity
    func deleteIdentity(withId identityId: String) throws {
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
            existingDocument.updateProperties(document.data)
            existingDocument.updateRevision(Int64(document.revision))
        } else {
            // Create new document
            let persistentDocument = PersistentDocument.from(document)
            modelContext.insert(persistentDocument)
        }
        
        try modelContext.save()
    }
    
    /// Fetch documents for a contract
    func fetchDocuments(contractId: String) throws -> [DocumentModel] {
        let predicate = PersistentDocument.predicate(contractId: contractId)
        let descriptor = FetchDescriptor<PersistentDocument>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentDocuments = try modelContext.fetch(descriptor)
        return persistentDocuments.map { $0.toDocumentModel() }
    }
    
    /// Fetch documents owned by an identity
    func fetchDocuments(ownerId: String) throws -> [DocumentModel] {
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
        let predicate = PersistentContract.predicate(contractId: contract.id)
        let descriptor = FetchDescriptor<PersistentContract>(predicate: predicate)
        
        if let existingContract = try modelContext.fetch(descriptor).first {
            // Update existing contract
            existingContract.name = contract.name
            existingContract.updateVersion(Int32(contract.version))
            existingContract.schema = contract.schema
            existingContract.documentTypes = contract.documentTypes
            existingContract.keywords = contract.keywords
            existingContract.contractDescription = contract.description
        } else {
            // Create new contract
            let persistentContract = PersistentContract.from(contract)
            modelContext.insert(persistentContract)
        }
        
        try modelContext.save()
    }
    
    /// Fetch all contracts
    func fetchContracts() throws -> [ContractModel] {
        let descriptor = FetchDescriptor<PersistentContract>(
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentContracts = try modelContext.fetch(descriptor)
        return persistentContracts.map { $0.toContractModel() }
    }
    
    /// Fetch contracts with tokens
    func fetchContractsWithTokens() throws -> [ContractModel] {
        let descriptor = FetchDescriptor<PersistentContract>(
            predicate: PersistentContract.contractsWithTokensPredicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentContracts = try modelContext.fetch(descriptor)
        return persistentContracts.map { $0.toContractModel() }
    }
    
    // MARK: - Token Balance Operations
    
    /// Save or update a token balance
    func saveTokenBalance(tokenId: String, identityId: String, balance: UInt64, frozen: Bool = false, tokenInfo: (name: String, symbol: String, decimals: Int32)? = nil) throws {
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
    func fetchTokenBalances(identityId: String) throws -> [(tokenId: String, balance: UInt64, frozen: Bool)] {
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
    func markIdentityAsSynced(identityId: String) throws {
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
        try modelContext.delete(model: PersistentContract.self)
        
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
        let contractCount = try modelContext.fetchCount(FetchDescriptor<PersistentContract>())
        let tokenBalanceCount = try modelContext.fetchCount(FetchDescriptor<PersistentTokenBalance>())
        
        return (identities: identityCount, documents: documentCount, contracts: contractCount, tokenBalances: tokenBalanceCount)
    }
}