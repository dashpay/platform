import Foundation
import SwiftData

/// SwiftData model for persisting document type definitions
@Model
public final class PersistentDocumentType {
    @Attribute(.unique) public var id: Data
    public var contractId: Data
    public var name: String

    // Schema stored as JSON
    public var schemaJSON: Data
    public var propertiesJSON: Data

    // Document behavior settings
    public var documentsKeepHistory: Bool
    public var documentsMutable: Bool
    public var documentsCanBeDeleted: Bool
    public var documentsTransferable: Bool

    // indexOnly storage mode (meta-schema v3, protocol version 14): no
    // stored rows — the index entries ARE the documents
    public var indexOnly: Bool = false

    // Required fields
    public var requiredFieldsJSON: Data?

    // Security
    public var securityLevel: Int

    // Trade and creation restrictions
    public var tradeMode: Int
    public var creationRestrictionMode: Int

    // Identity encryption keys
    public var requiresIdentityEncryptionBoundedKey: Bool
    public var requiresIdentityDecryptionBoundedKey: Bool

    // Timestamps
    public var createdAt: Date
    public var lastAccessedAt: Date

    // Relationship to data contract
    public var dataContract: PersistentDataContract?

    // Relationship to documents
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.documentType_relation)
    public var documents: [PersistentDocument]?

    // Relationship to indices
    @Relationship(deleteRule: .cascade, inverse: \PersistentIndex.documentType)
    public var indices: [PersistentIndex]?

    // Relationship to properties
    @Relationship(deleteRule: .cascade, inverse: \PersistentProperty.documentType)
    public var propertiesList: [PersistentProperty]?

    public init(contractId: Data, name: String, schemaJSON: Data, propertiesJSON: Data) {
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
        self.tradeMode = 0
        self.creationRestrictionMode = 0
        self.requiresIdentityEncryptionBoundedKey = false
        self.requiresIdentityDecryptionBoundedKey = false
        self.createdAt = Date()
        self.lastAccessedAt = Date()
    }
}

// MARK: - Computed Properties
extension PersistentDocumentType {
    public var contractIdBase58: String {
        contractId.toBase58String()
    }

    public var schema: [String: Any]? {
        try? JSONSerialization.jsonObject(with: schemaJSON, options: []) as? [String: Any]
    }

    public var properties: [String: Any]? {
        try? JSONSerialization.jsonObject(with: propertiesJSON, options: []) as? [String: Any]
    }

    public var persistentProperties: [PersistentProperty]? {
        return propertiesList
    }

    public var requiredFields: [String]? {
        guard let data = requiredFieldsJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data, options: []) as? [String]
    }

    public var documentCount: Int {
        documents?.count ?? 0
    }
}
