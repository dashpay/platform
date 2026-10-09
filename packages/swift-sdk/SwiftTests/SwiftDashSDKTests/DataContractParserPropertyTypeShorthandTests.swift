import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Protocol version 14 lets a contract write a byte array property with a
/// type shorthand: `"type": "identifier"` for a 32-byte identifier, and
/// `"type": "bytes"` with a `size` for a byte array of exactly that many
/// bytes. The network stores and returns the contract as sent, so the fetched
/// contract JSON holds the shorthand. `DataContractParser` reads the contract
/// through Rust's long-form view of it
/// (`dash_sdk_data_contract_json_expand_property_type_shorthands`), so the
/// persisted rows and schemas hold `"type": "array"`, `byteArray` and the
/// sizes, exactly as if the contract had been written in full.
///
/// These run the real parser and the real FFI over an in-memory store, with
/// contracts shaped like the fetch JSON (`documentSchemas`, `schemaDefs`).
@MainActor
final class DataContractParserPropertyTypeShorthandTests: XCTestCase {

    private let contractId = Data(repeating: 0xC3, count: 32)

    private static let identifierMediaType = "application/x.dash.dpp.identifier"

    // MARK: - Top-level properties

    func testShouldPersistAnIdentifierPropertyAsAnIdentifierByteArrayRow() throws {
        let (context, docType) = try parse(properties: [
            "recipientId": ["type": "identifier", "position": 0]
        ])

        let row = try property("recipientId", in: context)
        XCTAssertEqual(row.type, "array")
        XCTAssertTrue(row.byteArray)
        XCTAssertEqual(row.minItems, 32)
        XCTAssertEqual(row.maxItems, 32)
        XCTAssertEqual(row.contentMediaType, Self.identifierMediaType)

        // The persisted schema and properties hold the long form too
        assertIdentifierLongForm(docType.properties?["recipientId"], position: 0)
        assertIdentifierLongForm(
            (docType.schema?["properties"] as? [String: Any])?["recipientId"], position: 0)
    }

    func testShouldPersistABytesPropertyAsAByteArrayRowOfItsSize() throws {
        let (context, docType) = try parse(properties: [
            "txHash": ["type": "bytes", "size": 20, "position": 0]
        ])

        let row = try property("txHash", in: context)
        XCTAssertEqual(row.type, "array")
        XCTAssertTrue(row.byteArray)
        XCTAssertEqual(row.minItems, 20)
        XCTAssertEqual(row.maxItems, 20)
        XCTAssertNil(row.contentMediaType)

        assertBytesLongForm(docType.properties?["txHash"], size: 20, position: 0)
        // A byte array is never a typed array
        XCTAssertNil(docType.typedArray(named: "txHash"))
    }

    // MARK: - Object members

    func testShouldWriteAnIdentifierObjectMemberInFull() throws {
        let (context, docType) = try parse(properties: [
            "payment": [
                "type": "object",
                "properties": [
                    "to": ["type": "identifier", "position": 0],
                    "memo": ["type": "string", "maxLength": 32, "position": 1]
                ],
                "additionalProperties": false,
                "position": 0
            ]
        ])

        XCTAssertEqual(try property("payment", in: context).type, "object")
        let members = try XCTUnwrap(
            (docType.properties?["payment"] as? [String: Any])?["properties"] as? [String: Any])
        assertIdentifierLongForm(members["to"], position: 0)
        let memo = try XCTUnwrap(members["memo"] as? [String: Any])
        XCTAssertEqual(memo["type"] as? String, "string")
    }

    func testShouldWriteABytesObjectMemberInFullAtAnyDepth() throws {
        let (_, docType) = try parse(properties: [
            "receipt": [
                "type": "object",
                "properties": [
                    "proof": [
                        "type": "object",
                        "properties": [
                            "hash": ["type": "bytes", "size": 20, "position": 0]
                        ],
                        "additionalProperties": false,
                        "position": 0
                    ]
                ],
                "additionalProperties": false,
                "position": 0
            ]
        ])

        let receipt = try XCTUnwrap(docType.schema?["properties"] as? [String: Any])["receipt"]
        let proof = try XCTUnwrap(
            ((receipt as? [String: Any])?["properties"] as? [String: Any])?["proof"] as? [String: Any])
        assertBytesLongForm(
            (proof["properties"] as? [String: Any])?["hash"], size: 20, position: 0)
    }

    // MARK: - Typed array items

