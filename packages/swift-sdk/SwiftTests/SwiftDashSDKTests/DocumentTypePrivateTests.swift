import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Coverage for the protocol-version-15 `private` keyword: `DataContractParser`
/// persists the whole type dictionary, and `PersistentDocumentType.isPrivate`
/// and `privateEntrySize` read the keyword back off it without a column of
/// their own.
@MainActor
final class DocumentTypePrivateTests: XCTestCase {

    private let contractId = Data(repeating: 0xC3, count: 32)

    func testShouldReadAPrivateTypesEntrySizeThroughTheParser() throws {
        let docType = try parseSingleDocumentType([
            "type": "object",
            "private": ["entrySize": 400],
            "documentsMutable": false,
            "canBeDeleted": false,
            "properties": [
                "text": ["type": "string", "maxLength": 40, "position": 0]
            ],
            "required": ["text"],
            "additionalProperties": false
        ])

        XCTAssertTrue(docType.isPrivate)
        XCTAssertEqual(docType.privateEntrySize, 400)
    }

    func testShouldReadATypeWithoutTheKeywordAsNotPrivate() throws {
        let docType = try parseSingleDocumentType([
            "type": "object",
            "properties": [
                "text": ["type": "string", "maxLength": 63, "position": 0]
            ],
            "additionalProperties": false
        ])

        XCTAssertFalse(docType.isPrivate)
        XCTAssertNil(docType.privateEntrySize)
    }

    func testShouldReadNoEntrySizeFromADeclarationWithoutAnInteger() {
        let malformed: [[String: Any]] = [
            ["private": [:] as [String: Any]],
            ["private": ["entrySize": "400"]],
            ["private": ["entrySize": true]],
            ["private": ["entrySize": 400.5]],
            ["private": true]
        ]
        for schema in malformed {
            XCTAssertTrue(PersistentDocumentType.declaresPrivate(inDocumentTypeSchema: schema))
            XCTAssertNil(
                PersistentDocumentType.privateEntrySize(inDocumentTypeSchema: schema),
                "\(schema)"
            )
        }
    }

    func testShouldReadNothingFromAMissingSchema() {
        XCTAssertFalse(PersistentDocumentType.declaresPrivate(inDocumentTypeSchema: nil))
        XCTAssertNil(PersistentDocumentType.privateEntrySize(inDocumentTypeSchema: nil))
    }

    // MARK: - Helpers

    private func parseSingleDocumentType(
        _ typeDict: [String: Any]
    ) throws -> PersistentDocumentType {
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
            contractData: ["documents": ["secret": typeDict]],
            contractId: contractId,
            modelContext: context
        )

        let id = contractId
        let descriptor = FetchDescriptor<PersistentDocumentType>(
            predicate: #Predicate { $0.contractId == id }
        )
        let types = try context.fetch(descriptor)
        return try XCTUnwrap(types.first, "parser should have persisted one document type")
    }
}
