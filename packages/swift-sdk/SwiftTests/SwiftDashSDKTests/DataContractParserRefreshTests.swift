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

    private func noteType(withBody: Bool, withIndex: Bool = true) -> [String: Any] {
        var properties: [String: Any] = [
            "message": ["type": "string", "maxLength": 63, "position": 0]
        ]
        if withBody {
            properties["body"] = [
                "type": "string", "maxLength": 255, "position": 1, "description": "Body"
            ]
        }
        var type: [String: Any] = [
            "type": "object",
            "properties": properties,
            "additionalProperties": false
        ]
        if withIndex {
            type["indices"] = [["name": "byMessage", "properties": [["message": "asc"]]]]
        }
        return type
    }

    private func insertFixtureContract(into context: ModelContext) throws {
        context.insert(
            PersistentDataContract(
                id: contractId,
                name: "Fixture",
                serializedContract: Data(),
                network: .testnet
            )
        )
        try context.save()
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

    /// The fixture need not be an update Platform would accept: it only
    /// checks that the stored index rows follow the re-parsed definition.
    func testShouldRemoveAnIndexTheReparsedTypeNoLongerDeclares() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }
        try insertFixtureContract(into: context)

        let id = contractId
        let indexRows = FetchDescriptor<PersistentIndex>(predicate: #Predicate { $0.contractId == id })
        for withIndex in [true, false] {
            try DataContractParser.parseDataContract(
                contractData: ["documents": ["note": noteType(withBody: false, withIndex: withIndex)]],
                contractId: contractId,
                modelContext: context
            )
            try context.save()
            XCTAssertEqual(try context.fetch(indexRows).count, withIndex ? 1 : 0)
        }

        let noteRow = try XCTUnwrap(
            try context.fetch(
                FetchDescriptor<PersistentDocumentType>(predicate: #Predicate { $0.contractId == id })
            ).first
        )
        XCTAssertEqual(noteRow.indices?.count ?? 0, 0)
    }

    /// A property or index row id is the contract id, the type name and the
    /// property or index name with no separator, so `note`.`title` and
    /// `notet`.`itle` (and the indexes `note`.`tbyItle` and `notet`.`byItle`)
    /// share one. A type added later with a colliding definition must not
    /// take over, or rewrite, the row of the type that has it.
    func testShouldKeepATypesPropertyRowWhenAnotherTypesPropertyIdCollidesWithIt() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }
        try insertFixtureContract(into: context)

        let note: [String: Any] = [
            "type": "object",
            "properties": ["title": ["type": "string", "maxLength": 10, "position": 0]],
            "indices": [["name": "tbyItle", "properties": [["title": "asc"]]]],
            "additionalProperties": false
        ]
        let notet: [String: Any] = [
            "type": "object",
            "properties": ["itle": ["type": "string", "maxLength": 99, "position": 0]],
            "indices": [["name": "byItle", "properties": [["itle": "asc"]]]],
            "additionalProperties": false
        ]
        XCTAssertEqual(
            PersistentProperty(contractId: contractId, documentTypeName: "note", name: "title", type: "string").id,
            PersistentProperty(contractId: contractId, documentTypeName: "notet", name: "itle", type: "string").id
        )

        // `notet` is added by an update, then the contract is parsed again.
        let parses: [[String: Any]] = [
            ["note": note],
            ["note": note, "notet": notet],
            ["note": note, "notet": notet]
        ]
        for documents in parses {
            try DataContractParser.parseDataContract(
                contractData: ["documents": documents],
                contractId: contractId,
                modelContext: context
            )
            try context.save()
        }

        let id = contractId
        let types = try context.fetch(
            FetchDescriptor<PersistentDocumentType>(predicate: #Predicate { $0.contractId == id })
        )
        let noteRow = try XCTUnwrap(types.first { $0.name == "note" })
        let notetRow = try XCTUnwrap(types.first { $0.name == "notet" })

        let properties = try context.fetch(
            FetchDescriptor<PersistentProperty>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(properties.count, 1)
        let title = try XCTUnwrap(properties.first)
        XCTAssertEqual(title.documentTypeName, "note")
        XCTAssertEqual(title.name, "title")
        XCTAssertEqual(title.maxLength, 10)
        XCTAssertTrue(title.documentType === noteRow)
        XCTAssertEqual(noteRow.propertiesList?.count, 1)
        XCTAssertEqual(notetRow.propertiesList?.count ?? 0, 0)

        let indices = try context.fetch(
            FetchDescriptor<PersistentIndex>(predicate: #Predicate { $0.contractId == id })
        )
        XCTAssertEqual(indices.count, 1)
        let index = try XCTUnwrap(indices.first)
        XCTAssertEqual(index.documentTypeName, "note")
        XCTAssertEqual(index.name, "tbyItle")
        XCTAssertEqual(index.properties, ["title"])
        XCTAssertTrue(index.documentType === noteRow)
        XCTAssertEqual(notetRow.indices?.count ?? 0, 0)
    }

    /// A stored token's `isPaused` is its live paused status (the pause and
    /// resume actions and the token status sync write it); the contract's
    /// `startAsPaused` only seeds it on the first parse.
    func testShouldKeepAStoredTokensPausedStatusOnAReparse() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }
        try insertFixtureContract(into: context)

        var unpausedToken = token
        unpausedToken["startAsPaused"] = false
        try DataContractParser.parseDataContract(
            contractData: ["tokens": ["0": unpausedToken]],
            contractId: contractId,
            modelContext: context
        )
        try context.save()

        let id = contractId
        let tokenRows = FetchDescriptor<PersistentToken>(predicate: #Predicate { $0.contractId == id })
        let tokenRow = try XCTUnwrap(try context.fetch(tokenRows).first)
        XCTAssertFalse(tokenRow.isPaused)
        tokenRow.isPaused = true
        try context.save()

        // The re-parsed definition changes, so the refresh does run.
        unpausedToken["conventions"] = [
            "decimals": 3,
            "localizations": ["en": ["singularForm": "Coin", "pluralForm": "Coins"]]
        ]
        try DataContractParser.parseDataContract(
            contractData: ["tokens": ["0": unpausedToken]],
            contractId: contractId,
            modelContext: context
        )
        try context.save()

        let refreshed = try context.fetch(tokenRows)
        XCTAssertEqual(refreshed.count, 1)
        XCTAssertTrue(refreshed.first === tokenRow)
        XCTAssertEqual(tokenRow.decimals, 3)
        XCTAssertTrue(tokenRow.isPaused)
    }

    /// An explicit `maxSupply` of 0 is a cap of zero, not "no cap", and a
    /// re-parse keeps that difference both ways.
    func testShouldKeepAnExplicitZeroMaxSupplyApartFromNoCapOnAReparse() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        defer { withExtendedLifetime(container) {} }
        try insertFixtureContract(into: context)

        let id = contractId
        let tokenRows = FetchDescriptor<PersistentToken>(predicate: #Predicate { $0.contractId == id })
        func parse(maxSupply: Any?) throws -> PersistentToken {
            var cappedToken = token
            if let maxSupply {
                cappedToken["maxSupply"] = maxSupply
            }
            try DataContractParser.parseDataContract(
                contractData: ["tokens": ["0": cappedToken]],
                contractId: contractId,
                modelContext: context
            )
            try context.save()
            let rows = try context.fetch(tokenRows)
            XCTAssertEqual(rows.count, 1)
            return try XCTUnwrap(rows.first)
        }

        XCTAssertEqual(try parse(maxSupply: 0).maxSupply, "0")
        XCTAssertNil(try parse(maxSupply: nil).maxSupply)
        XCTAssertEqual(try parse(maxSupply: 0).maxSupply, "0")
        XCTAssertNil(try parse(maxSupply: NSNull()).maxSupply)
        XCTAssertEqual(try parse(maxSupply: 5000).maxSupply, "5000")
    }
}
