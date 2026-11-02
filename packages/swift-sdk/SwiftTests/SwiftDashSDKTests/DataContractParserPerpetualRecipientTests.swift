import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `DataContractParser`'s perpetual-distribution recipient
/// parsing.
///
/// rs-dpp serialises `TokenDistributionRecipient` with a custom `Serialize`
/// that emits a flat, `$type`-tagged map (since 4.0.0-beta.4), and the FFI
/// hands that same serde output to Swift:
///
/// ```json
/// "perpetualDistribution": {
///   "$formatVersion": "0",
///   "distributionType": {
///     "$type": "blockBasedDistribution",
///     "interval": 1000,
///     "function": { "$type": "fixedAmount", "amount": 100 }
///   },
///   "distributionRecipient": { "$type": "contractOwner" }
/// }
/// ```
///
/// The parser used to read `distributionRecipient` as a bare string
/// ("ContractOwner"), a shape that is neither emitted nor accepted any more,
/// so the branch was dead and every real contract kept the init default
/// "AllEqualShare". These tests drive the public `parseDataContract` entry
/// point against an in-memory `ModelContainer` (no network, no FFI handle)
/// and assert the canonical string that lands on the persisted
/// `PersistentToken`.
@MainActor
final class DataContractParserPerpetualRecipientTests: XCTestCase {

    private let contractId = Data(repeating: 0xAB, count: 32)

    /// A canonical base58 identity id, the shape rs-dpp writes for the
    /// `identity` variant.
    private let recipientBase58 = "29n6ZVEWvTVM7BaQCs7Ubsi4KdnqkPBTQ8QxEyyXciAc"

    private func makeContext() throws -> ModelContext {
        let container = try DashModelContainer.createInMemory()
        return ModelContext(container)
    }

    /// The parser's `parseTokens` bails out early unless a
    /// `PersistentDataContract` row already exists for `contractId`
    /// (tokens hang off the contract relationship). Seed that row, then
    /// run the parser and hand back the single parsed token.
    private func parseSingleToken(
        tokenDict: [String: Any],
        in context: ModelContext
    ) throws -> PersistentToken {
        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: Data(),
            network: .testnet
        )
        context.insert(contract)
        try context.save()

        let contractData: [String: Any] = [
            "tokens": ["0": tokenDict]
        ]

        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractId,
            modelContext: context
        )

        let id = contractId
        let descriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.contractId == id }
        )
        let tokens = try context.fetch(descriptor)
        return try XCTUnwrap(tokens.first, "parser should have persisted one token")
    }

    /// The real perpetual block as rs-dpp emits it, with only the
    /// `distributionRecipient` payload varying per test.
    private func tokenDict(recipient: Any) -> [String: Any] {
        return [
            "baseSupply": 0,
            "distributionRules": [
                "perpetualDistribution": [
                    "$formatVersion": "0",
                    "distributionType": [
                        "$type": "blockBasedDistribution",
                        "interval": 1000,
                        "function": ["$type": "fixedAmount", "amount": 100]
                    ],
                    "distributionRecipient": recipient
                ]
            ]
        ]
    }

    private func parseRecipient(_ recipient: Any) throws -> String {
        let context = try makeContext()
        let token = try parseSingleToken(
            tokenDict: tokenDict(recipient: recipient),
            in: context
        )
        let perpetual = try XCTUnwrap(
            token.perpetualDistribution,
            "a perpetualDistribution block should have been persisted"
        )
        return perpetual.distributionRecipient
    }

    // MARK: - 1. contractOwner

    /// `{"$type": "contractOwner"}` should store the canonical
    /// "ContractOwner" string, matching `AuthorizedActionTakers`.
    func testContractOwnerRecipientStoresCanonicalString() throws {
        let stored = try parseRecipient(["$type": "contractOwner"])
        XCTAssertEqual(stored, "ContractOwner")
        XCTAssertEqual(stored, TokenDistributionRecipient.contractOwner.rawValue)
    }

    // MARK: - 2. identity

    /// `{"$type": "identity", "identity": "<base58>"}` should store
    /// "Identity:<base58>" with the base58 id taken verbatim from the JSON.
    func testIdentityRecipientStoresPrefixedBase58() throws {
        let stored = try parseRecipient([
            "$type": "identity",
            "identity": recipientBase58
        ])
        XCTAssertEqual(stored, "Identity:" + recipientBase58)
        XCTAssertEqual(stored, TokenDistributionRecipient.identity(recipientBase58))
    }

    // MARK: - 3. evonodesByParticipation

    /// `{"$type": "evonodesByParticipation"}` should store the canonical
    /// "EvonodesByParticipation" string.
    func testEvonodesByParticipationRecipientStoresCanonicalString() throws {
        let stored = try parseRecipient(["$type": "evonodesByParticipation"])
        XCTAssertEqual(stored, "EvonodesByParticipation")
        XCTAssertEqual(stored, TokenDistributionRecipient.evonodesByParticipation.rawValue)
    }

    // MARK: - 4. Unknown variant

    /// A `$type` this build does not know about (a variant added by a newer
    /// protocol version) should be stored verbatim, so the value is visible
    /// instead of silently replaced by the init default.
    func testUnknownRecipientTypeIsStoredVerbatim() throws {
        let stored = try parseRecipient(["$type": "somethingNew"])
        XCTAssertEqual(stored, "somethingNew")
    }

    // MARK: - 5. Malformed identity variant

    /// An `identity` variant whose `identity` field is missing must not
    /// crash. The chosen fallback is the raw `$type` discriminator, the same
    /// treatment an unrecognised variant gets: it records that the recipient
    /// is some identity without inventing a base58 id, and it never claims
    /// the default "AllEqualShare".
    func testIdentityRecipientWithoutIdentityFieldFallsBackToRawType() throws {
        let stored = try parseRecipient(["$type": "identity"])
        XCTAssertEqual(stored, "identity")
        XCTAssertNotEqual(stored, "AllEqualShare")
    }

    // MARK: - 6. Surrounding perpetual fields

    /// The same fixture should still land `distributionType` as a JSON
    /// string and default `enabled` to true, so the recipient change did not
    /// disturb the rest of the perpetual block.
    func testDistributionTypeStaysJsonStringAndEnabledDefaultsTrue() throws {
        let context = try makeContext()
        let token = try parseSingleToken(
            tokenDict: tokenDict(recipient: ["$type": "contractOwner"]),
            in: context
        )

        let perpetual = try XCTUnwrap(token.perpetualDistribution)
        XCTAssertTrue(perpetual.enabled, "enabled defaults to true when absent")

        let typeData = try XCTUnwrap(
            perpetual.distributionType.data(using: .utf8),
            "distributionType should be stored as a JSON string"
        )
        let decoded = try XCTUnwrap(
            JSONSerialization.jsonObject(with: typeData) as? [String: Any],
            "distributionType should decode back to a JSON object"
        )
        XCTAssertEqual(decoded["$type"] as? String, "blockBasedDistribution")
        XCTAssertEqual(decoded["interval"] as? Int, 1000)

        let function = try XCTUnwrap(decoded["function"] as? [String: Any])
        XCTAssertEqual(function["$type"] as? String, "fixedAmount")
        XCTAssertEqual(function["amount"] as? Int, 100)
    }
}
