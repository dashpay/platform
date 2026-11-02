import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Protocol version 14 index keywords that have no column of their own on
/// `PersistentIndex`: `summableOffCountIndex`, and the `{ "at": ... }` form of
/// `rankedCountable` / `rankedSummable` / `rankedAverageable`.
/// `DataContractParser` records a ranking declared in either spelling in its
/// boolean column, and the counter source and the ranked levels are read back
/// off the document type's persisted schema.
///
/// These tests run the real parser over an in-memory store, so every index
/// goes through the `JSONSerialization` round trip the persisted `schemaJSON`
/// takes.
@MainActor
final class DataContractParserIndexTests: XCTestCase {

    private let contractId = Data(repeating: 0xC3, count: 32)

    func testShouldKeepACounterIndexSourceAndAtRankings() throws {
        let (context, indices) = try parse(indices: [
            [
                "name": "byPost",
                "properties": [["postId": "asc"]],
                "countable": "countable"
            ],
            [
                "name": "byAuthorPost",
                "properties": [["postAuthor": "asc"], ["postId": "asc"]],
                "summableOffCountIndex": "byPost",
                "rangeCountable": true,
                "rangeSummable": true,
                "rankedSummable": ["at": ["postAuthor", "postId"]],
                "rankedAverageable": ["at": ["postAuthor"]],
                "preallocated": true
            ]
        ])
        defer { withExtendedLifetime(context) {} }

        let counter = try XCTUnwrap(indices["byAuthorPost"])
        XCTAssertEqual(counter.summableOffCountIndex, "byPost")
        XCTAssertTrue(counter.rangeCountable)
        XCTAssertTrue(counter.rangeSummable)
        XCTAssertTrue(counter.preallocated)

        XCTAssertFalse(counter.rankedCountable)
        XCTAssertEqual(counter.rankedCountableAt, [])
        XCTAssertTrue(counter.rankedSummable)
        XCTAssertEqual(counter.rankedSummableAt, ["postAuthor", "postId"])
        XCTAssertTrue(counter.rankedAverageable)
        XCTAssertEqual(counter.rankedAverageableAt, ["postAuthor"])
        XCTAssertEqual(
            counter.authoredKeywords,
            AuthoredIndexKeywords(
                summableOffCountIndex: "byPost",
                rankedCountableAt: [],
                rankedSummableAt: ["postAuthor", "postId"],
                rankedAverageableAt: ["postAuthor"]
            )
        )

        let source = try XCTUnwrap(indices["byPost"])
        XCTAssertNil(source.summableOffCountIndex)
    }

    func testShouldReadRankingsInTheBooleanAndSingleLevelForms() throws {
        let (context, indices) = try parse(indices: [
            [
                "name": "byTagPost",
                "properties": [["tag": "asc"], ["postId": "asc"]],
                "countable": true,
                "rankedCountable": ["at": "tag"],
                "rankedSummable": true,
                "rankedAverageable": false
            ]
        ])
        defer { withExtendedLifetime(context) {} }

        let index = try XCTUnwrap(indices["byTagPost"])
        XCTAssertTrue(index.rankedCountable)
        XCTAssertEqual(index.rankedCountableAt, ["tag"])
        XCTAssertTrue(index.rankedSummable)
        XCTAssertEqual(index.rankedSummableAt, [])
        XCTAssertFalse(index.rankedAverageable)
        XCTAssertEqual(index.rankedAverageableAt, [])
        XCTAssertNil(index.summableOffCountIndex)
    }

    func testShouldReadNoAuthoredKeywordsWithoutADocumentType() {
        let index = PersistentIndex(
            contractId: contractId,
            documentTypeName: "like",
            name: "byAuthorPost",
            properties: ["postAuthor", "postId"]
        )

        XCTAssertNil(index.authoredDefinition)
        XCTAssertNil(index.summableOffCountIndex)
        XCTAssertEqual(index.rankedSummableAt, [])
    }

    // MARK: - Helpers

    /// Run the real parser over a contract with one indexOnly document type
    /// carrying `indices`, and hand back the context (which the rows need
    /// alive) and the persisted index rows by name. `parseDocumentTypes`
    /// needs the `PersistentDataContract` row first.
    private func parse(
        indices: [[String: Any]]
    ) throws -> (ModelContext, [String: PersistentIndex]) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: Data(),
            network: .testnet
        )
        context.insert(contract)
        try context.save()

        try DataContractParser.parseDataContract(
            contractData: [
                "documents": [
                    "like": [
                        "type": "object",
                        "indexOnly": true,
                        "properties": [
                            "postId": ["type": "string", "maxLength": 63, "position": 0],
                            "postAuthor": ["type": "string", "maxLength": 63, "position": 1],
                            "tag": ["type": "string", "maxLength": 63, "position": 2]
                        ],
                        "indices": indices,
                        "additionalProperties": false
                    ]
                ]
            ],
            contractId: contractId,
            modelContext: context
        )

        let id = contractId
        let descriptor = FetchDescriptor<PersistentIndex>(
            predicate: #Predicate { $0.contractId == id }
        )
        let rows = try context.fetch(descriptor)
        return (context, Dictionary(uniqueKeysWithValues: rows.map { ($0.name, $0) }))
    }
}
