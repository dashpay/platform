import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the contract moderation and fee pot JSON the
/// `rs-sdk-ffi` `dash_sdk_contract_moderation_*` and
/// `dash_sdk_contract_get_fee_pots` reads return, decoded by
/// `ContractModerationJSON` exactly as the `SDK` wrappers decode it, and for
/// the list and status bytes those functions take.
///
/// The payloads are the ones the FFI's
/// `packages/rs-sdk-ffi/src/contract_moderation/json.rs` tests pin: the ids
/// are the base58 of 32 bytes of one value (`[3; 32]` and so on). No SDK
/// handle, no network.
final class ContractModerationDecodingTests: XCTestCase {
    private let id1 = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
    private let id2 = "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
    private let id3 = "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8"
    private let id4 = "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq"
    private let id5 = "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY"
    private let id6 = "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF"
    private let id9 = "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN"
    private let id20 = "2MNus2KCpxwXnp19iyXNpWSFtBD2UGjQBAL8AbtywfT9"
    private let id21 = "2RJD1KnDRGEkvuFfAGrJ7PD28LRE9LRDjZznDywagzmr"
    private let id22 = "2VDW9dFE1ZXz4zWAbaBDQFynNVdRpQ73HyfSHMzBSL6Z"
    private let id30 = "32ZsJ2yJjwuoBiWE5xnZjG9tKmK3CubbmEzgkQLyQzgD"
    private let id31 = "36VASLSKLFD2KokjXG7V28veZvXEsyHRKefLonPaAKzv"

    private func decode<T: Decodable>(_ type: T.Type, _ json: String) throws -> T {
        try ContractModerationJSON.decode(type, from: json)
    }

    private func reason(_ text: String) -> ContractModerationReason {
        ContractModerationReason(code: nil, text: text, documents: nil, reasonDocumentId: nil)
    }

    private var fullReasonJSON: String {
        """
        {"code":7,"text":"spam",\
        "documents":[{"documentTypeName":"post","documentId":"\(id5)"}],\
        "reasonDocumentId":"\(id6)"}
        """
    }

    // MARK: - Arguments

    /// The bytes must agree with `ContractModerationListFFI` and
    /// `ContractTeamActionStatusFFI` in `rs-sdk-ffi`; any other byte is
    /// refused there.
    func testShouldEncodeListsAndStatusesAsTheFFIBytes() {
        XCTAssertEqual(ContractModerationList.banlist.ffiTag, 0)
        XCTAssertEqual(ContractModerationList.suspensions.ffiTag, 1)
        XCTAssertEqual(ContractModerationList.warnings.ffiTag, 2)
        XCTAssertEqual(ContractTeamActionStatus.active.ffiTag, 0)
        XCTAssertEqual(ContractTeamActionStatus.closed.ffiTag, 1)
        XCTAssertEqual(
            ContractModerationList.allCases.map(\.rawValue),
            ["banlist", "suspensions", "warnings"]
        )
    }

    func testShouldRefuseAnIdWithANulCharacterBeforeAnyCall() {
        XCTAssertThrowsError(
            try withContractModerationCStrings([id3, "a\u{0}b"], field: "documentIds") { _, _ in 0 }
        ) { error in
            guard case SDKError.invalidParameter = error else {
                return XCTFail("expected an invalid parameter, got \(error)")
            }
        }
        let count = try? withContractModerationCStrings([id3, id4], field: "documentIds") { pointer, count in
            XCTAssertEqual(pointer.flatMap { $0[1] }.map { String(cString: $0) }, self.id4)
            return count
        }
        XCTAssertEqual(count, 2)
    }

    // MARK: - Reasons and status

