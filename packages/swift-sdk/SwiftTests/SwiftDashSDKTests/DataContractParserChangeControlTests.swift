import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `DataContractParser`'s change-control-rule parsing.
///
/// rs-dpp serialises a `ChangeControlRules` value flat, tagged with its
/// format version, and serialises each `AuthorizedActionTakers` field
/// inside it as its own flat `$type`-tagged map:
///
/// ```json
/// "manualMintingRules": {
///   "$formatVersion": "0",
///   "authorizedToMakeChange": { "$type": "group", "position": 3 },
///   "adminActionTakers": { "$type": "contractOwner" },
///   "changingAuthorizedActionTakersToNoOneAllowed": true,
///   "changingAdminActionTakersToNoOneAllowed": false,
///   "selfChangingAdminActionTakersAllowed": true
/// }
/// ```
///
/// The parser previously read both action-taker fields with `as? String`,
/// a branch that never matches real contract JSON, so every rule on every
/// real contract persisted the `mostRestrictive()` default ("NoOne" for
/// both fields) and the example app reported that no one was authorized
/// for any token action.
///
/// These tests drive the public `parseDataContract` entry point against
/// an in-memory `ModelContainer` (no network, no FFI handle) and assert
/// the canonical strings land on the persisted `PersistentToken`.
@MainActor
final class DataContractParserChangeControlTests: XCTestCase {

    private let contractId = Data(repeating: 0xAB, count: 32)

    /// A canonical base58 identity id, the shape the `identity` variant
    /// carries in contract JSON.
    private let identityBase58 = "29n6ZVEWvTVM7BaQCs7Ubsi4KdnqkPBTQ8QxEyyXciAc"

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

    /// A `manualMintingRules` block in the real rs-dpp wire shape.
    private func tokenDict(manualMintingRule: [String: Any]) -> [String: Any] {
        return [
            "baseSupply": 0,
            "manualMintingRules": manualMintingRule
        ]
    }

    /// The five `$type`-tagged wire values paired with the canonical
    /// string the parser is expected to persist for each.
    private func wireVariants() -> [(name: String, wire: [String: Any], expected: String)] {
        return [
            (
                "noOne",
                ["$type": AuthorizedActionTakers.WireType.noOne],
                AuthorizedActionTakers.noOne.rawValue
            ),
            (
                "contractOwner",
                ["$type": AuthorizedActionTakers.WireType.contractOwner],
                AuthorizedActionTakers.contractOwner.rawValue
            ),
            (
                "mainGroup",
                ["$type": AuthorizedActionTakers.WireType.mainGroup],
                AuthorizedActionTakers.mainGroup.rawValue
            ),
            (
                "identity",
                [
                    "$type": AuthorizedActionTakers.WireType.identity,
                    "identity": identityBase58
                ],
                "Identity:\(identityBase58)"
            ),
            (
                "group",
                [
                    "$type": AuthorizedActionTakers.WireType.group,
                    "position": 3
                ],
                "Group:3"
            )
        ]
    }

    // MARK: - 1. Every variant, authorizedToMakeChange

    /// Each `$type` variant maps onto its canonical persisted string.
    func testAuthorizedToMakeChangeParsesEveryWireVariant() throws {
        for variant in wireVariants() {
            let context = try makeContext()

            let rule: [String: Any] = [
                "$formatVersion": "0",
                "authorizedToMakeChange": variant.wire,
                "adminActionTakers": ["$type": AuthorizedActionTakers.WireType.noOne]
            ]

            let token = try parseSingleToken(
                tokenDict: tokenDict(manualMintingRule: rule),
                in: context
            )

            let parsed = try XCTUnwrap(token.manualMintingRules)
            XCTAssertEqual(
                parsed.authorizedToMakeChange,
                variant.expected,
                "wire $type \(variant.name) should persist as \(variant.expected)"
            )
        }
    }

    // MARK: - 2. Every variant, adminActionTakers

    /// The admin field goes through the same helper, so it maps the same
    /// way. Its default is also "NoOne", so pin `authorizedToMakeChange`
    /// to `contractOwner` here to keep the two fields distinguishable.
    func testAdminActionTakersParsesEveryWireVariant() throws {
        for variant in wireVariants() {
            let context = try makeContext()

            let rule: [String: Any] = [
                "$formatVersion": "0",
                "authorizedToMakeChange": [
                    "$type": AuthorizedActionTakers.WireType.contractOwner
                ],
                "adminActionTakers": variant.wire
            ]

            let token = try parseSingleToken(
                tokenDict: tokenDict(manualMintingRule: rule),
                in: context
            )

            let parsed = try XCTUnwrap(token.manualMintingRules)
            XCTAssertEqual(
                parsed.authorizedToMakeChange,
                AuthorizedActionTakers.contractOwner.rawValue
            )
            XCTAssertEqual(
                parsed.adminActionTakers,
                variant.expected,
                "wire $type \(variant.name) should persist as \(variant.expected)"
            )
        }
    }

    // MARK: - 3. Unknown discriminator

    /// A variant introduced by a newer protocol version is stored
    /// verbatim in both fields, so the value stays visible instead of
    /// silently collapsing to "NoOne".
    func testUnknownWireTypeIsStoredVerbatimInBothFields() throws {
        let context = try makeContext()

        let futureType = "someFutureVariant"
        let rule: [String: Any] = [
            "$formatVersion": "0",
            "authorizedToMakeChange": ["$type": futureType],
            "adminActionTakers": ["$type": futureType]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(manualMintingRule: rule),
            in: context
        )

        let parsed = try XCTUnwrap(token.manualMintingRules)
        XCTAssertEqual(parsed.authorizedToMakeChange, futureType)
        XCTAssertEqual(parsed.adminActionTakers, futureType)
    }