    func testShouldReadIdentifierItemsAsIdentifierElements() throws {
        let (context, docType) = try parse(properties: [
            "reasons": [
                "type": "array",
                "items": ["type": "identifier"],
                "maxItems": 4,
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
                maxItems: 4,
                uniqueItems: true
            )
        )
        // The array itself stays a typed array row, not a byte array
        let row = try property("reasons", in: context)
        XCTAssertEqual(row.type, "array")
        XCTAssertFalse(row.byteArray)
        XCTAssertEqual(row.maxItems, 4)
    }

    func testShouldReadBytesItemsAsByteArrayElementsOfTheirSize() throws {
        let (_, docType) = try parse(properties: [
            "hashes": [
                "type": "array",
                "items": ["type": "bytes", "size": 20],
                "minItems": 1,
                "maxItems": 8,
                "position": 0
            ]
        ])

        XCTAssertEqual(
            docType.typedArray(named: "hashes"),
            DocumentTypedArray(
                path: "hashes",
                element: .byteArray(minSize: 20, maxSize: 20),
                minItems: 1,
                maxItems: 8,
                uniqueItems: false
            )
        )
        XCTAssertEqual(docType.typedArrays.map(\.path), ["hashes"])
    }

    // MARK: - References to definitions

    func testShouldPersistAReferenceToAnIdentifierDefinitionAsTheLongFormRow() throws {
        let (context, _) = try parse(
            properties: ["owner": ["$ref": "#/$defs/owner", "position": 0]],
            schemaDefs: ["owner": ["type": "identifier"]]
        )

        let row = try property("owner", in: context)
        XCTAssertEqual(row.type, "array")
        XCTAssertTrue(row.byteArray)
        XCTAssertEqual(row.minItems, 32)
        XCTAssertEqual(row.maxItems, 32)
        XCTAssertEqual(row.contentMediaType, Self.identifierMediaType)
    }

    func testShouldPersistAReferenceToABytesDefinitionAsTheLongFormRow() throws {
        let (context, _) = try parse(
            properties: ["hash": ["$ref": "#/$defs/hash", "position": 0]],
            schemaDefs: ["hash": ["type": "bytes", "size": 20, "description": "A hash"]]
        )

        let row = try property("hash", in: context)
        XCTAssertEqual(row.type, "array")
        XCTAssertTrue(row.byteArray)
        XCTAssertEqual(row.minItems, 20)
        XCTAssertEqual(row.maxItems, 20)
        XCTAssertNil(row.contentMediaType)
        XCTAssertEqual(row.fieldDescription, "A hash")
    }

    /// The contract's own JSON shape names the document types `documents`
    /// and the definitions `$defs`; both are read the same way.
    func testShouldReadTheContractsOwnDocumentsAndDefsKeys() throws {
        let (context, _) = try parse(contract: [
            "documents": [
                "note": [
                    "type": "object",
                    "properties": [
                        "author": ["type": "identifier", "position": 0],
                        "digest": ["$ref": "#/$defs/digest", "position": 1]
                    ],
                    "additionalProperties": false
                ]
            ],
            "$defs": ["digest": ["type": "bytes", "size": 32]]
        ])

        let author = try property("author", in: context)
        XCTAssertTrue(author.byteArray)
        XCTAssertEqual(author.contentMediaType, Self.identifierMediaType)
        let digest = try property("digest", in: context)
        XCTAssertEqual(digest.type, "array")
        XCTAssertTrue(digest.byteArray)
        XCTAssertEqual(digest.minItems, 32)
        XCTAssertEqual(digest.maxItems, 32)
    }

    // MARK: - Contracts written in full, and what the caller keeps

    func testShouldParseAContractWrittenInFullExactlyAsBefore() throws {
        let properties: [String: Any] = [
            "recipientId": [
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": Self.identifierMediaType,
                "position": 0
            ],
            "txHash": [
                "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20, "position": 1
            ],
            "size": ["type": "integer", "minimum": 0, "position": 2],
            "title": ["type": "string", "maxLength": 63, "position": 3]
        ]
        let typeDict: [String: Any] = [
            "type": "object",
            "properties": properties,
            "required": ["title"],
            "additionalProperties": false
        ]
        let (context, docType) = try parse(contract: contractJSON(documentType: typeDict))

        // The schema and properties are stored as given
        XCTAssertEqual(docType.schema as NSDictionary?, typeDict as NSDictionary)
        XCTAssertEqual(docType.properties as NSDictionary?, properties as NSDictionary)

        let recipient = try property("recipientId", in: context)
        XCTAssertEqual(recipient.type, "array")
        XCTAssertTrue(recipient.byteArray)
        XCTAssertEqual(recipient.minItems, 32)
        XCTAssertEqual(recipient.maxItems, 32)
        XCTAssertEqual(recipient.contentMediaType, Self.identifierMediaType)
        let txHash = try property("txHash", in: context)
        XCTAssertTrue(txHash.byteArray)
        XCTAssertEqual(txHash.minItems, 20)
        XCTAssertNil(txHash.contentMediaType)
        // A property named `size` is a property, not a shorthand's size
        XCTAssertEqual(try property("size", in: context).type, "integer")
        let title = try property("title", in: context)
        XCTAssertEqual(title.type, "string")
        XCTAssertEqual(title.maxLength, 63)
        XCTAssertTrue(title.isRequired)
    }

    /// The long form is a view the parser reads, never what the caller keeps:
    /// the dictionary handed in and the contract JSON stored beside it
    /// (`PersistentDataContract.serializedContract`) still hold the shorthand.
    func testShouldLeaveTheContractTheCallerHoldsAsSent() throws {
        let contract = contractJSON(documentType: [
            "type": "object",
            "properties": ["recipientId": ["type": "identifier", "position": 0]],
            "additionalProperties": false
        ])
        let (context, docType) = try parse(contract: contract)

        XCTAssertEqual(try property("recipientId", in: context).type, "array")
        XCTAssertEqual(recipientType(in: contract), "identifier")

        let stored = try XCTUnwrap(docType.dataContract?.parsedContract)
        XCTAssertEqual(recipientType(in: stored), "identifier")
    }

    /// A dictionary JSON cannot hold (here, raw `Data`) is read as given
    /// rather than raising inside `JSONSerialization`.
    func testShouldReadAContractJSONCannotHoldAsGiven() {
        let contract: [String: Any] = [
            "documentSchemas": [
                "note": ["type": "object", "properties": ["blob": Data([1, 2, 3])]]
            ]
        ]

        let read = DataContractParser.expandingPropertyTypeShorthands(in: contract)

        XCTAssertEqual(read as NSDictionary, contract as NSDictionary)
    }

    // MARK: - Helpers

    /// A contract shaped like the JSON `dash_sdk_data_contract_fetch_with_serialization`
    /// returns, declaring one document type `note`.
    private func contractJSON(
        documentType: [String: Any],
        schemaDefs: [String: Any]? = nil
    ) -> [String: Any] {
        [
            "$formatVersion": "1",
            "id": contractId.toBase58String(),
            "version": 1,
            "config": ["$formatVersion": "1", "canBeDeleted": false, "readonly": false],
            "schemaDefs": schemaDefs.map { $0 as Any } ?? NSNull(),
            "documentSchemas": ["note": documentType]
        ]
    }

    private func parse(
        properties: [String: Any],
        schemaDefs: [String: Any]? = nil
    ) throws -> (ModelContext, PersistentDocumentType) {
        try parse(contract: contractJSON(
            documentType: [
                "type": "object",
                "properties": properties,
                "additionalProperties": false
            ],
            schemaDefs: schemaDefs
        ))
    }

    /// Run the real parser over `contract` and hand back the context and the
    /// persisted document type row. `parseDocumentTypes` needs the
    /// `PersistentDataContract` row to exist first; it keeps `contract` as its
    /// stored JSON, as `ContractDownloader` does.
    private func parse(
        contract: [String: Any]
    ) throws -> (ModelContext, PersistentDocumentType) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        let persistent = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: try JSONSerialization.data(withJSONObject: contract),
            network: .testnet
        )
        context.insert(persistent)
        try context.save()

