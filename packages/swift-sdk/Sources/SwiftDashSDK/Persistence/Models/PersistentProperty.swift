import Foundation
import SwiftData

/// SwiftData model for persisting document type properties
@Model
public final class PersistentProperty {
    @Attribute(.unique) public var id: Data
    public var contractId: Data
    public var documentTypeName: String
    public var name: String

    // Property type and constraints
    public var type: String
    public var format: String?
    public var contentMediaType: String?
    public var byteArray: Bool
    public var minItems: Int?
    public var maxItems: Int?
    public var pattern: String?
    public var minLength: Int?
    public var maxLength: Int?
    public var minValue: Int?
    public var maxValue: Int?
    public var fieldDescription: String?

    // Property attributes
    public var transient: Bool
    public var isRequired: Bool

    // Timestamps
    public var createdAt: Date

    // Relationship to document type
    public var documentType: PersistentDocumentType?

    public init(contractId: Data, documentTypeName: String, name: String, type: String) {
        // Create unique ID by combining contract ID, document type name, and property name
        var idData = contractId
        idData.append(documentTypeName.data(using: .utf8) ?? Data())
        idData.append(name.data(using: .utf8) ?? Data())
        self.id = idData

        self.contractId = contractId
        self.documentTypeName = documentTypeName
        self.name = name
        self.type = type
        self.byteArray = false
        self.transient = false
        self.isRequired = false
        self.createdAt = Date()
    }
}
