import Foundation
import SwiftData

/// SwiftData model for persisting Document data
@Model
final class PersistentDocument {
    // MARK: - Core Properties
    @Attribute(.unique) var documentId: String
    var contractId: String
    var documentType: String
    var ownerId: String
    var revision: Int64
    
    // MARK: - Properties Storage
    /// JSON encoded properties from the document
    var propertiesData: Data
    
    // MARK: - Timestamps
    var createdAt: Date?
    var updatedAt: Date?
    var transferredAt: Date?
    var deletedAt: Date?
    
    // MARK: - Block Heights
    var createdAtBlockHeight: Int64?
    var updatedAtBlockHeight: Int64?
    var transferredAtBlockHeight: Int64?
    var deletedAtBlockHeight: Int64?
    
    // MARK: - Core Block Heights
    var createdAtCoreBlockHeight: Int32?
    var updatedAtCoreBlockHeight: Int32?
    var transferredAtCoreBlockHeight: Int32?
    var deletedAtCoreBlockHeight: Int32?
    
    // MARK: - Metadata
    var isDeleted: Bool
    var lastSyncedAt: Date?
    
    // MARK: - Relationships
    @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.documents) 
    var owner: PersistentIdentity?
    
    @Relationship(deleteRule: .nullify, inverse: \PersistentContract.documents)
    var contract: PersistentContract?
    
    // MARK: - Initialization
    init(
        documentId: String,
        contractId: String,
        documentType: String,
        ownerId: String,
        revision: Int64 = 0,
        properties: [String: Any] = [:],
        createdAt: Date? = nil,
        updatedAt: Date? = nil,
        isDeleted: Bool = false
    ) {
        self.documentId = documentId
        self.contractId = contractId
        self.documentType = documentType
        self.ownerId = ownerId
        self.revision = revision
        self.propertiesData = (try? JSONSerialization.data(withJSONObject: properties)) ?? Data()
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.transferredAt = nil
        self.deletedAt = nil
        self.createdAtBlockHeight = nil
        self.updatedAtBlockHeight = nil
        self.transferredAtBlockHeight = nil
        self.deletedAtBlockHeight = nil
        self.createdAtCoreBlockHeight = nil
        self.updatedAtCoreBlockHeight = nil
        self.transferredAtCoreBlockHeight = nil
        self.deletedAtCoreBlockHeight = nil
        self.isDeleted = isDeleted
        self.lastSyncedAt = nil
    }
    
    // MARK: - Computed Properties
    var properties: [String: Any] {
        get {
            guard let json = try? JSONSerialization.jsonObject(with: propertiesData),
                  let dict = json as? [String: Any] else {
                return [:]
            }
            return dict
        }
        set {
            propertiesData = (try? JSONSerialization.data(withJSONObject: newValue)) ?? Data()
        }
    }
    
    // MARK: - Methods
    func updateRevision(_ newRevision: Int64) {
        self.revision = newRevision
        self.updatedAt = Date()
    }
    
    func markAsDeleted(at blockHeight: Int64? = nil, coreBlockHeight: Int32? = nil) {
        self.isDeleted = true
        self.deletedAt = Date()
        self.deletedAtBlockHeight = blockHeight
        self.deletedAtCoreBlockHeight = coreBlockHeight
    }
    
    func markAsTransferred(to newOwnerId: String, at blockHeight: Int64? = nil, coreBlockHeight: Int32? = nil) {
        self.ownerId = newOwnerId
        self.transferredAt = Date()
        self.transferredAtBlockHeight = blockHeight
        self.transferredAtCoreBlockHeight = coreBlockHeight
        self.owner = nil // Will be updated by relationship
    }
    
    func markAsSynced() {
        self.lastSyncedAt = Date()
    }
    
    func updateProperties(_ newProperties: [String: Any]) {
        self.properties = newProperties
        self.updatedAt = Date()
    }
}

// MARK: - Conversion Extensions

extension PersistentDocument {
    /// Convert to app's DocumentModel
    func toDocumentModel() -> DocumentModel {
        return DocumentModel(
            id: documentId,
            contractId: contractId,
            documentType: documentType,
            ownerId: ownerId,
            data: properties,
            createdAt: createdAt,
            updatedAt: updatedAt,
            dppDocument: nil, // Would need to reconstruct from data
            revision: Revision(revision)
        )
    }
    
