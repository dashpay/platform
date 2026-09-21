import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Coverage for the protocol-version-14 `immutable` /
/// `immutableAllowSetting` keywords on a mutable document type: the parse
/// (`DataContractParser` persists the whole type dictionary, and
/// `PersistentDocumentType.immutability` reads the keywords back off it) and
/// the lock the replace forms apply.
///
/// Consensus refuses a replace that changes, adds or removes a frozen
/// property with `DocumentImmutablePropertyChangedError` (state error code
/// 40128) and still charges for the transition, so the forms must lock
/// exactly what DPP freezes: every `immutable` entry, except an
/// `immutableAllowSetting` entry on a document that has no value for it yet.
@MainActor
final class DocumentTypeImmutabilityTests: XCTestCase {

    private let contractId = Data(repeating: 0xC1, count: 32)

    // MARK: - Parsing

    func testBothListsAreParsedAndSorted() throws {
        let docType = try parseSingleDocumentType([
            "type": "object",
            "documentsMutable": true,
            "properties": [
                "author": ["type": "string", "position": 0],
                "mood": ["type": "string", "position": 1]
            ],
            "immutable": ["mood", "author"],
            "immutableAllowSetting": ["mood"]
        ])

        XCTAssertEqual(docType.immutableProperties, ["author", "mood"])
        XCTAssertEqual(docType.immutableAllowSetting, ["mood"])
        XCTAssertFalse(docType.immutability.isEmpty)
    }

    func testAbsentKeywordsFreezeNothing() throws {
        let docType = try parseSingleDocumentType([
            "type": "object",
            "documentsMutable": true,
            "properties": ["author": ["type": "string", "position": 0]]
        ])

        XCTAssertEqual(docType.immutableProperties, [])
        XCTAssertEqual(docType.immutableAllowSetting, [])
        XCTAssertTrue(docType.immutability.isEmpty)
    }

    /// The SDK persists the keywords as authored and does not validate them:
    /// DPP refuses a contract whose allowance names a property outside
    /// `immutable`, so the pairing is consensus's business, not the client's.
    /// The allowance still unlocks nothing on its own, which
    /// `testAllowSettingOutsideImmutableUnlocksNothing` pins.
    func testAllowSettingWithoutImmutableIsParsedAsGiven() throws {
        let docType = try parseSingleDocumentType([
            "type": "object",
            "documentsMutable": true,
            "properties": ["mood": ["type": "string", "position": 0]],
            "immutableAllowSetting": ["mood"]
        ])

        XCTAssertEqual(docType.immutableProperties, [])
        XCTAssertEqual(docType.immutableAllowSetting, ["mood"])
    }

    func testNonStringEntriesAreIgnored() {
        let immutability = DocumentTypeImmutability(documentTypeSchema: [
            "immutable": ["author", 7]
        ])

        XCTAssertEqual(immutability.immutableProperties, ["author"])
    }

    func testDuplicateEntriesCollapse() {
        let immutability = DocumentTypeImmutability(documentTypeSchema: [
            "immutable": ["author", "author", "mood"],
            "immutableAllowSetting": ["mood", "mood"]
        ])

        XCTAssertEqual(immutability.immutableProperties, ["author", "mood"])
        XCTAssertEqual(immutability.immutableAllowSetting, ["mood"])
    }

    // MARK: - Lock state

    func testFrozenPropertyIsLockedWhateverTheStoredDocumentHolds() {
        let immutability = DocumentTypeImmutability(
            immutable: ["author"], allowSetting: [])

        XCTAssertEqual(
            immutability.lockState(for: "author", hasStoredValue: false), .frozen)
        XCTAssertEqual(
            immutability.lockState(for: "author", hasStoredValue: true), .frozen)
    }

    func testSettableOnceOnlyWhileTheStoredDocumentHasNoValue() {
        let immutability = DocumentTypeImmutability(
            immutable: ["author", "mood"], allowSetting: ["mood"])

        XCTAssertEqual(
            immutability.lockState(for: "mood", hasStoredValue: false), .settableOnce)
        XCTAssertEqual(
            immutability.lockState(for: "mood", hasStoredValue: true), .frozen)
    }

    func testPropertyOutsideBothListsStaysEditable() {
        let immutability = DocumentTypeImmutability(
            immutable: ["author"], allowSetting: [])

        XCTAssertEqual(
            immutability.lockState(for: "body", hasStoredValue: true), .editable)
        XCTAssertEqual(
            DocumentTypeImmutability.none.lockState(for: "body", hasStoredValue: false),
            .editable)
    }

    /// A hand-edited contract can list an allowance for a property that is not
    /// immutable. It must not unlock anything: the property was editable
    /// already, and nothing else moves.
    func testAllowSettingOutsideImmutableUnlocksNothing() {
        let immutability = DocumentTypeImmutability(
            immutable: ["author"], allowSetting: ["body"])

        XCTAssertEqual(
            immutability.lockState(for: "body", hasStoredValue: false), .editable)
        XCTAssertEqual(
            immutability.lockState(for: "author", hasStoredValue: false), .frozen)
    }

    // MARK: - Helpers

    /// Run the real parser over a one-document-type contract and hand back the
    /// persisted row. `parseDocumentTypes` needs the `PersistentDataContract`
    /// row to exist first (document types hang off that relationship).
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
            contractData: ["documents": ["post": typeDict]],
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