        try DataContractParser.parseDataContract(
            contractData: contract,
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

    private func property(_ name: String, in context: ModelContext) throws -> PersistentProperty {
        let id = contractId
        let descriptor = FetchDescriptor<PersistentProperty>(
            predicate: #Predicate { $0.contractId == id }
        )
        return try XCTUnwrap(
            try context.fetch(descriptor).first { $0.name == name },
            "parser should have persisted the property \(name)"
        )
    }

    /// The `type` the `recipientId` property of `note` declares in `contract`.
    private func recipientType(in contract: [String: Any]) -> String? {
        let note = (contract["documentSchemas"] as? [String: Any])?["note"] as? [String: Any]
        let recipient = (note?["properties"] as? [String: Any])?["recipientId"] as? [String: Any]
        return recipient?["type"] as? String
    }

    /// `schema` is the long form of `"type": "identifier"`, `position` kept.
    private func assertIdentifierLongForm(
        _ schema: Any?,
        position: Int,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        guard let schema = schema as? [String: Any] else {
            return XCTFail("expected a property schema, got \(String(describing: schema))",
                           file: file, line: line)
        }
        XCTAssertEqual(schema["type"] as? String, "array", file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonBool(schema["byteArray"]), true, file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonInteger(schema["minItems"]), 32, file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonInteger(schema["maxItems"]), 32, file: file, line: line)
        XCTAssertEqual(
            schema["contentMediaType"] as? String, Self.identifierMediaType, file: file, line: line)
        XCTAssertEqual(
            DocumentTypedArray.jsonInteger(schema["position"]), position, file: file, line: line)
        XCTAssertEqual(schema.count, 6, "no keyword beyond the long form: \(schema)", file: file, line: line)
    }

    /// `schema` is the long form of `"type": "bytes", "size": size`,
    /// `position` kept and `size` gone.
    private func assertBytesLongForm(
        _ schema: Any?,
        size: Int,
        position: Int,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        guard let schema = schema as? [String: Any] else {
            return XCTFail("expected a property schema, got \(String(describing: schema))",
                           file: file, line: line)
        }
        XCTAssertEqual(schema["type"] as? String, "array", file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonBool(schema["byteArray"]), true, file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonInteger(schema["minItems"]), size, file: file, line: line)
        XCTAssertEqual(DocumentTypedArray.jsonInteger(schema["maxItems"]), size, file: file, line: line)
        XCTAssertEqual(
            DocumentTypedArray.jsonInteger(schema["position"]), position, file: file, line: line)
        XCTAssertNil(schema["size"], file: file, line: line)
        XCTAssertNil(schema["contentMediaType"], file: file, line: line)
        XCTAssertEqual(schema.count, 5, "no keyword beyond the long form: \(schema)", file: file, line: line)
    }
}
