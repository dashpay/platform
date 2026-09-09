import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `DataContractParser`'s pre-programmed-distribution
/// parsing — the parse half of the iOS "pre-programmed recipient can
/// never open the Claim row" fix.
///
/// The real rs-dpp contract JSON encodes a pre-programmed distribution
/// as a `distributions` map keyed by trigger-timestamp-in-milliseconds,
/// each value a map of recipient-base58 -> amount:
///
/// ```json
/// "preProgrammedDistribution": {
///   "$formatVersion": "0",
///   "distributions": { "<timestampMs>": { "<recipientBase58>": <amount> } }
/// }
/// ```
///
/// The parser previously only understood a synthetic
/// `distributionSchedule` array shape, so on real contracts the schedule
/// came back empty and the recipient information was lost — leaving the
/// Claim-action resolver unable to match a listed recipient. These tests
/// drive the public `parseDataContract` entry point against an in-memory
/// `ModelContainer` (no network, no FFI handle) and assert the recipient,
/// amount, and trigger-time land on the persisted `PersistentToken`.
///
/// Mirrors the live testnet contract
/// `CNpuW3ZjmTrfxKm2VyEJYSBm2jFn7Va2YCmRyVP1ZPvC` / token `androidqadist`.
@MainActor
final class DataContractParserPreProgrammedTests: XCTestCase {

    private let contractId = Data(repeating: 0xCD, count: 32)

    /// A canonical base58 identity id (the recipient key shape used in
    /// the real contract's `distributions` map).
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

    /// Wrap a `preProgrammedDistribution` payload in the minimal token
    /// dict shape the parser expects (`distributionRules` wrapper).
    private func tokenDict(preProgrammed: [String: Any]) -> [String: Any] {
        return [
            "baseSupply": 0,
            "distributionRules": [
                "preProgrammedDistribution": preProgrammed
            ]
        ]
    }

    // MARK: - 1. Real rs-dpp shape

