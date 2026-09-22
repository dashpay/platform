import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Protocol version 14 adds typed arrays to document schemas: a
/// `type: "array"` property declared by an `items` schema instead of
/// `byteArray: true`. The Swift SDK does not support them yet, and
/// `PersistentProperty` has no element type, so `DataContractParser` must
/// refuse such a contract with `ParseError.unsupportedTypedArray` instead of
/// persisting the property as a bare array. Byte arrays parse as before.
@MainActor
final class DataContractParserTypedArrayTests: XCTestCase {

    private let contractId = Data(repeating: 0xC2, count: 32)

    func testTypedArrayOfScalarsIsRefused() throws {
        try assertRefused(property: "tags", schema: [
            "type": "array",
            "items": ["type": "string", "maxLength": 32],
            "maxItems": 8,
            "position": 0
        ])
    }

    /// An element that is itself a byte array (here an identifier) does not
    /// make the property a byte array: the property declares `items`, and only
    /// its elements declare `byteArray`.
    func testTypedArrayOfIdentifiersIsRefused() throws {
        try assertRefused(property: "reasons", schema: [
            "type": "array",
            "items": [
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier"
            ],
            "maxItems": 64,
            "uniqueItems": true,
            "position": 0
        ])
    }

    func testByteArrayStillParses() throws {
        let context = try makeContext()

        try parse(properties: [
            "owner": [
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0
            ]
        ], in: context)

        let property = try XCTUnwrap(
            try fetchProperties(in: context).first { $0.name == "owner" },
            "parser should have persisted the byte array property"
        )
        XCTAssertEqual(property.type, "array")
        XCTAssertTrue(property.byteArray)
        XCTAssertEqual(property.minItems, 32)
        XCTAssertEqual(property.maxItems, 32)
        XCTAssertEqual(property.contentMediaType, "application/x.dash.dpp.identifier")
    }

    // MARK: - Helpers

    private func assertRefused(
        property name: String,
        schema: [String: Any],
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        let context = try makeContext()

        XCTAssertThrowsError(
            try parse(properties: [name: schema], in: context),
            file: file,
            line: line
        ) { error in
            XCTAssertEqual(
                error as? DataContractParser.ParseError,
                .unsupportedTypedArray(documentType: "post", property: name),
                file: file,
                line: line
            )
            let message = error.localizedDescription
            XCTAssertTrue(message.contains("typed arrays"), message, file: file, line: line)
            XCTAssertTrue(message.contains("document type post"), message, file: file, line: line)
            XCTAssertTrue(message.contains("property \(name)"), message, file: file, line: line)
        }

        // Refused, not mis-parsed: no row stands in for the typed array
        XCTAssertFalse(
            try fetchProperties(in: context).contains { $0.name == name },
            "a typed array must not be persisted as a property",
            file: file,
            line: line
        )
    }

    private func makeContext() throws -> ModelContext {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        // Document types hang off the contract row, so it must exist first
        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: Data(),
            network: .testnet
        )
        context.insert(contract)
        try context.save()
        return context
    }

    private func parse(properties: [String: Any], in context: ModelContext) throws {
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
    }

    private func fetchProperties(in context: ModelContext) throws -> [PersistentProperty] {
        let id = contractId
        let descriptor = FetchDescriptor<PersistentProperty>(
            predicate: #Predicate { $0.contractId == id }
        )
        return try context.fetch(descriptor)
    }
}
