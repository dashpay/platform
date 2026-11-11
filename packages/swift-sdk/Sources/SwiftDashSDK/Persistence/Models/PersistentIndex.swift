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

    // Count / sum axes (meta-schema v3, protocol version 14). Every
    // keyword is persisted VERBATIM as authored in the contract JSON —
    // `countable` keeps its boolean-or-string spelling ("true" /
    // "countable" / "countableAllowingOffset"), and the `averageable` /
    // `rangeAverageable` sugar is stored as-is rather than desugared.
    // Interpreting the spellings (DPP's normalization rules) is protocol
    // logic and stays out of the SDK; display layers map them for
    // presentation.
    public var countable: String?
    public var rangeCountable: Bool = false
    public var summable: String?
    public var rangeSummable: Bool = false
    public var averageable: String?
    public var rangeAverageable: Bool = false

    // Ranking axes (each adds one ordered secondary tree). True when the
    // keyword is declared in either spelling, `true` or `{ "at": ... }`;
    // the levels of the object form are read by `rankedCountableAt` and
    // its siblings.
    public var rankedCountable: Bool = false
    public var rankedSummable: Bool = false
    public var rankedAverageable: Bool = false

    // indexOnly member key (the property whose value keys each entry).
    // Persisted only when declared; an omitted terminal on an indexOnly
    // type means $ownerId per DPP, a default display layers apply, except
    // on a `summableOffCountIndex` index, which has no terminal.
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

    /// This index's dictionary as authored, read off the owning document
    /// type's persisted schema, or `nil` when the row has no document type
    /// or the schema has no index of this name.
    ///
    /// The protocol-version-14 keywords below are read through here rather
    /// than stored: `PersistentDocumentType.schemaJSON` already holds the
    /// whole type dictionary, `indices` included, and a new stored property
    /// would move this model's entity hash, which costs a schema version and
    /// a fixture store (see `DashModelContainer.modelTypes` and
    /// `DashModelMigrationTests`).
    public var authoredDefinition: [String: Any]? {
        guard let indices = documentType?.schema?["indices"] as? [[String: Any]] else {
            return nil
        }
        return indices.first { $0["name"] as? String == name }
    }

    /// Every keyword below from one read of `authoredDefinition`, which
    /// parses the document type's whole persisted schema on each access.
    /// A caller that shows more than one of them, like a list row, reads
    /// this once instead of each accessor.
    public var authoredKeywords: AuthoredIndexKeywords {
        AuthoredIndexKeywords(authoredDefinition: authoredDefinition)
    }

    /// The source index named by `summableOffCountIndex` (protocol version
    /// 14), or `nil` on any other index. Such an index keeps one counter per
    /// group of the source index instead of entries, so it has no terminal.
    public var summableOffCountIndex: String? {
        authoredKeywords.summableOffCountIndex
    }

    /// The levels `rankedCountable` ranks at through its `{ "at": ... }`
    /// form (protocol version 14), in order. Empty for `true` or when absent.
    public var rankedCountableAt: [String] {
        authoredKeywords.rankedCountableAt
    }

    /// The levels `rankedSummable` ranks at; see `rankedCountableAt`.
    public var rankedSummableAt: [String] {
        authoredKeywords.rankedSummableAt
    }

    /// The levels `rankedAverageable` ranks at; see `rankedCountableAt`.
    public var rankedAverageableAt: [String] {
        authoredKeywords.rankedAverageableAt
    }
}

/// The protocol version 14 keywords of one index that have no column on
/// `PersistentIndex`, as authored. Read through
/// `PersistentIndex.authoredKeywords`; each field is documented on the
/// `PersistentIndex` accessor of the same name.
public struct AuthoredIndexKeywords: Equatable, Sendable {
    public let summableOffCountIndex: String?
    public let rankedCountableAt: [String]
    public let rankedSummableAt: [String]
    public let rankedAverageableAt: [String]
}

// Declared in an extension so the memberwise initializer stays available.
extension AuthoredIndexKeywords {
    /// The keywords of `definition`, an index dictionary as authored; all
    /// absent when it is `nil`.
    init(authoredDefinition definition: [String: Any]?) {
        self.init(
            summableOffCountIndex: definition?["summableOffCountIndex"] as? String,
            rankedCountableAt: Self.rankedAtLevels(definition?["rankedCountable"]),
            rankedSummableAt: Self.rankedAtLevels(definition?["rankedSummable"]),
            rankedAverageableAt: Self.rankedAtLevels(definition?["rankedAverageable"])
        )
    }

    /// The levels of a ranking keyword's `{ "at": ... }` form, where `at` is
    /// one property name or an ordered array of them. Empty for `true` or
    /// when absent.
    private static func rankedAtLevels(_ ranking: Any?) -> [String] {
        guard let ranking = ranking as? [String: Any] else { return [] }
        if let level = ranking["at"] as? String {
            return [level]
        }
        return ranking["at"] as? [String] ?? []
    }
}