    /// Create from DocumentModel
    static func from(_ model: DocumentModel) -> PersistentDocument {
        let persistent = PersistentDocument(
            documentId: model.id,
            contractId: model.contractId,
            documentType: model.documentType,
            ownerId: model.ownerId,
            revision: Int64(model.revision),
            properties: model.data,
            createdAt: model.createdAt,
            updatedAt: model.updatedAt,
            isDeleted: false
        )
        
        // Copy DPP document data if available
        if let dppDoc = model.dppDocument {
            persistent.createdAtBlockHeight = dppDoc.createdAtBlockHeight.map { Int64($0) }
            persistent.updatedAtBlockHeight = dppDoc.updatedAtBlockHeight.map { Int64($0) }
            persistent.transferredAtBlockHeight = dppDoc.transferredAtBlockHeight.map { Int64($0) }
            persistent.deletedAtBlockHeight = dppDoc.deletedAtBlockHeight.map { Int64($0) }
            
            persistent.createdAtCoreBlockHeight = dppDoc.createdAtCoreBlockHeight
            persistent.updatedAtCoreBlockHeight = dppDoc.updatedAtCoreBlockHeight
            persistent.transferredAtCoreBlockHeight = dppDoc.transferredAtCoreBlockHeight
            persistent.deletedAtCoreBlockHeight = dppDoc.deletedAtCoreBlockHeight
        }
        
        return persistent
    }
    
    /// Create from DPPDocument
    static func from(_ dppDocument: DPPDocument, contractId: String, documentType: String) -> PersistentDocument {
        // Convert PlatformValue properties to JSON-serializable format
        var jsonProperties: [String: Any] = [:]
        for (key, value) in dppDocument.properties {
            jsonProperties[key] = value.toJSONValue()
        }
        
        let persistent = PersistentDocument(
            documentId: dppDocument.idString,
            contractId: contractId,
            documentType: documentType,
            ownerId: dppDocument.ownerIdString,
            revision: Int64(dppDocument.revision ?? 0),
            properties: jsonProperties,
            createdAt: dppDocument.createdDate,
            updatedAt: dppDocument.updatedDate,
            isDeleted: dppDocument.deletedAt != nil
        )
        
        // Set timestamps
        persistent.transferredAt = dppDocument.transferredDate
        persistent.deletedAt = dppDocument.deletedDate
        
        // Set block heights
        persistent.createdAtBlockHeight = dppDocument.createdAtBlockHeight.map { Int64($0) }
        persistent.updatedAtBlockHeight = dppDocument.updatedAtBlockHeight.map { Int64($0) }
        persistent.transferredAtBlockHeight = dppDocument.transferredAtBlockHeight.map { Int64($0) }
        persistent.deletedAtBlockHeight = dppDocument.deletedAtBlockHeight.map { Int64($0) }
        
        persistent.createdAtCoreBlockHeight = dppDocument.createdAtCoreBlockHeight
        persistent.updatedAtCoreBlockHeight = dppDocument.updatedAtCoreBlockHeight
        persistent.transferredAtCoreBlockHeight = dppDocument.transferredAtCoreBlockHeight
        persistent.deletedAtCoreBlockHeight = dppDocument.deletedAtCoreBlockHeight
        
        return persistent
    }
}

// MARK: - PlatformValue to JSON Extension

extension PlatformValue {
    /// Convert PlatformValue to JSON-serializable value
    func toJSONValue() -> Any {
        switch self {
        case .null:
            return NSNull()
        case .bool(let value):
            return value
        case .integer(let value):
            return value
        case .float(let value):
            return value
        case .string(let value):
            return value
        case .bytes(let data):
            return data.base64EncodedString()
        case .array(let values):
            return values.map { $0.toJSONValue() }
        case .map(let dict):
            return dict.mapValues { $0.toJSONValue() }
        }
    }
}

// MARK: - Queries

extension PersistentDocument {
    /// Predicate to find document by ID
    static func predicate(documentId: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.documentId == documentId
        }
    }
    
    /// Predicate to find documents by contract
    static func predicate(contractId: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.contractId == contractId
        }
    }
    
    /// Predicate to find documents by owner
    static func predicate(ownerId: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.ownerId == ownerId
        }
    }
    
    /// Predicate to find documents by type
    static func predicate(documentType: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.documentType == documentType
        }
    }
    
    /// Predicate to find active (non-deleted) documents
    static var activeDocumentsPredicate: Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.isDeleted == false
        }
    }
    
    /// Predicate to find documents needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.lastSyncedAt == nil || document.lastSyncedAt! < date
        }
    }
    
    /// Predicate to find documents by contract and type
    static func predicate(contractId: String, documentType: String) -> Predicate<PersistentDocument> {
        #Predicate<PersistentDocument> { document in
            document.contractId == contractId && document.documentType == documentType
        }
    }
}