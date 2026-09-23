import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Protocol version 14 adds typed arrays to document schemas: a
/// `type: "array"` property declared by an `items` schema (one scalar
/// element) instead of `byteArray: true`. `DataContractParser` persists one as
/// an ordinary `"array"` `PersistentProperty` row (`byteArray` false, element
/// counts in `minItems` / `maxItems`), and the element schema is read back off
/// the persisted document type schema by `PersistentDocumentType.typedArrays`
/// and `typedArray(named:)`, with no stored column of its own.
///
/// These tests run the real parser over an in-memory store, so every
/// declaration below goes through the `JSONSerialization` round trip the
/// persisted `schemaJSON` takes: booleans and numbers come back as
/// `NSNumber`s, which the readers must not confuse.
@MainActor
final class DataContractParserTypedArrayTests: XCTestCase {

    private let contractId = Data(repeating: 0xC2, count: 32)

    private let identifierItems: [String: Any] = [
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier"
    ]

    // MARK: - Persisted property rows

    func testTypedArrayPropertiesArePersistedAsArrayRowsWithoutThrowing() throws {
        let (context, _) = try parse(properties: [
            "tags": [
                "type": "array",
                "items": ["type": "string", "maxLength": 32],
                "minItems": 1,
                "maxItems": 8,
                "position": 0
            ],
            "reasons": [
                "type": "array",
                "items": identifierItems,
                "maxItems": 64,
                "uniqueItems": true,
                "position": 1
            ]
        ])

        let properties = try fetchProperties(in: context)
        let tags = try XCTUnwrap(properties.first { $0.name == "tags" })
        XCTAssertEqual(tags.type, "array")
        XCTAssertFalse(tags.byteArray)
        XCTAssertEqual(tags.minItems, 1)
        XCTAssertEqual(tags.maxItems, 8)
        // The element's own bounds stay on the element, not on the row
        XCTAssertNil(tags.maxLength)

        // An identifier element does not make the property an identifier
        let reasons = try XCTUnwrap(properties.first { $0.name == "reasons" })
        XCTAssertEqual(reasons.type, "array")
        XCTAssertFalse(reasons.byteArray)
        XCTAssertNil(reasons.minItems)
        XCTAssertEqual(reasons.maxItems, 64)
        XCTAssertNil(reasons.contentMediaType)
    }

    func testByteArrayPropertyStillParsesAndIsNotATypedArray() throws {
        let (context, docType) = try parse(properties: [
            "owner": [
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0
            ],
            "payload": [
                "type": "array",
                "byteArray": true,
                "maxItems": 64,
                "position": 1
            ]
        ])

        let owner = try XCTUnwrap(try fetchProperties(in: context).first { $0.name == "owner" })
        XCTAssertEqual(owner.type, "array")
        XCTAssertTrue(owner.byteArray)
        XCTAssertEqual(owner.minItems, 32)
        XCTAssertEqual(owner.maxItems, 32)
        XCTAssertEqual(owner.contentMediaType, "application/x.dash.dpp.identifier")

        XCTAssertNil(docType.typedArray(named: "owner"))
        XCTAssertNil(docType.typedArray(named: "payload"))
        XCTAssertEqual(docType.typedArrays, [])
    }

    // MARK: - Element kinds

