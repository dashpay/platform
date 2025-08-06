import Foundation
import SwiftData

@Model
final class PersistentDocumentType {
    @Attribute(.unique) var id: Data // Combines contractId + name
    var contractId: Data
    var name: String
    
    // Schema stored as JSON
    var schemaJSON: Data
    var propertiesJSON: Data // Flattened properties
    
    // Document behavior settings
    var documentsKeepHistory: Bool
    var documentsMutable: Bool
    var documentsCanBeDeleted: Bool
    var documentsTransferable: Bool
    
    // Required fields
    var requiredFieldsJSON: Data? // Array of field names
    
    // Security
    var securityLevel: Int // 0 = lowest, higher numbers = more secure
    
    // Trade mode
    var tradeMode: Bool
    
    // Identity encryption keys
    var requiresIdentityEncryptionBoundedKey: Bool
    var requiresIdentityDecryptionBoundedKey: Bool
    
    // Timestamps
    var createdAt: Date
    var lastAccessedAt: Date
    
    // Relationship to data contract
    var dataContract: PersistentDataContract?
    
    // Relationship to documents
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.documentType_relation)
    var documents: [PersistentDocument]?
    
    // Relationship to indices
    @Relationship(deleteRule: .cascade, inverse: \PersistentIndex.documentType)
    var indices: [PersistentIndex]?
    
    // Relationship to properties
    @Relationship(deleteRule: .cascade, inverse: \PersistentProperty.documentType)
    var propertiesList: [PersistentProperty]?
    
    init(contractId: Data, name: String, schemaJSON: Data, propertiesJSON: Data) {
        // Create unique ID by combining contract ID and name
        var idData = contractId
        idData.append(name.data(using: .utf8) ?? Data())
        self.id = idData
        
        self.contractId = contractId
        self.name = name
        self.schemaJSON = schemaJSON
        self.propertiesJSON = propertiesJSON
        self.documentsKeepHistory = false
        self.documentsMutable = true
        self.documentsCanBeDeleted = true
        self.documentsTransferable = false
        self.securityLevel = 0
        self.tradeMode = false
        self.requiresIdentityEncryptionBoundedKey = false
        self.requiresIdentityDecryptionBoundedKey = false
        self.createdAt = Date()
        self.lastAccessedAt = Date()
    }
}

// MARK: - Computed Properties
extension PersistentDocumentType {
    var contractIdBase58: String {
        contractId.toBase58String()
    }
    
    var schema: [String: Any]? {
        try? JSONSerialization.jsonObject(with: schemaJSON, options: []) as? [String: Any]
    }
    
    var properties: [String: Any]? {
        try? JSONSerialization.jsonObject(with: propertiesJSON, options: []) as? [String: Any]
    }
    
    // Use propertiesList when available, otherwise fall back to JSON
    var persistentProperties: [PersistentProperty]? {
        return propertiesList
    }
    
    var requiredFields: [String]? {
        guard let data = requiredFieldsJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data, options: []) as? [String]
    }
    
    var documentCount: Int {
        documents?.count ?? 0
    }
}