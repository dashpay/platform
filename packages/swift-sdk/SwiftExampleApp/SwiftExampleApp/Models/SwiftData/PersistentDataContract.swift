import Foundation
import SwiftData

@Model
final class PersistentDataContract {
    @Attribute(.unique) var id: Data
    var name: String
    var serializedContract: Data
    var createdAt: Date
    var lastAccessedAt: Date
    
    // Binary serialization (CBOR format)
    var binarySerialization: Data?
    
    // Version info
    var version: Int?
    var ownerId: Data?
    
    // Contract configuration
    var canBeDeleted: Bool
    var readonly: Bool
    var keepsHistory: Bool
    var schemaDefs: Int?
    
    // Document defaults
    var documentsKeepHistoryContractDefault: Bool
    var documentsMutableContractDefault: Bool
    var documentsCanBeDeletedContractDefault: Bool
    
    // Relationships with cascade delete
    @Relationship(deleteRule: .cascade, inverse: \PersistentToken.dataContract)
    var tokens: [PersistentToken]?
    
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocumentType.dataContract)
    var documentTypes: [PersistentDocumentType]?
    
    // Computed properties
    var idBase58: String {
        id.toBase58String()
    }
    
    var ownerIdBase58: String? {
        ownerId?.toBase58String()
    }
    
    var parsedContract: [String: Any]? {
        try? JSONSerialization.jsonObject(with: serializedContract, options: []) as? [String: Any]
    }
    
    var binarySerializationHex: String? {
        binarySerialization?.toHexString()
    }
    
    init(id: Data, name: String, serializedContract: Data) {
        self.id = id
        self.name = name
        self.serializedContract = serializedContract
        self.createdAt = Date()
        self.lastAccessedAt = Date()
        
        // Default values for contract configuration
        self.canBeDeleted = false
        self.readonly = false
        self.keepsHistory = false
        self.documentsKeepHistoryContractDefault = false
        self.documentsMutableContractDefault = true
        self.documentsCanBeDeletedContractDefault = true
    }
    
    func updateLastAccessed() {
        self.lastAccessedAt = Date()
    }
}