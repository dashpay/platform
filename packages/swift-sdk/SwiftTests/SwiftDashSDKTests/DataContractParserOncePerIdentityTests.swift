import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for the once-per-identity token distribution on iOS: a fixed
/// amount every identity may claim exactly once (protocol version 14).
///
/// rs-dpp serialises it inside a token's `distributionRules`, next to the
/// perpetual and pre-programmed blocks:
///
/// ```json
/// "distributionRules": {
///   "oncePerIdentityDistribution": { "$formatVersion": "0", "amount": 5000 }
/// }
/// ```
///
/// `amount` is a protocol `u64`, so it arrives as a JSON number up to
/// 2^53 - 1 and as a decimal string above that. Both must land on the model
/// as the same exact decimal string.
///
/// Unlike the perpetual and pre-programmed kinds this one has no column on
/// `PersistentToken`: `DashSchemaV5` is frozen, and a new stored property
/// would move the model's entity hash (see `DashModelContainer.modelTypes`
/// and `DashModelMigrationTests`). `PersistentToken.oncePerIdentityDistribution`
/// therefore derives the value from the contract JSON stored on the owning
/// `PersistentDataContract`, which is why these tests seed
/// `serializedContract` rather than leaving it empty like the sibling parser
/// suites do.
@MainActor
final class DataContractParserOncePerIdentityTests: XCTestCase {

    private let contractId = Data(repeating: 0xEF, count: 32)

    /// 18446744073709551615 == `UInt64.max`, well above `Int64.max`. A value
    /// this large only ever arrives as a JSON string.
    private let hugeAmount = "18446744073709551615"

    private func makeContext() throws -> ModelContext {
        let container = try DashModelContainer.createInMemory()
        return ModelContext(container)
    }

