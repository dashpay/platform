import XCTest
import SwiftData
import SwiftDashSDK
@testable import SwiftExampleApp

/// `ContractDownloader.apply`, the part of `refreshPersistedContract` after
/// the network fetch, run against an in-memory store with the container's
/// main context standing in for the app's.
///
/// The refresh writes in a context of its own: a refused refresh leaves an
/// unrelated pending edit in the main context alone (no rollback of it), a
/// successful one saves only its own rows, and the main context sees them.
@MainActor
final class ContractRefreshApplyTests: XCTestCase {

    private let contractId = Data(repeating: 0xC7, count: 32)
    private let otherContractId = Data(repeating: 0xC8, count: 32)

    private let token: [String: Any] = [
        "baseSupply": 1000,
        "conventions": [
            "decimals": 2,
            "localizations": ["en": ["singularForm": "Coin", "pluralForm": "Coins"]]
        ]
    ]

    /// A stored contract at version 2 with keyword `alpha` and description
    /// `Old`, and another contract whose name has an unsaved edit.
    private struct Fixture {
        let container: ModelContainer
        let contract: PersistentDataContract
        let other: PersistentDataContract
    }

    private func makeFixture() throws -> Fixture {
        let container = try DashModelContainer.createInMemory()
        let main = container.mainContext
        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: Data("{}".utf8),
            version: 2,
            keywords: ["alpha"],
            description: "Old",
            network: .testnet
        )
        let other = PersistentDataContract(
            id: otherContractId,
            name: "Other",
            serializedContract: Data("{}".utf8),
            network: .testnet
        )
        main.insert(contract)
        main.insert(other)
        try main.save()

        other.name = "Edited"
        XCTAssertTrue(main.hasChanges)
        return Fixture(container: container, contract: contract, other: other)
    }

    private func fetched(_ fields: [String: Any]) throws -> ContractDownloader.FetchedContract {
        var contractData = fields
        contractData["id"] = contractId.toBase58String()
        return ContractDownloader.FetchedContract(
            jsonData: try JSONSerialization.data(withJSONObject: contractData),
            contractData: contractData,
            binaryData: Data([0x01, 0x02])
        )
    }

    /// The stored row as a fresh context reads it from the store.
    private func storedRow(_ id: Data, in container: ModelContainer) throws -> PersistentDataContract {
        try XCTUnwrap(
            try ModelContext(container).fetch(
                FetchDescriptor<PersistentDataContract>(predicate: #Predicate { $0.id == id })
            ).first
        )
    }

    /// Run the main run loop until `condition` holds or two seconds pass,
    /// for the main context to merge another context's save.
    private func waitForMainContext(_ condition: () -> Bool) {
        let deadline = Date().addingTimeInterval(2)
        while !condition() && Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
    }

    // MARK: - Refused

    func testShouldRefuseAnOlderContractAndLeaveTheMainContextsPendingEdit() throws {
        let fixture = try makeFixture()
        defer { withExtendedLifetime(fixture.container) {} }
        let main = fixture.container.mainContext

        // Below the stored version; at the stored version but below the
        // version the caller knows Platform holds.
        let refusals: [(version: Int, minimumVersion: Int?)] = [(1, nil), (1, 3), (2, 3)]
        for refusal in refusals {
            var registered = false
            XCTAssertThrowsError(
                try ContractDownloader.apply(
                    fetched: try fetched(["version": refusal.version, "description": "Older"]),
                    toContractWithId: contractId,
                    minimumVersion: refusal.minimumVersion,
                    container: fixture.container
                ) { _ in registered = true },
                "\(refusal)"
            ) { error in
                guard case ContractDownloadError.fetchFailed = error else {
                    return XCTFail("expected fetchFailed, got \(error)")
                }
            }
            XCTAssertFalse(registered, "an older binary must not reach the trusted context")
        }

        XCTAssertTrue(main.hasChanges)
        XCTAssertEqual(fixture.other.name, "Edited")
        XCTAssertEqual(fixture.contract.version, 2)
        XCTAssertEqual(fixture.contract.contractDescription, "Old")

        let stored = try storedRow(contractId, in: fixture.container)
        XCTAssertEqual(stored.version, 2)
        XCTAssertEqual(stored.contractDescription, "Old")
        XCTAssertEqual(try storedRow(otherContractId, in: fixture.container).name, "Other")
    }

    // MARK: - Applied

    /// An update that changed only the description and keywords.
    func testShouldRefreshTheDescriptionAndKeywordsOfAMetadataOnlyChange() throws {
        let fixture = try makeFixture()
        defer { withExtendedLifetime(fixture.container) {} }
        let main = fixture.container.mainContext

        var registered: Data?
        try ContractDownloader.apply(
            fetched: try fetched(["version": 3, "description": "New", "keywords": ["beta", "gamma"]]),
            toContractWithId: contractId,
            minimumVersion: 3,
            container: fixture.container
        ) { registered = $0 }
        XCTAssertEqual(registered, Data([0x01, 0x02]))

        let stored = try storedRow(contractId, in: fixture.container)
        XCTAssertEqual(stored.version, 3)
        XCTAssertEqual(stored.contractDescription, "New")
        XCTAssertEqual(stored.keywords.sorted(), ["beta", "gamma"])
        XCTAssertFalse(stored.hasTokens)
        let base58 = contractId.toBase58String()
        let keywordRows = try ModelContext(fixture.container).fetch(
            FetchDescriptor<PersistentKeyword>(predicate: #Predicate { $0.contractId == base58 })
        )
        XCTAssertEqual(keywordRows.map(\.keyword).sorted(), ["beta", "gamma"])

        // The refresh saved its own rows only: the main context's edit is
        // still pending, and not in the store.
        XCTAssertTrue(main.hasChanges)
        XCTAssertEqual(fixture.other.name, "Edited")
        XCTAssertEqual(try storedRow(otherContractId, in: fixture.container).name, "Other")

        // The main context sees the refreshed row.
        waitForMainContext { fixture.contract.version == 3 }
        XCTAssertEqual(fixture.contract.version, 3)
        XCTAssertEqual(fixture.contract.contractDescription, "New")
        XCTAssertEqual(fixture.contract.keywords.sorted(), ["beta", "gamma"])
    }

    /// An update that added the contract's first token; the fetched contract
    /// drops the description and keywords.
    func testShouldRecordAFirstTokenAndClearAnAbsentDescription() throws {
        let fixture = try makeFixture()
        defer { withExtendedLifetime(fixture.container) {} }

        try ContractDownloader.apply(
            fetched: try fetched(["version": 3, "tokens": ["0": token]]),
            toContractWithId: contractId,
            minimumVersion: nil,
            container: fixture.container
        ) { _ in }

        let stored = try storedRow(contractId, in: fixture.container)
        XCTAssertEqual(stored.version, 3)
        XCTAssertTrue(stored.hasTokens)
        XCTAssertEqual(stored.tokens?.count, 1)
        XCTAssertNil(stored.contractDescription)
        XCTAssertEqual(stored.keywords, [])

        waitForMainContext { fixture.contract.hasTokens }
        XCTAssertTrue(fixture.contract.hasTokens)
        XCTAssertEqual(fixture.contract.tokens?.count, 1)
    }
}
