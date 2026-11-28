import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the contract fee claim JSON the `rs-platform-wallet-ffi`
/// `platform_wallet_contract_fee_claim` and
/// `platform_wallet_contract_fee_claim_eligibility` functions write, decoded
/// exactly as the `ManagedPlatformWallet` wrappers decode it, and for the pot
/// byte the claim takes.
///
/// The payloads are the ones the FFI's
/// `packages/rs-platform-wallet-ffi/src/contract_fee_claim.rs` tests pin. No
/// wallet handle, no network.
final class ContractFeeClaimDecodingTests: XCTestCase {
    private let id1 = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi"
    private let id2 = "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR"
    private let id3 = "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8"
    private let id4 = "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq"
    private let id5 = "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY"

    /// The bytes must agree with `CONTRACT_FEE_POT_OWNER` and
    /// `CONTRACT_FEE_POT_MODERATORS` in the FFI; any other byte is refused
    /// there.
    func testShouldEncodeThePotsAsTheFFIBytes() {
        XCTAssertEqual(ContractFeePot.owner.ffiTag, 0)
        XCTAssertEqual(ContractFeePot.moderators.ffiTag, 1)
        XCTAssertEqual(ContractFeePot.allCases.map(\.rawValue), ["owner", "moderators"])
    }

    func testShouldDecodeAClaimWithThePotItLeftAndTheBalancesItPaid() throws {
        let json = """
        {"contractId":"\(id1)","pot":"moderators",\
        "potAfterClaim":{"credits":2,"lastClaimEpoch":12,"lastClaimTimeMs":1700000000000,\
        "lastClaimantId":"\(id3)"},\
        "balances":[{"identityId":"\(id3)","balance":1500},\
        {"identityId":"\(id4)","balance":18446744073709551615}]}
        """
        let claimed = try ClaimedContractFees.decode(fromJSON: json)
        XCTAssertEqual(claimed.contractId, id1)
        XCTAssertEqual(claimed.pot, .moderators)
        XCTAssertEqual(claimed.potAfterClaim.credits, 2)
        XCTAssertEqual(claimed.potAfterClaim.lastClaimEpoch, 12)
        XCTAssertEqual(claimed.potAfterClaim.lastClaimTimeMs, 1_700_000_000_000)
        XCTAssertEqual(claimed.potAfterClaim.lastClaimantId, id3)
        XCTAssertEqual(
            claimed.balances,
            [
                ContractFeeClaimBalance(identityId: id3, balance: 1_500),
                ContractFeeClaimBalance(identityId: id4, balance: UInt64.max),
            ]
        )
    }

    /// The pot a claim leaves has the shape of one pot of the fee pot read,
    /// so it decodes as the read's own type.
    func testShouldDecodeThePotAfterAClaimAsTheFeePotReadDecodesAPot() throws {
        let pot = """
        {"credits":2,"lastClaimEpoch":12,"lastClaimTimeMs":1700000000000,"lastClaimantId":"\(id3)"}
        """
        let claimed = try ClaimedContractFees.decode(fromJSON: """
        {"contractId":"\(id1)","pot":"owner","potAfterClaim":\(pot),"balances":[]}
        """)
        let read = try ContractModerationJSON.decode(ContractFeePots.self, from: """
        {"owner":\(pot),"moderators":{"credits":0}}
        """)
        XCTAssertEqual(claimed.potAfterClaim, read.owner)
        XCTAssertEqual(claimed.pot, .owner)
    }

    func testShouldDecodeWhoMayClaimEachPot() throws {
        let json = """
        {"contractId":"\(id1)","ownerPotClaimants":["\(id2)"],\
        "moderatorsPotClaimants":["\(id2)","\(id5)"],"moderatorsPotTeam":"electedSeated"}
        """
        let eligibility = try ContractFeeClaimEligibility.decode(fromJSON: json)
        XCTAssertEqual(eligibility.contractId, id1)
        XCTAssertEqual(eligibility.claimants(of: .owner), [id2])
        XCTAssertEqual(eligibility.claimants(of: .moderators), [id2, id5])
        XCTAssertEqual(eligibility.moderatorsPotTeam, .electedSeated)

        for (name, team) in [
            ("noModeration", ContractModeratorsPotTeam.noModeration),
            ("declared", .declared),
            ("electedInterim", .electedInterim),
            ("electedSeated", .electedSeated),
        ] {
            let decoded = try ContractFeeClaimEligibility.decode(fromJSON: """
            {"contractId":"\(id1)","ownerPotClaimants":[],"moderatorsPotClaimants":[],\
            "moderatorsPotTeam":"\(name)"}
            """)
            XCTAssertEqual(decoded.moderatorsPotTeam, team)
        }
    }

    func testShouldRefuseJSONOfAnotherShapeAsADeserializationError() {
        XCTAssertThrowsError(try ClaimedContractFees.decode(fromJSON: "{\"pot\":\"treasury\"}")) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected a deserialization error, got \(error)")
            }
        }
        XCTAssertThrowsError(try ContractFeeClaimEligibility.decode(fromJSON: "[]")) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected a deserialization error, got \(error)")
            }
        }
    }

    /// The claim and every signed moderation decode their output through
    /// `readingPaidOutput` once Rust reported the write as done: an output
    /// that does not decode reads as "may have gone through", never as a
    /// refusal before any payment.
    func testShouldClassAnUnreadableOutputOfAPaidWriteAsMaybeSent() {
        let reads: [(String, () throws -> Any)] = [
            ("fee claim", { try ClaimedContractFees.decode(fromJSON: "{\"pot\":\"treasury\"}") }),
            ("user moderation", { try ModeratedContractUser.decode(fromJSON: "[]") }),
            ("document deletion", { try ContractDocumentDeletion.decode(fromJSON: "[]") }),
            ("document restore", { try ContractDocumentRestore.decode(fromJSON: "[]") }),
            ("team action", { try ContractTeamActionSignature.decode(fromJSON: "[]") }),
            ("field change", { try ContractDocumentFieldsChange.decode(fromJSON: "[]") }),
        ]
        for (what, read) in reads {
            XCTAssertThrowsError(try PlatformWalletError.readingPaidOutput(read), what) { error in
                guard case PlatformWalletError.deserialization(_, let payment) = error else {
                    return XCTFail("\(what): expected a deserialization error, got \(error)")
                }
                XCTAssertEqual(payment, .maybeSent, what)
            }
        }
    }
}
