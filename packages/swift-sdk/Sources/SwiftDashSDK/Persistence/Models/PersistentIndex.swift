import Foundation
import SwiftData

/// SwiftData model for persisting document type indices
@Model
public final class PersistentIndex {
    @Attribute(.unique) public var id: Data
    public var contractId: Data
    public var documentTypeName: String
    public var name: String

    // Index configuration
    public var unique: Bool
    public var nullSearchable: Bool
    public var contested: Bool

    // Properties in the index with sorting
    public var propertiesJSON: Data

    // Contested details (if contested)
    public var contestedDetailsJSON: Data?

    // Timestamps
    public var createdAt: Date

    // Relationship to document type
    public var documentType: PersistentDocumentType?

    public init(contractId: Data, documentTypeName: String, name: String, properties: [String]) {
        // Create unique ID by combining contract ID, document type name, and index name
        var idData = contractId
        idData.append(documentTypeName.data(using: .utf8) ?? Data())
        idData.append(name.data(using: .utf8) ?? Data())
        self.id = idData

        self.contractId = contractId
        self.documentTypeName = documentTypeName
        self.name = name
        self.unique = false
        self.nullSearchable = false
        self.contested = false

        // Store properties as JSON array
        if let jsonData = try? JSONSerialization.data(withJSONObject: properties, options: []) {
            self.propertiesJSON = jsonData
        } else {
            self.propertiesJSON = Data()
        }

        self.createdAt = Date()
    }
}

// MARK: - Computed Properties
extension PersistentIndex {
    public var properties: [String]? {
        try? JSONSerialization.jsonObject(with: propertiesJSON, options: []) as? [String]
    }

    public var contestedDetails: [String: Any]? {
        guard let data = contestedDetailsJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
    }
}
