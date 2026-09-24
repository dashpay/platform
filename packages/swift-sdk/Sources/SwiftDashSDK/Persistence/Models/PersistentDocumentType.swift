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

    /// The type's `immutable` / `immutableAllowSetting` keywords (protocol
    /// version 14), read off the persisted schema.
    ///
    /// Derived rather than stored in columns of its own: `schemaJSON` already
    /// holds the whole document type dictionary as authored, so the keywords
    /// are persisted with every contract the parser writes, and a new stored
    /// property would move this model's entity hash. That costs a schema
    /// version and a fixture store (see `DashModelContainer.modelTypes` and
    /// `DashModelMigrationTests`), which a display-only keyword does not
    /// justify. `indexOnly` predates that discipline and kept its column.
    public var immutability: DocumentTypeImmutability {
        DocumentTypeImmutability(documentTypeSchema: schema)
    }

    /// Top-level properties frozen at document creation, sorted. Empty when
    /// the type declares no `immutable` list.
    public var immutableProperties: [String] {
        immutability.immutableProperties
    }

    /// The `immutable` entries a replace may still set while the stored
    /// document has no value for them, sorted. Empty when none are declared.
    public var immutableAllowSetting: [String] {
        immutability.immutableAllowSetting
    }

    /// Every typed array property the type declares (protocol version 14),
    /// those nested in object properties included under their dotted path
    /// (`"team.leads"`), sorted by path. Empty when it declares none.
    ///
    /// Derived from the persisted schema rather than stored, for the same
    /// reason as `immutability`: `schemaJSON` holds the whole document type
    /// dictionary as authored, element schemas included, and a new stored
    /// property on this model or on `PersistentProperty` would move an entity
    /// hash, which costs a schema version and a fixture store (see
    /// `DashModelContainer.modelTypes` and `DashModelMigrationTests`).
    /// `PersistentProperty` keeps a typed array as an ordinary `"array"` row
    /// with `byteArray` false and its element counts in `minItems` /
    /// `maxItems`.
    public var typedArrays: [DocumentTypedArray] {
        DocumentTypedArray.all(inDocumentTypeSchema: schema)
    }

    /// The typed array declared as the top-level property `name`, or `nil`
    /// when that property is absent or is not a typed array (a byte array
    /// among them). Read off the persisted schema; see `typedArrays`.
    public func typedArray(named name: String) -> DocumentTypedArray? {
        DocumentTypedArray.named(name, inDocumentTypeSchema: schema)
    }

    public var documentCount: Int {
        documents?.count ?? 0
    }
}
