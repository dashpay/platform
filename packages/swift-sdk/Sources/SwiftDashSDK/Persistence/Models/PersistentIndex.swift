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

    // Count / sum axes (meta-schema v3, protocol version 14).
    // `countable` is normalized to its string form ("countable" /
    // "countableAllowingOffset"); the `averageable` sugar is desugared
    // into `countable` + `summable` exactly as DPP does.
    public var countable: String?
    public var rangeCountable: Bool = false
    public var summable: String?
    public var rangeSummable: Bool = false

    // Ranking axes (each adds one ordered secondary tree)
    public var rankedCountable: Bool = false
    public var rankedSummable: Bool = false
    public var rankedAverageable: Bool = false

    // indexOnly member key (the property whose value keys each entry)
    public var terminal: String?

    // Preallocation: creating the refersTo-referenced document also
    // creates this index's trees, and deleting the last entry keeps them
    public var preallocated: Bool = false

    // Time-range bucketing transform ({on, range, step, phase}), if any
    public var timeRangeJSON: Data?

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

    /// The timeRange transform ({on, range, step, phase}) if the index
    /// buckets its first property into time ranges
    public var timeRange: [String: Any]? {
        guard let data = timeRangeJSON else { return nil }
        return try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
    }
}