    func testShouldDecodeAReasonWithANullCodeOrEveryPart() throws {
        XCTAssertEqual(
            try decode(ContractModerationReason.self, #"{"code":null,"text":"spam"}"#),
            reason("spam")
        )
        let full = try decode(ContractModerationReason.self, fullReasonJSON)
        XCTAssertEqual(full.code, 7)
        XCTAssertEqual(full.documents, [ContractModerationDocument(documentTypeName: "post", documentId: id5)])
        XCTAssertEqual(full.reasonDocumentId, id6)
    }

    func testShouldDecodeAStatusOnEveryList() throws {
        let json = """
            {"lists":["banlist","suspensions","warnings"],\
            "banned":true,"banReason":\(fullReasonJSON),\
            "suspendedUntil":1700000000000,"suspensionReason":{"code":null,"text":"cool off"},\
            "warnings":[{"warnedAt":1600000000000,"reason":{"code":null,"text":"first"}}]}
            """
        let status = try decode(ContractModerationStatus.self, json)

        XCTAssertEqual(status.lists, [.banlist, .suspensions, .warnings])
        XCTAssertEqual(status.banned, true)
        XCTAssertEqual(status.banReason?.text, "spam")
        XCTAssertEqual(status.suspendedUntilMs, 1_700_000_000_000)
        XCTAssertEqual(status.suspendedUntil, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertEqual(status.suspensionReason, reason("cool off"))
        XCTAssertEqual(status.warnings, [ContractWarning(warnedAtMs: 1_600_000_000_000, reason: reason("first"))])
    }

    /// A list that was not read leaves its fields absent: unknown, not clear.
    func testShouldDecodeAStatusOnSomeListsWithTheOthersAbsent() throws {
        let banlistOnly = try decode(ContractModerationStatus.self, #"{"lists":["banlist"],"banned":false}"#)
        XCTAssertEqual(banlistOnly.lists, [.banlist])
        XCTAssertEqual(banlistOnly.banned, false)
        XCTAssertNil(banlistOnly.suspendedUntilMs)
        XCTAssertNil(banlistOnly.warnings)

        let clear = try decode(
            ContractModerationStatus.self, #"{"lists":["suspensions","warnings"],"warnings":[]}"#)
        XCTAssertNil(clear.banned)
        XCTAssertNil(clear.suspendedUntilMs)
        XCTAssertEqual(clear.warnings, [])
    }

    func testShouldRefuseAnUnknownList() {
        XCTAssertThrowsError(try decode(ContractModerationStatus.self, #"{"lists":["mutes"]}"#))
    }

    // MARK: - Entries

    func testShouldDecodeAnEntriesPageWithItsCursor() throws {
        let json = """
            {"entries":[\
            {"identityId":"\(id3)","until":1700000000000,"reason":{"code":null,"text":"cool off"}},\
            {"identityId":"\(id4)","until":1700000000000,"reason":{"code":null,"text":"cool off"}}],\
            "nextStartAfter":"\(id4)"}
            """
        let page = try decode(ContractModerationEntriesPage.self, json)

        XCTAssertEqual(page.entries.map(\.identityId), [id3, id4])
        XCTAssertEqual(page.entries[0].untilMs, 1_700_000_000_000)
        XCTAssertNil(page.entries[0].warnings)
        XCTAssertEqual(page.nextStartAfter, id4)

        let last = try decode(ContractModerationEntriesPage.self, #"{"entries":[]}"#)
        XCTAssertEqual(last.entries, [])
        XCTAssertNil(last.nextStartAfter)
    }

    func testShouldDecodeAWarningEntryWithItsWarnings() throws {
        let json = """
            {"entries":[{"identityId":"\(id3)","reason":{"code":null,"text":"second"},\
            "warnings":[{"warnedAt":1,"reason":{"code":null,"text":"first"}},\
            {"warnedAt":2,"reason":{"code":null,"text":"second"}}]}]}
            """
        let entry = try XCTUnwrap(try decode(ContractModerationEntriesPage.self, json).entries.first)

        XCTAssertNil(entry.untilMs)
        XCTAssertEqual(entry.reason, reason("second"))
        XCTAssertEqual(entry.warnings?.map(\.warnedAtMs), [1, 2])
    }

    // MARK: - Removals

    func testShouldDecodeRemovalsWithKeptFieldsRestorationAndCursor() throws {
        let hash = String(repeating: "ab", count: 32)
        let json = """
            {"removals":[\
            {"documentId":"\(id3)","documentOwnerId":"\(id20)","moderatorId":"\(id21)",\
            "reason":{"code":null,"text":"off topic"},"removedAt":1700000000000,\
            "documentHash":"\(hash)",\
            "keptFields":{"$createdAt":1600000000000,"author":"\(id9)","hashtag":"dash",\
            "meta":{"tags":["privacy"]},"score":-3}},\
            {"documentId":"\(id4)","documentOwnerId":"\(id20)","moderatorId":"\(id21)",\
            "reason":{"code":null,"text":"off topic"},"removedAt":1700000000000,\
            "documentHash":"\(hash)","restoredBy":"\(id22)","restoredAt":1700000100000,\
            "keptFields":{}}],\
            "nextStartAfter":"\(id4)"}
            """
        let page = try ContractModerationJSON.decodeRemovalsPage(from: json)
        let removed = page.removals[0]
        let restored = page.removals[1]

        XCTAssertEqual(removed.documentId, id3)
        XCTAssertEqual(removed.documentOwnerId, id20)
        XCTAssertEqual(removed.moderatorId, id21)
        XCTAssertEqual(removed.removedAtMs, 1_700_000_000_000)
        XCTAssertEqual(removed.documentHash, hash)
        XCTAssertNil(removed.restoredBy)
        XCTAssertNil(removed.restoredAt)
        XCTAssertEqual(removed.keptFields, [
            "$createdAt": .int(1_600_000_000_000),
            "author": .string(id9),
            "hashtag": .string("dash"),
            "meta": .object(["tags": .array([.string("privacy")])]),
            "score": .int(-3),
        ])
        XCTAssertEqual(restored.restoredBy, id22)
        XCTAssertEqual(restored.restoredAtMs, 1_700_000_100_000)
        XCTAssertEqual(restored.keptFields, [:])
        XCTAssertEqual(page.nextStartAfter, id4)
    }

    func testShouldDecodeKeptValuesBeyondTheSignedRange() throws {
        let values = try decode(
            [Swift.String: ContractKeptFieldValue].self,
            #"{"big":18446744073709551615,"flag":true,"none":null,"ratio":0.5}"#
        )
        XCTAssertEqual(values["big"], .uint(UInt64.max))
        XCTAssertEqual(values["flag"], .bool(true))
        XCTAssertEqual(values["none"], .null)
        XCTAssertEqual(values["ratio"], .double(0.5))
    }

    /// `JSONDecoder` reads `1.0` and `-0.0` as integers; the SDK reads kept
    /// values with `JSONSerialization`, which keeps their kind and sign.
    func testShouldKeepTheNumericKindOfKeptValues() throws {
        let hash = String(repeating: "ab", count: 32)
        let json = """
            {"removals":[{"documentId":"\(id3)","documentOwnerId":"\(id20)","moderatorId":"\(id21)",\
            "reason":{"code":null,"text":"x"},"removedAt":1,"documentHash":"\(hash)",\
            "keptFields":{"whole":1.0,"negativeZero":-0.0,"int":1,"big":18446744073709551615,\
            "min":-9223372036854775808,"flag":false,"nested":{"ratio":1.5,"list":[1e3,2]}}}]}
            """
        let kept = try XCTUnwrap(ContractModerationJSON.decodeRemovalsPage(from: json).removals.first).keptFields

        XCTAssertEqual(kept["whole"], .double(1.0))
        guard case .double(let negativeZero) = kept["negativeZero"] else {
            return XCTFail("expected a double, got \(String(describing: kept["negativeZero"]))")
        }
        XCTAssertEqual(negativeZero, 0)
        XCTAssertEqual(negativeZero.sign, .minus)
        XCTAssertEqual(kept["int"], .int(1))
        XCTAssertEqual(kept["big"], .uint(UInt64.max))
        XCTAssertEqual(kept["min"], .int(Int64.min))
        XCTAssertEqual(kept["flag"], .bool(false))
        XCTAssertEqual(kept["nested"], .object(["ratio": .double(1.5), "list": .array([.double(1000), .int(2)])]))
    }

    /// The catalog re-encodes a page with `JSONEncoder`, which keeps the sign
    /// of a negative zero.
    func testShouldEncodeANegativeZeroWithItsSign() throws {
        let data = try JSONEncoder().encode(ContractKeptFieldValue.double(-0.0))
        XCTAssertEqual(String(decoding: data, as: UTF8.self), "-0")
    }

    // MARK: - Team actions

    func testShouldDecodeATeamActionsPageWithItsCursor() throws {
        let json = """
            {"actions":[\
            {"actionId":"\(id3)","proposerId":"\(id30)","proposedAt":1000,\
            "event":{"type":"deleteSettledDocument","documentTypeName":"post","documentId":"\(id31)",\
            "documentLastModifiedAt":10,"documentRevision":4,"reason":{"code":null,"text":"spam"}},\
            "approvalCount":2},\
            {"actionId":"\(id4)","proposerId":"\(id30)","proposedAt":1000,\
            "event":{"type":"deleteSettledDocument","documentTypeName":"post","documentId":"\(id31)",\
            "documentLastModifiedAt":10,"reason":{"code":null,"text":"spam"}},\
            "approvalCount":2}],\
            "nextStartAtActionId":"\(id4)","nextStartAtActionIdIncluded":false}
            """
        let page = try decode(ContractTeamActionsPage.self, json)

        XCTAssertEqual(page.actions.map(\.actionId), [id3, id4])
        XCTAssertEqual(page.actions[0].proposerId, id30)
        XCTAssertEqual(page.actions[0].proposedAtMs, 1_000)
        XCTAssertEqual(page.actions[0].approvalCount, 2)
        XCTAssertEqual(
            page.actions[0].event,
            .deleteSettledDocument(ContractTeamActionDeleteSettledDocument(
                documentTypeName: "post",
                documentId: id31,
                documentLastModifiedAtMs: 10,
                documentRevision: 4,
                reason: reason("spam")
            ))
        )
        guard case .deleteSettledDocument(let second) = page.actions[1].event else {
            return XCTFail("expected a settled document deletion")
        }
        XCTAssertNil(second.documentRevision)
        XCTAssertEqual(page.nextStartAtActionId, id4)
        XCTAssertEqual(page.nextStartAtActionIdIncluded, false)
    }

    func testShouldRefuseAnUnknownTeamActionEvent() {
        XCTAssertThrowsError(
            try decode(ContractTeamActionEvent.self, #"{"type":"mint","documentId":"x"}"#)
        )
    }

    /// Re-encoding writes the `type` tag beside the event's fields, as the
    /// FFI does, so the catalog shows the same JSON.
    func testShouldEncodeATeamActionEventWithItsTag() throws {
        let event = ContractTeamActionEvent.deleteSettledDocument(ContractTeamActionDeleteSettledDocument(
            documentTypeName: "post",
            documentId: id31,
            documentLastModifiedAtMs: 10,
            documentRevision: nil,
            reason: reason("spam")
        ))
        let data = try JSONEncoder().encode(event)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])

        XCTAssertEqual(object["type"] as? String, "deleteSettledDocument")
        XCTAssertEqual(object["documentLastModifiedAt"] as? Int, 10)
        XCTAssertEqual(try decode(ContractTeamActionEvent.self, String(decoding: data, as: UTF8.self)), event)
    }

    // MARK: - Signers, counts and fee pots

    func testShouldDecodeSignersAndActionCounts() throws {
        XCTAssertEqual(
            try decode(ContractTeamActionSigners.self, #"{"signerIds":["\#(id1)","\#(id2)"]}"#).signerIds,
            [id1, id2]
        )
        XCTAssertEqual(try decode(ContractTeamActionSigners.self, #"{"signerIds":[]}"#).signerIds, [])

        let counts = try decode(
            ContractModerationActionCounts.self,
            #"{"counts":[{"identityId":"\#(id1)","count":3},{"identityId":"\#(id2)","count":5}]}"#
        )
        XCTAssertEqual(counts.counts, [
            ContractModerationActionCount(identityId: id1, count: 3),
            ContractModerationActionCount(identityId: id2, count: 5),
        ])
    }

    /// A pot never paid out has no last claim; one paid out in epoch 0 has
    /// one. Credits use the whole u64 range.
    func testShouldDecodeFeePotsWithALastClaimOnlyWhenPaidOut() throws {
        let json = """
            {"owner":{"credits":10000000},\
            "moderators":{"credits":18446744073709551615,"lastClaimEpoch":0,\
            "lastClaimTimeMs":1700000000000,"lastClaimantId":"\(id9)"}}
            """
        let pots = try decode(ContractFeePots.self, json)

        XCTAssertEqual(pots.owner.credits, 10_000_000)
        XCTAssertNil(pots.owner.lastClaimEpoch)
        XCTAssertNil(pots.owner.lastClaimTime)
        XCTAssertEqual(pots.moderators.credits, UInt64.max)
        XCTAssertEqual(pots.moderators.lastClaimEpoch, 0)
        XCTAssertEqual(pots.moderators.lastClaimTimeMs, 1_700_000_000_000)
        XCTAssertEqual(pots.moderators.lastClaimantId, id9)
    }
}
