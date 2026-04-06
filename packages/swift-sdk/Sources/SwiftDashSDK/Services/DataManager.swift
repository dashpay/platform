import Foundation
import SwiftData


/// Service to manage SwiftData operations for the app
@MainActor
public final class DataManager: ObservableObject {
    private let modelContext: ModelContext
    public var currentNetwork: AppNetwork

    public init(modelContext: ModelContext, currentNetwork: AppNetwork = .testnet) {
        self.modelContext = modelContext
        self.currentNetwork = currentNetwork
    }

    // MARK: - Identity Operations

    /// Save or update an identity
    public func saveIdentity(_ identity: IdentityModel) throws {
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
            let persistentIdentity = PersistentIdentity.from(identity, network: currentNetwork)
            modelContext.insert(persistentIdentity)
        }

        try modelContext.save()
    }

    /// Fetch all identities for current network
    public func fetchIdentities() throws -> [IdentityModel] {
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: PersistentIdentity.predicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentIdentities = try modelContext.fetch(descriptor)
        return persistentIdentities.map { $0.toIdentityModel() }
    }

    /// Delete an identity
    public func deleteIdentity(withId identityId: Data) throws {
        let predicate = PersistentIdentity.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)

        if let identity = try modelContext.fetch(descriptor).first {
            modelContext.delete(identity)
            try modelContext.save()
        }
    }

    // MARK: - Document Operations

    /// Save or update a document
    public func saveDocument(_ document: DocumentModel) throws {
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
    public func fetchDocuments(contractId: String) throws -> [DocumentModel] {
        let predicate = PersistentDocument.predicate(contractId: contractId, network: currentNetwork.rawValue)
        let descriptor = FetchDescriptor<PersistentDocument>(
            predicate: predicate,
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentDocuments = try modelContext.fetch(descriptor)
        return persistentDocuments.map { $0.toDocumentModel() }
    }

    // MARK: - Contract Operations

    /// Save or update a contract
    public func saveContract(_ contract: ContractModel) throws {
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
    public func fetchContracts() throws -> [ContractModel] {
        let descriptor = FetchDescriptor<PersistentDataContract>(
            predicate: PersistentDataContract.predicate(network: currentNetwork.rawValue),
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        let persistentContracts = try modelContext.fetch(descriptor)
        return persistentContracts.map { $0.toContractModel() }
    }

    // MARK: - Utility Operations

    /// Get statistics about stored data
    public func getDataStatistics() throws -> (identities: Int, documents: Int, contracts: Int, tokenBalances: Int) {
        let identityCount = try modelContext.fetchCount(FetchDescriptor<PersistentIdentity>())
        let documentCount = try modelContext.fetchCount(FetchDescriptor<PersistentDocument>())
        let contractCount = try modelContext.fetchCount(FetchDescriptor<PersistentDataContract>())
        let tokenBalanceCount = try modelContext.fetchCount(FetchDescriptor<PersistentTokenBalance>())

        return (identities: identityCount, documents: documentCount, contracts: contractCount, tokenBalances: tokenBalanceCount)
    }

    /// Remove private key reference from a public key
    public func removePrivateKeyReference(identityId: Data, keyId: Int32) throws {
        let predicate = PersistentIdentity.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: predicate)

        if let identity = try modelContext.fetch(descriptor).first,
           let publicKey = identity.publicKeys.first(where: { $0.keyId == keyId }) {
            publicKey.privateKeyKeychainIdentifier = nil
            try modelContext.save()
        }
    }
}
