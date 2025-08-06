import Foundation
import SwiftData

@Model
final class PersistentDocument {
    // Primary key
    @Attribute(.unique) var documentId: String
    
    // Core document properties
    var documentType: String
    var revision: Int32
    var data: Data // JSON serialized document properties
    
    // References (stored as strings for queries)
    var contractId: String
    var ownerId: String
    
    // Binary data for efficient operations
    var contractIdData: Data
    var ownerIdData: Data
    
    // Timestamps
    var createdAt: Date
    var updatedAt: Date
    var transferredAt: Date?
    
    // Block heights
    var createdAtBlockHeight: Int64?
    var updatedAtBlockHeight: Int64?
    var transferredAtBlockHeight: Int64?
    
    // Core block heights
    var createdAtCoreBlockHeight: Int64?
    var updatedAtCoreBlockHeight: Int64?
    var transferredAtCoreBlockHeight: Int64?
    
    // Network
    var network: String
    
    // Deletion flag
    var isDeleted: Bool = false
    
    // Local tracking
    var localCreatedAt: Date
    var localUpdatedAt: Date
    
    // Relationships
    var documentType_relation: PersistentDocumentType?
    
    // Optional reference to local identity (if owner is local)
    var ownerIdentity: PersistentIdentity?
    
    // Computed properties
    var id: Data {
        Data.identifier(fromBase58: documentId) ?? Data()
    }
    
    var idBase58: String {
        documentId
    }
    
    var ownerIdBase58: String {
        ownerId
    }
    
    var contractIdBase58: String {
        contractId
    }
    
    var properties: [String: Any]? {
        try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
    }
    
    var displayTitle: String {
        // Try to extract a title from common property names
        guard let props = properties else { return "Document" }
        
        if let title = props["title"] as? String { return title }
        if let name = props["name"] as? String { return name }
        if let label = props["label"] as? String { return label }
        if let normalizedLabel = props["normalizedLabel"] as? String { return normalizedLabel }
        
        return documentType
    }
    
    var summary: String {
        var parts: [String] = []
        
        parts.append("Type: \(documentType)")
        
        parts.append("Rev: \(revision)")
        
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        parts.append("Created: \(formatter.string(from: createdAt))")
        
        return parts.joined(separator: " • ")
    }
    
    init(
        documentId: String,
        documentType: String,
        revision: Int32,
        data: Data,
        contractId: String,
        ownerId: String,
        network: String = "testnet"
    ) {
        self.documentId = documentId
        self.documentType = documentType
        self.revision = revision
        self.data = data
        self.contractId = contractId
        self.ownerId = ownerId
        self.contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
        self.ownerIdData = Data.identifier(fromBase58: ownerId) ?? Data()
        self.network = network
        self.createdAt = Date()
        self.updatedAt = Date()
        self.localCreatedAt = Date()
        self.localUpdatedAt = Date()
    }
    
    // MARK: - Methods
    func updateProperties(_ newData: Data) {
        self.data = newData
        self.updatedAt = Date()
    }
    
    func updateRevision(_ newRevision: Int64) {
        self.revision = Int32(newRevision)
        self.updatedAt = Date()
    }
    
    func markAsDeleted() {
        self.isDeleted = true
        self.updatedAt = Date()
    }
    
    func toDocumentModel() -> DocumentModel {
        // Convert data from binary to dictionary
        let dataDict = (try? JSONSerialization.jsonObject(with: data, options: [])) as? [String: Any] ?? [:]
        
        return DocumentModel(
            id: documentId,
            contractId: contractId,
            documentType: documentType,
            ownerId: Data.identifier(fromBase58: ownerId) ?? Data(),
            data: dataDict,
            createdAt: createdAt,
            updatedAt: updatedAt,
            dppDocument: nil,
            revision: Revision(revision)
        )
    }
    
    // MARK: - Static Methods
    static func from(_ document: DocumentModel) -> PersistentDocument {
        // Convert dictionary to binary data
        let dataToStore = (try? JSONSerialization.data(withJSONObject: document.data, options: [])) ?? Data()
        
        return PersistentDocument(
            documentId: document.id,
            documentType: document.documentType,
            revision: Int32(document.revision),
            data: dataToStore,
            contractId: document.contractId,
            ownerId: document.ownerId.toBase58String(),
            network: "testnet"
        )
    }
    
    static func predicate(documentId: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { doc in
            doc.documentId == documentId && doc.isDeleted == false
        }
    }
    
    static func predicate(contractId: String, network: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { doc in
            doc.contractId == contractId && doc.network == network && doc.isDeleted == false
        }
    }
    
    static func predicate(ownerId: Data) -> Predicate<PersistentDocument> {
        let ownerIdString = ownerId.toBase58String()
        return #Predicate<PersistentDocument> { doc in
            doc.ownerId == ownerIdString && doc.isDeleted == false
        }
    }
    
    // MARK: - Identity Linking
    func linkToLocalIdentityIfNeeded(in modelContext: ModelContext) {
        // Check if we already have an owner identity linked
        guard ownerIdentity == nil else { return }
        
        // Try to find a local identity matching the owner ID
        let ownerIdToMatch = self.ownerIdData
        let identityPredicate = #Predicate<PersistentIdentity> { identity in
            identity.identityId == ownerIdToMatch && identity.isLocal == true
        }
        
        let descriptor = FetchDescriptor<PersistentIdentity>(predicate: identityPredicate)
        
        do {
            if let localIdentity = try modelContext.fetch(descriptor).first {
                self.ownerIdentity = localIdentity
                self.localUpdatedAt = Date()
            }
        } catch {
            print("Failed to link document to local identity: \(error)")
        }
    }
}