    /// Seed the `PersistentDataContract` row the parser needs (tokens hang off
    /// the contract relationship, and `parseTokens` bails out without it),
    /// then run the parser and hand back every persisted token.
    ///
    /// The contract row carries the JSON-serialised `contractData` on
    /// `serializedContract`, which is what both of `ContractDownloader`'s
    /// persist paths do before calling the parser. The derived property reads
    /// it back through `PersistentDataContract.parsedContract`.
    @discardableResult
    private func parseTokens(
        _ tokens: [String: Any],
        in context: ModelContext
    ) throws -> [PersistentToken] {
        let contractData: [String: Any] = ["tokens": tokens]
        let serialized = try JSONSerialization.data(
            withJSONObject: contractData,
            options: []
        )

        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: serialized,
            network: .testnet
        )
        context.insert(contract)
        try context.save()

        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractId,
            modelContext: context
        )

        let id = contractId
        let descriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.contractId == id }
        )
        return try context.fetch(descriptor)
    }

    private func parseSingleToken(
        tokenDict: [String: Any],
        in context: ModelContext
    ) throws -> PersistentToken {
        let tokens = try parseTokens(["0": tokenDict], in: context)
        XCTAssertEqual(tokens.count, 1, "fixture declares exactly one token")
        return try XCTUnwrap(tokens.first, "parser should have persisted one token")
    }

    /// Wrap a `oncePerIdentityDistribution` payload in the minimal token dict
    /// the parser expects. Passing nil leaves `distributionRules` present but
    /// empty, which is the "token declares no distribution at all" case.
    private func tokenDict(oncePerIdentity: [String: Any]?) -> [String: Any] {
        var rules: [String: Any] = [:]
        if let oncePerIdentity {
            rules["oncePerIdentityDistribution"] = oncePerIdentity
        }
        return [
            "baseSupply": 0,
            "distributionRules": rules
        ]
    }

    // MARK: - 1. Amount encodings

    /// The common shape: `amount` as a JSON number.
    func testNumericAmountParsesToDecimalString() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": 5000
            ]),
            in: context
        )

        let distribution = try XCTUnwrap(token.oncePerIdentityDistribution)
        XCTAssertEqual(distribution.amount, "5000")
    }

    /// `amount` as a JSON string, the encoding rs-dpp uses for large values,
    /// lands unchanged.
    func testStringAmountParsesVerbatim() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": "12345"
            ]),
            in: context
        )

        let distribution = try XCTUnwrap(token.oncePerIdentityDistribution)
        XCTAssertEqual(distribution.amount, "12345")
    }

    /// An amount above `Int64.max` survives with every digit intact: no
    /// truncation, no overflow, no round trip through a floating-point type.
    func testAmountAboveInt64MaxPreservedVerbatim() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": hugeAmount
            ]),
            in: context
        )

        let distribution = try XCTUnwrap(token.oncePerIdentityDistribution)
        XCTAssertEqual(distribution.amount, hugeAmount)
        XCTAssertNil(
            Int64(distribution.amount),
            "fixture must exceed Int64.max for this test to mean anything"
        )
    }

    // MARK: - 2. Presence drives `hasDistribution`

    /// A token whose only distribution is once-per-identity still reports
    /// `hasDistribution`, so the search filter and the details section find it
    /// even though no column is set.
    func testOncePerIdentityAloneMakesHasDistributionTrue() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": 1
            ]),
            in: context
        )

        XCTAssertNil(token.perpetualDistribution)
        XCTAssertNil(token.preProgrammedDistribution)
        XCTAssertNotNil(token.oncePerIdentityDistribution)
        XCTAssertTrue(token.hasDistribution)
    }

    /// No `oncePerIdentityDistribution` key: the property stays nil, and with
    /// no other distribution the token reports none.
    func testNoOncePerIdentityKeyLeavesPropertyNilAndNoDistribution() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: nil),
            in: context
        )

        XCTAssertNil(token.oncePerIdentityDistribution)
        XCTAssertNil(token.perpetualDistribution)
        XCTAssertNil(token.preProgrammedDistribution)
        XCTAssertFalse(token.hasDistribution)
    }

    /// A malformed block, present but carrying no `amount`, reads as "no
    /// once-per-identity distribution" rather than as an amount of zero.
    func testBlockWithoutAmountYieldsNil() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: ["$formatVersion": "0"]),
            in: context
        )

        XCTAssertNil(token.oncePerIdentityDistribution)
        XCTAssertFalse(token.hasDistribution)
    }

    /// The kind coexists with the column-backed ones rather than replacing
    /// them, and each token derives the block at its own position key.
    func testEachTokenPositionDerivesItsOwnAmount() throws {
        let context = try makeContext()

        let tokens = try parseTokens(
            [
                "0": tokenDict(oncePerIdentity: ["amount": 100]),
                "1": tokenDict(oncePerIdentity: ["amount": 200]),
                "2": tokenDict(oncePerIdentity: nil)
            ],
            in: context
        )
        XCTAssertEqual(tokens.count, 3)

        let first = try XCTUnwrap(tokens.first { $0.position == 0 })
        let second = try XCTUnwrap(tokens.first { $0.position == 1 })
        let third = try XCTUnwrap(tokens.first { $0.position == 2 })

        XCTAssertEqual(first.oncePerIdentityDistribution?.amount, "100")
        XCTAssertEqual(second.oncePerIdentityDistribution?.amount, "200")
        XCTAssertNil(third.oncePerIdentityDistribution)
    }

    // MARK: - 3. The shared parse function

    /// The derived property and the contract parser share one function, so
    /// pin its behaviour directly rather than only through a persisted token.
    func testParseOncePerIdentityDistributionAcceptsAndRejectsInputShapes() throws {
        XCTAssertNil(
            DataContractParser.parseOncePerIdentityDistribution(nil),
            "absent block"
        )
        XCTAssertNil(
            DataContractParser.parseOncePerIdentityDistribution("oncePerIdentity"),
            "block that is not a dictionary"
        )
        let emptyBlock: [String: Any] = [:]
        XCTAssertNil(
            DataContractParser.parseOncePerIdentityDistribution(emptyBlock),
            "dictionary without an amount"
        )
        XCTAssertNil(
            DataContractParser.parseOncePerIdentityDistribution(["amount": ["nested": 1]]),
            "amount that is neither a number nor a string"
        )

        XCTAssertEqual(
            DataContractParser.parseOncePerIdentityDistribution(["amount": 7])?.amount,
            "7"
        )
        XCTAssertEqual(
            DataContractParser.parseOncePerIdentityDistribution([
                "$formatVersion": "0",
                "amount": hugeAmount
            ])?.amount,
            hugeAmount
        )
    }
}