    /// The live-contract shape: one timestamp key, one recipient, a
    /// numeric amount. Assert the flattened event carries the recipient,
    /// amount, and a trigger-time derived from the ms timestamp.
    func testRealDistributionsShapeParsesRecipientAmountAndTime() throws {
        let context = try makeContext()

        let preProgrammed: [String: Any] = [
            "$formatVersion": "0",
            "distributions": [
                "1719000000000": [
                    recipientBase58: 5000
                ]
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(preProgrammed: preProgrammed),
            in: context
        )

        let schedule = try XCTUnwrap(token.preProgrammedDistribution?.distributionSchedule)
        XCTAssertEqual(schedule.count, 1)

        let event = try XCTUnwrap(schedule.first)
        XCTAssertEqual(event.recipient, recipientBase58)
        XCTAssertEqual(event.amount, "5000")
        // 1719000000000 ms -> 1719000000 s since epoch.
        XCTAssertEqual(event.triggerTime, Date(timeIntervalSince1970: 1719000000))
    }

    // MARK: - 2. String + large (> Int64.max) amounts

    /// Amounts encoded as JSON strings — including a value larger than
    /// `Int64.max` — must survive verbatim (no truncation / overflow).
    func testAmountAsStringAndBeyondInt64MaxPreservedVerbatim() throws {
        let context = try makeContext()

        // 18446744073709551615 == UInt64.max, which exceeds Int64.max.
        let hugeAmount = "18446744073709551615"
        let smallStringAmount = "12345"

        let recipientB = "Gsj5AqEnKzGZAdA4kZUZRWkQRxqXV7UwSFDdKQPGoXA6"

        let preProgrammed: [String: Any] = [
            "$formatVersion": "0",
            "distributions": [
                "1719000000000": [
                    recipientBase58: hugeAmount,
                    recipientB: smallStringAmount
                ]
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(preProgrammed: preProgrammed),
            in: context
        )

        let schedule = try XCTUnwrap(token.preProgrammedDistribution?.distributionSchedule)
        XCTAssertEqual(schedule.count, 2)

        let hugeEvent = try XCTUnwrap(schedule.first { $0.recipient == recipientBase58 })
        XCTAssertEqual(hugeEvent.amount, hugeAmount)

        let smallEvent = try XCTUnwrap(schedule.first { $0.recipient == recipientB })
        XCTAssertEqual(smallEvent.amount, smallStringAmount)
    }

    // MARK: - 3. Multiple timestamps × multiple recipients

    /// Two timestamps, each with two recipients, flatten to four events —
    /// one per (timestamp, recipient) pair — with correct per-pair data.
    func testMultipleTimestampsAndRecipientsProduceOneEventPerPair() throws {
        let context = try makeContext()

        let recipientB = "Gsj5AqEnKzGZAdA4kZUZRWkQRxqXV7UwSFDdKQPGoXA6"

        let preProgrammed: [String: Any] = [
            "$formatVersion": "0",
            "distributions": [
                "1719000000000": [
                    recipientBase58: 100,
                    recipientB: 200
                ],
                "1720000000000": [
                    recipientBase58: 300,
                    recipientB: 400
                ]
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(preProgrammed: preProgrammed),
            in: context
        )

        let schedule = try XCTUnwrap(token.preProgrammedDistribution?.distributionSchedule)
        XCTAssertEqual(schedule.count, 4, "2 timestamps × 2 recipients = 4 events")

        // Spot-check one specific (timestamp, recipient) pairing.
        let earlyTime = Date(timeIntervalSince1970: 1719000000)
        let lateTime = Date(timeIntervalSince1970: 1720000000)

        let earlyForA = schedule.first {
            $0.recipient == recipientBase58 && $0.triggerTime == earlyTime
        }
        XCTAssertEqual(earlyForA?.amount, "100")

        let lateForB = schedule.first {
            $0.recipient == recipientB && $0.triggerTime == lateTime
        }
        XCTAssertEqual(lateForB?.amount, "400")

        // Every recipient string is one of the two we supplied.
        let recipients = Set(schedule.map { $0.recipient })
        XCTAssertEqual(recipients, [recipientBase58, recipientB])
    }

    // MARK: - 4. Backward-compat: legacy `distributionSchedule` array

    /// The older/synthetic `distributionSchedule` array shape must still
    /// parse when no real `distributions` map is present.
    func testLegacyDistributionScheduleArrayStillParses() throws {
        let context = try makeContext()

        let preProgrammed: [String: Any] = [
            "distributionSchedule": [
                [
                    "amount": "7500",
                    "recipient": recipientBase58,
                    "triggerType": "Time"
                ]
            ]
        ]

        let token = try parseSingleToken(
            tokenDict: tokenDict(preProgrammed: preProgrammed),
            in: context
        )

        let schedule = try XCTUnwrap(token.preProgrammedDistribution?.distributionSchedule)
        XCTAssertEqual(schedule.count, 1)

        let event = try XCTUnwrap(schedule.first)
        XCTAssertEqual(event.recipient, recipientBase58)
        XCTAssertEqual(event.amount, "7500")
        XCTAssertEqual(event.triggerType, "Time")
    }

    // MARK: - 5. No preProgrammedDistribution key

    /// A token whose `distributionRules` carry no
    /// `preProgrammedDistribution` leaves the model's optional nil.
    func testNoPreProgrammedDistributionLeavesPropertyNil() throws {
        let context = try makeContext()

        let dict: [String: Any] = [
            "baseSupply": 0,
            "distributionRules": [
                // Only a perpetual block — no preProgrammedDistribution.
                "perpetualDistribution": [
                    "distributionType": ["Fixed": ["amount": 1]],
                    "distributionRecipient": "ContractOwner"
                ]
            ]
        ]

        let token = try parseSingleToken(tokenDict: dict, in: context)

        XCTAssertNil(
            token.preProgrammedDistribution,
            "no preProgrammedDistribution key -> property stays nil"
        )
    }
}