    // MARK: - 4. Malformed payloads

    /// An `identity` tag with no `identity` key keeps the discriminator
    /// rather than naming an identity the parser does not have.
    func testIdentityWithoutIdKeepsDiscriminator() throws {
        let context = try makeContext()

        let rule: [String: Any] = [
            "$formatVersion": "0",
            "authorizedToMakeChange": [
                "$type": AuthorizedActionTakers.WireType.identity
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(manualMintingRule: rule),
            in: context
        )

        let parsed = try XCTUnwrap(token.manualMintingRules)
        XCTAssertEqual(
            parsed.authorizedToMakeChange,
            AuthorizedActionTakers.WireType.identity
        )
    }

    /// A `group` tag with no `position` keeps the discriminator too.
    func testGroupWithoutPositionKeepsDiscriminator() throws {
        let context = try makeContext()

        let rule: [String: Any] = [
            "$formatVersion": "0",
            "authorizedToMakeChange": [
                "$type": AuthorizedActionTakers.WireType.group
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(manualMintingRule: rule),
            in: context
        )

        let parsed = try XCTUnwrap(token.manualMintingRules)
        XCTAssertEqual(
            parsed.authorizedToMakeChange,
            AuthorizedActionTakers.WireType.group
        )
    }

    // MARK: - 5. Boolean flags alongside the maps

    /// The three boolean flags still parse when the action-taker fields
    /// are maps rather than strings.
    func testBooleanFlagsParseAlongsideActionTakerMaps() throws {
        let context = try makeContext()

        let rule: [String: Any] = [
            "$formatVersion": "0",
            "authorizedToMakeChange": [
                "$type": AuthorizedActionTakers.WireType.group,
                "position": 7
            ],
            "adminActionTakers": [
                "$type": AuthorizedActionTakers.WireType.mainGroup
            ],
            "changingAuthorizedActionTakersToNoOneAllowed": true,
            "changingAdminActionTakersToNoOneAllowed": false,
            "selfChangingAdminActionTakersAllowed": true
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(manualMintingRule: rule),
            in: context
        )

        let parsed = try XCTUnwrap(token.manualMintingRules)
        XCTAssertEqual(parsed.authorizedToMakeChange, "Group:7")
        XCTAssertEqual(parsed.adminActionTakers, AuthorizedActionTakers.mainGroup.rawValue)
        XCTAssertTrue(parsed.changingAuthorizedActionTakersToNoOneAllowed)
        XCTAssertFalse(parsed.changingAdminActionTakersToNoOneAllowed)
        XCTAssertTrue(parsed.selfChangingAdminActionTakersAllowed)
    }

    // MARK: - 6. Nested rule locations

    /// The same helper runs for rules nested under `distributionRules`
    /// and `marketplaceRules`, not only the top-level token rules.
    func testNestedRuleLocationsUseTheSameParsing() throws {
        let context = try makeContext()

        let dict: [String: Any] = [
            "baseSupply": 0,
            "distributionRules": [
                "perpetualDistributionRules": [
                    "$formatVersion": "0",
                    "authorizedToMakeChange": [
                        "$type": AuthorizedActionTakers.WireType.identity,
                        "identity": identityBase58
                    ],
                    "adminActionTakers": [
                        "$type": AuthorizedActionTakers.WireType.contractOwner
                    ]
                ]
            ],
            "marketplaceRules": [
                "tradeModeChangeRules": [
                    "$formatVersion": "0",
                    "authorizedToMakeChange": [
                        "$type": AuthorizedActionTakers.WireType.group,
                        "position": 12
                    ]
                ]
            ]
        ]

        let token = try parseSingleToken(tokenDict: dict, in: context)

        let perpetual = try XCTUnwrap(token.distributionChangeRules?.perpetualDistributionRules)
        XCTAssertEqual(perpetual.authorizedToMakeChange, "Identity:\(identityBase58)")
        XCTAssertEqual(
            perpetual.adminActionTakers,
            AuthorizedActionTakers.contractOwner.rawValue
        )

        let tradeMode = try XCTUnwrap(token.tradeModeChangeRules)
        XCTAssertEqual(tradeMode.authorizedToMakeChange, "Group:12")
    }

    // MARK: - 7. Missing field keeps the default

    /// A rule with no `authorizedToMakeChange` key keeps the
    /// most-restrictive default, exactly as before this change.
    func testMissingAuthorizedToMakeChangeKeepsMostRestrictiveDefault() throws {
        let context = try makeContext()

        let rule: [String: Any] = [
            "$formatVersion": "0",
            "adminActionTakers": [
                "$type": AuthorizedActionTakers.WireType.contractOwner
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(manualMintingRule: rule),
            in: context
        )

        let parsed = try XCTUnwrap(token.manualMintingRules)
        XCTAssertEqual(
            parsed.authorizedToMakeChange,
            AuthorizedActionTakers.noOne.rawValue
        )
        XCTAssertEqual(
            parsed.adminActionTakers,
            AuthorizedActionTakers.contractOwner.rawValue
        )
    }
}
