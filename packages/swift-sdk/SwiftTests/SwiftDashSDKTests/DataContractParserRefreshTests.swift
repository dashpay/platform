import SwiftData
import XCTest

@testable import SwiftDashSDK

/// A contract parsed a second time, as the example app does after a contract
/// update, refreshes the rows of its first parse in place: no second row with
/// the same unique id, and the documents of a document type and the balances
/// of a token stay attached.
@MainActor
final class DataContractParserRefreshTests: XCTestCase {

    private let contractId = Data(repeating: 0xD4, count: 32)

    private func noteType(withBody: Bool) -> [String: Any] {
        var properties: [String: Any] = [
            "message": ["type": "string", "maxLength": 63, "position": 0]
        ]
        if withBody {
            properties["body"] = [
                "type": "string", "maxLength": 255, "position": 1, "description": "Body"
            ]
        }
        return [
            "type": "object",
            "properties": properties,
            "indices": [["name": "byMessage", "properties": [["message": "asc"]]]],
            "additionalProperties": false
        ]
    }

    private let token: [String: Any] = [
        "baseSupply": 1000,
        "conventions": [
            "decimals": 2,
            "localizations": ["en": ["singularForm": "Coin", "pluralForm": "Coins"]]
        ]
    ]

    func testShouldRefreshAReparsedContractWithoutDuplicatesAndKeepItsDependents() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }

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
                "version": 1,
                "documents": ["note": noteType(withBody: false)],
                "tokens": ["0": token]
            ],
            contractId: contractId,
            modelContext: context
        )
        try context.save()

        // State other views hang off the definitions.
        let noteRow = try XCTUnwrap(contract.documentTypes?.first { $0.name == "note" })
        let document = PersistentDocument(
            documentId: "doc1",
            documentType: "note",
            revision: 1,
            data: Data("{}".utf8),
            contractId: contractId.toBase58String(),
            ownerId: contractId.toBase58String(),
            network: .testnet
        )
        context.insert(document)
        document.dataContract = contract
        document.documentType_relation = noteRow
        let tokenRow = try XCTUnwrap(contract.tokens?.first)
        let balance = PersistentTokenBalance(
            tokenId: "token0",
            identityId: Data(repeating: 0x01, count: 32),
            balance: 42,
            network: .testnet
        )
        context.insert(balance)
        balance.token = tokenRow
        try context.save()

        // The updated contract: `note` gains a property, a type is added.
        try DataContractParser.parseDataContract(
            contractData: [
                "version": 2,
                "documents": [
                    "note": noteType(withBody: true),
                    "comment": noteType(withBody: false)
                ],
                "tokens": ["0": token],
                "groups": ["0": ["members": [:], "requiredPower": 1]]
            ],
            contractId: contractId,
            modelContext: context
        )
        try context.save()

        let id = contractId
        let types = try context.fetch(
            FetchDescriptor<PersistentDocumentType>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(types.map(\.name).sorted(), ["comment", "note"])
        let tokens = try context.fetch(
            FetchDescriptor<PersistentToken>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(tokens.count, 1)
        let indices = try context.fetch(
            FetchDescriptor<PersistentIndex>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(indices.count, 2)
        let noteProperties = try context.fetch(
            FetchDescriptor<PersistentProperty>(
                predicate: #Predicate { $0.contractId == id && $0.documentTypeName == "note" }
            )
        )
        XCTAssertEqual(noteProperties.map(\.name).sorted(), ["body", "message"])
        XCTAssertEqual(noteProperties.first { $0.name == "body" }?.fieldDescription, "Body")

        // The stored rows were refreshed, not replaced.
        XCTAssertTrue(types.contains { $0 === noteRow })
        XCTAssertTrue(tokens.first === tokenRow)
        XCTAssertNotNil(noteRow.properties?["body"])
        XCTAssertTrue(document.documentType_relation === noteRow)
        XCTAssertTrue(document.dataContract === contract)
        XCTAssertTrue(balance.token === tokenRow)
        XCTAssertEqual(tokenRow.balances?.count, 1)

        XCTAssertEqual(contract.version, 2)
        XCTAssertNotNil(contract.groups?["0"])
    }

    func testShouldRemoveAPropertyTheReparsedTypeNoLongerDeclares() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }

        context.insert(
            PersistentDataContract(
                id: contractId,
                name: "Fixture",
                serializedContract: Data(),
                network: .testnet
            )
        )
        try context.save()

        for withBody in [true, false] {
            try DataContractParser.parseDataContract(
                contractData: ["documents": ["note": noteType(withBody: withBody)]],
                contractId: contractId,
                modelContext: context
            )
            try context.save()
        }

        let id = contractId
        let properties = try context.fetch(
            FetchDescriptor<PersistentProperty>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(properties.map(\.name), ["message"])
    }
}