    func testIntegerElementsReportTheirBoundsAndAllowedValues() throws {
        let (_, docType) = try parse(properties: [
            "scores": [
                "type": "array",
                "items": ["type": "integer", "minimum": 1, "maximum": 10, "enum": [1, 5, 10]],
                "minItems": 1,
                "maxItems": 5,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "scores"),
            DocumentTypedArray(
                path: "scores",
                element: .integer(minimum: 1, maximum: 10, allowedValues: [1, 5, 10]),
                minItems: 1,
                maxItems: 5,
                uniqueItems: false
            )
        )
    }

    func testIntegerElementsWithoutBoundsReportNone() throws {
        let (_, docType) = try parse(properties: [
            "counts": [
                "type": "array",
                "items": ["type": "integer"],
                "maxItems": 4,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "counts")?.element,
            .integer(minimum: nil, maximum: nil, allowedValues: nil)
        )
    }

    func testNumberElementsReportTheirBoundsAndAllowedValues() throws {
        let (_, docType) = try parse(properties: [
            "weights": [
                "type": "array",
                "items": ["type": "number", "minimum": -1.5, "maximum": 2, "enum": [-1.5, 0, 2]],
                "maxItems": 3,
                "uniqueItems": true,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "weights"),
            DocumentTypedArray(
                path: "weights",
                element: .number(minimum: -1.5, maximum: 2, allowedValues: [-1.5, 0, 2]),
                minItems: nil,
                maxItems: 3,
                uniqueItems: true
            )
        )
    }

    /// `JSONSerialization` hands a JSON boolean back as an `NSNumber`, which
    /// casts to an integer: the element must still read as booleans.
    func testBooleanElementsReportTheirAllowedValues() throws {
        let (_, docType) = try parse(properties: [
            "flags": [
                "type": "array",
                "items": ["type": "boolean"],
                "maxItems": 2,
                "position": 0
            ],
            "confirmations": [
                "type": "array",
                "items": ["type": "boolean", "enum": [true]],
                "maxItems": 2,
                "position": 1
            ]
        ])

        XCTAssertEqual(docType.typedArray(named: "flags")?.element, .boolean(allowedValues: nil))
        XCTAssertEqual(
            docType.typedArray(named: "confirmations")?.element, .boolean(allowedValues: [true]))
    }

    func testStringElementsReportTheirLengthsAndAllowedValues() throws {
        let (_, docType) = try parse(properties: [
            "moods": [
                "type": "array",
                "items": [
                    "type": "string",
                    "minLength": 2,
                    "maxLength": 5,
                    "enum": ["happy", "sad", "ok"]
                ],
                "minItems": 0,
                "maxItems": 3,
                "uniqueItems": false,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "moods"),
            DocumentTypedArray(
                path: "moods",
                element: .string(minLength: 2, maxLength: 5, allowedValues: ["happy", "sad", "ok"]),
                minItems: 0,
                maxItems: 3,
                uniqueItems: false
            )
        )
    }

    /// An element's `minItems` / `maxItems` count its bytes; the array's own
    /// count its elements.
    func testByteArrayElementsReportTheirSize() throws {
        let (_, docType) = try parse(properties: [
            "hashes": [
                "type": "array",
                "items": ["type": "array", "byteArray": true, "minItems": 20, "maxItems": 32],
                "minItems": 2,
                "maxItems": 16,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "hashes"),
            DocumentTypedArray(
                path: "hashes",
                element: .byteArray(minSize: 20, maxSize: 32),
                minItems: 2,
                maxItems: 16,
                uniqueItems: false
            )
        )
    }

    /// `distinctFrom` (protocol version 14) may ride on identifier elements;
    /// it changes nothing about what an element is.
    func testIdentifierElementsReportAsIdentifiers() throws {
        var items = identifierItems
        items["distinctFrom"] = ["$ownerId"]
        let (_, docType) = try parse(properties: [
            "reasons": [
                "type": "array",
                "items": items,
                "maxItems": 64,
                "uniqueItems": true,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "reasons"),
            DocumentTypedArray(
                path: "reasons",
                element: .identifier,
                minItems: nil,
                maxItems: 64,
                uniqueItems: true
            )
        )
    }

    // MARK: - Nesting and lookup

    func testTypedArraysAreListedByPathIncludingThoseNestedInObjects() throws {
        let (_, docType) = try parse(properties: [
            "tags": [
                "type": "array",
                "items": ["type": "string"],
                "maxItems": 8,
                "position": 0
            ],
            "team": [
                "type": "object",
                "properties": [
                    "leads": [
                        "type": "array",
                        "items": identifierItems,
                        "minItems": 1,
                        "maxItems": 3,
                        "uniqueItems": true,
                        "position": 0
                    ],
                    "name": ["type": "string", "maxLength": 63, "position": 1]
                ],
                "additionalProperties": false,
                "position": 1
            ],
            "title": ["type": "string", "maxLength": 63, "position": 2]
        ])

        XCTAssertEqual(docType.typedArrays.map(\.path), ["tags", "team.leads"])
        XCTAssertEqual(
            docType.typedArrays.last,
            DocumentTypedArray(
                path: "team.leads",
                element: .identifier,
                minItems: 1,
                maxItems: 3,
                uniqueItems: true
            )
        )

        // The named lookup reads top-level properties only
        XCTAssertNotNil(docType.typedArray(named: "tags"))
        XCTAssertNil(docType.typedArray(named: "team"))
        XCTAssertNil(docType.typedArray(named: "leads"))
        XCTAssertNil(docType.typedArray(named: "team.leads"))
        XCTAssertNil(docType.typedArray(named: "title"))
        XCTAssertNil(docType.typedArray(named: "missing"))
    }

    func testDocumentTypeWithoutTypedArraysReportsNone() throws {
        let (_, docType) = try parse(properties: [
            "title": ["type": "string", "maxLength": 63, "position": 0]
        ])

        XCTAssertEqual(docType.typedArrays, [])
    }

    // MARK: - Declarations DPP refuses

    /// DPP refuses each of these at registration, so they reach a client only
    /// through hand-edited JSON. None may be mistaken for a typed array.
    func testDeclarationsThatAreNotTypedArraysReadAsNone() {
        let notTypedArrays: [String: [String: Any]] = [
            "byteArray false": [
                "type": "array", "byteArray": false,
                "items": ["type": "string"], "maxItems": 4
            ],
            "no items": ["type": "array", "maxItems": 4],
            "tuple items": ["type": "array", "items": [["type": "string"]], "maxItems": 4],
            "object items": ["type": "array", "items": ["type": "object"], "maxItems": 4],
            "array of arrays": [
                "type": "array", "items": ["type": "array", "items": ["type": "string"]],
                "maxItems": 4
            ],
            "no maxItems": ["type": "array", "items": ["type": "string"]],
            "not an array": ["type": "string", "items": ["type": "string"], "maxItems": 4]
        ]

        for (label, schema) in notTypedArrays {
            XCTAssertNil(DocumentTypedArray(path: "p", propertySchema: schema), label)
        }
    }

    // MARK: - Helpers

    /// Run the real parser over a one-document-type contract and hand back
    /// the context and the persisted document type row. `parseDocumentTypes`
    /// needs the `PersistentDataContract` row to exist first.
    private func parse(
        properties: [String: Any]
    ) throws -> (ModelContext, PersistentDocumentType) {
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
                    "post": [
                        "type": "object",
                        "properties": properties,
                        "additionalProperties": false
                    ]
                ]
            ],
            contractId: contractId,
            modelContext: context
        )

        let id = contractId
        let descriptor = FetchDescriptor<PersistentDocumentType>(
            predicate: #Predicate { $0.contractId == id }
        )
        let docType = try XCTUnwrap(
            try context.fetch(descriptor).first,
            "parser should have persisted one document type"
        )
        return (context, docType)
    }

    private func fetchProperties(in context: ModelContext) throws -> [PersistentProperty] {
        let id = contractId
        let descriptor = FetchDescriptor<PersistentProperty>(
            predicate: #Predicate { $0.contractId == id }
        )
        return try context.fetch(descriptor)
    }
}
