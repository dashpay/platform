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
/// as the same exact decimal string, and a value the carrier cannot hold
/// must read as "no distribution".
///
/// Which amounts a contract may actually declare (1 to `i64::MAX`) is
/// rs-dpp's rule, enforced at registration, so these tests do not assert it:
/// the parser reads what is on the wire and does not keep a second copy of a
/// protocol constant.
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

    /// 18446744073709551615 == `UInt64.max`, the largest amount the wire
    /// type holds. Far above 2^53 - 1, so it only ever arrives as a decimal
    /// string.
    private let uInt64MaxAmount = "18446744073709551615"

    /// One past `UInt64.max`: no longer a `u64`, so nothing can carry it.
    private let aboveUInt64MaxAmount = "18446744073709551616"

    /// One past `Int64.max`. Still a `u64`, so the parser takes it even
    /// though rs-dpp would not have let a contract declare it.
    private let aboveInt64MaxAmount = "9223372036854775808"

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
    /// it back through `TokenOncePerIdentityDistributionCache`.
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
    /// The carrier is a `u64`, so `UInt64.max` itself reads back verbatim.
    func testAmountAboveInt64MaxPreservedVerbatim() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": uInt64MaxAmount
            ]),
            in: context
        )
        XCTAssertEqual(
            try XCTUnwrap(token.oncePerIdentityDistribution).amount,
            uInt64MaxAmount
        )
        XCTAssertNil(
            Int64(uInt64MaxAmount),
            "fixture must exceed Int64.max for this test to mean anything"
        )

        let numericContext = try makeContext()
        let asNumber = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": Int64.max
            ]),
            in: numericContext
        )
        XCTAssertEqual(
            try XCTUnwrap(asNumber.oncePerIdentityDistribution).amount,
            String(Int64.max)
        )
    }

    /// An amount the `u64` carrier cannot hold reads as "no once-per-identity
    /// distribution" rather than as a claimable one, because there is no
    /// exact value to report.
    func testAmountAboveTheCarrierTypeIsRejected() throws {
        let context = try makeContext()

        let token = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: [
                "$formatVersion": "0",
                "amount": aboveUInt64MaxAmount
            ]),
            in: context
        )

        XCTAssertNil(token.oncePerIdentityDistribution)
        XCTAssertFalse(token.hasDistribution)
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
                "amount": uInt64MaxAmount
            ])?.amount,
            uInt64MaxAmount
        )
    }

    /// An `amount` the `u64` carrier cannot hold is refused, so iOS never
    /// reports a distribution whose amount it could not state exactly. The
    /// lenient `stringifyDistributionAmount` used by the pre-programmed
    /// schedule would have accepted most of these.
    func testParseOncePerIdentityDistributionRejectsAmountsTheCarrierCannotHold() throws {
        let rejected: [(String, Any)] = [
            ("negative number", -1),
            ("negative string", "-1"),
            ("fractional number", 1.5),
            ("fractional string", "1.5"),
            ("non-numeric string", "abc"),
            ("empty string", ""),
            ("one past UInt64.max", aboveUInt64MaxAmount),
            ("boolean, which bridges to NSNumber and would read as 0 or 1", true)
        ]
        for (description, amount) in rejected {
            XCTAssertNil(
                DataContractParser.parseOncePerIdentityDistribution(["amount": amount]),
                "should reject \(description)"
            )
        }

        // Everything the carrier can hold is read back, including values
        // rs-dpp's own rule would not let a contract declare: that rule is
        // enforced at registration, not mirrored here.
        let accepted: [(Any, String)] = [
            (0, "0"),
            (1, "1"),
            (aboveInt64MaxAmount, aboveInt64MaxAmount),
            (uInt64MaxAmount, uInt64MaxAmount),
            // A non-canonical spelling normalises rather than being handed
            // back verbatim.
            ("0005", "5")
        ]
        for (amount, expected) in accepted {
            XCTAssertEqual(
                DataContractParser.parseOncePerIdentityDistribution(["amount": amount])?.amount,
                expected,
                "should accept \(amount)"
            )
        }
    }

    // MARK: - 4. The decode is paid once per contract payload

    /// Reading the derived property decodes the contract JSON on the first
    /// read and answers from the memo afterwards. Without this the common
    /// token, which declares no distribution at all, would re-decode a whole
    /// contract per row per paint: `hasDistribution` falls through to this
    /// property for the badge in `TokenSearchView` and again in its filter,
    /// and the claim form and permission resolver read it too.
    func testRepeatedReadsDecodeTheContractOnce() throws {
        let cache = TokenOncePerIdentityDistributionCache.shared
        cache.removeAll()

        let context = try makeContext()
        let tokens = try parseTokens(
            [
                "0": tokenDict(oncePerIdentity: ["amount": 100]),
                "1": tokenDict(oncePerIdentity: nil)
            ],
            in: context
        )
        XCTAssertEqual(
            cache.decodeCount,
            0,
            "the contract parser writes the rows without deriving anything"
        )

        let withDistribution = try XCTUnwrap(tokens.first { $0.position == 0 })
        let withoutDistribution = try XCTUnwrap(tokens.first { $0.position == 1 })

        XCTAssertEqual(withDistribution.oncePerIdentityDistribution?.amount, "100")
        XCTAssertEqual(cache.decodeCount, 1, "first read decodes the payload")

        for _ in 0..<5 {
            XCTAssertEqual(withDistribution.oncePerIdentityDistribution?.amount, "100")
            XCTAssertNil(withoutDistribution.oncePerIdentityDistribution)
            XCTAssertTrue(withDistribution.hasDistribution)
            XCTAssertFalse(withoutDistribution.hasDistribution)
        }
        XCTAssertEqual(
            cache.decodeCount,
            1,
            "later reads, including the sibling position that has none, come from the memo"
        )
    }

    /// Two contracts are two payloads: the memo is keyed per contract, not
    /// shared across them.
    func testSeparateContractsDecodeSeparately() throws {
        let cache = TokenOncePerIdentityDistributionCache.shared
        cache.removeAll()

        let context = try makeContext()
        let first = try parseSingleToken(
            tokenDict: tokenDict(oncePerIdentity: ["amount": 11]),
            in: context
        )

        let otherId = Data(repeating: 0xAB, count: 32)
        let otherContractData: [String: Any] = [
            "tokens": ["0": tokenDict(oncePerIdentity: ["amount": 22])]
        ]
        let otherContract = PersistentDataContract(
            id: otherId,
            name: "Other",
            serializedContract: try JSONSerialization.data(
                withJSONObject: otherContractData,
                options: []
            ),
            network: .testnet
        )
        context.insert(otherContract)
        try context.save()
        try DataContractParser.parseDataContract(
            contractData: otherContractData,
            contractId: otherId,
            modelContext: context
        )
        let otherDescriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.contractId == otherId }
        )
        let second = try XCTUnwrap(try context.fetch(otherDescriptor).first)

        XCTAssertEqual(first.oncePerIdentityDistribution?.amount, "11")
        XCTAssertEqual(second.oncePerIdentityDistribution?.amount, "22")
        XCTAssertEqual(cache.decodeCount, 2)

        XCTAssertEqual(first.oncePerIdentityDistribution?.amount, "11")
        XCTAssertEqual(second.oncePerIdentityDistribution?.amount, "22")
        XCTAssertEqual(cache.decodeCount, 2, "both payloads stay memoised")
    }
}
