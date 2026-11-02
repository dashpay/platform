import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the decision proposal JSON the `rs-sdk-ffi`
/// `dash_sdk_decision_proposal_*` reads return, decoded by
/// `DecisionProposalJSON` exactly as the `SDK` wrappers decode it, and for the
/// `vote_choice` byte `dash_sdk_decision_proposal_cast_vote` takes.
///
/// The payloads mirror the shapes the FFI's
/// `packages/rs-sdk-ffi/src/decision_proposal/json.rs` writes (and its tests
/// pin). No SDK handle, no network.
final class DecisionProposalDecodingTests: XCTestCase {
    private let createdAt: UInt64 = 1_700_000_000_000
    private let endDate: UInt64 = 1_700_000_000_000 + 14 * 86_400_000

    private func decode<T: Decodable>(_ type: T.Type, _ json: String) throws -> T {
        try DecisionProposalJSON.decode(type, from: json)
    }

    private func proposalJSON(url: String) -> String {
        """
        {"id":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",\
        "ownerId":"CQgw5i7y2Y4Ns6rTgV6cq7MhQ7b4oJxEf1vsaFgM2KXJ",\
        "revision":1,"createdAt":\(createdAt),"updatedAt":\(createdAt),\
        "title":"Raise the block size?","description":"Yes doubles it; no keeps it.",\
        \(url)"passPercent":60,"minimumTurnoutPercent":10,"endDate":\(endDate)}
        """
    }

    private let fractionPollJSON = """
        {"$type":"yesNoVotePoll",\
        "purpose":{"$type":"decisionProposal","documentId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"},\
        "supermajorityNumerator":60,"supermajorityDenominator":100,\
        "minimumVotingPower":{"$type":"fractionOfTotal","numerator":10,"denominator":100,"rounding":"up"}}
        """

    // MARK: - Proposal

    func testShouldDecodeAProposalWithEveryField() throws {
        let proposal = try decode(
            DecisionProposal.self,
            proposalJSON(url: #""url":"https://forum.dash.org/t/block-size","#)
        )

        XCTAssertEqual(proposal.id, "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF")
        XCTAssertEqual(proposal.ownerId, "CQgw5i7y2Y4Ns6rTgV6cq7MhQ7b4oJxEf1vsaFgM2KXJ")
        XCTAssertEqual(proposal.revision, 1)
        XCTAssertEqual(proposal.createdAtMs, createdAt)
        XCTAssertEqual(proposal.updatedAtMs, createdAt)
        XCTAssertEqual(proposal.title, "Raise the block size?")
        XCTAssertEqual(proposal.description, "Yes doubles it; no keeps it.")
        XCTAssertEqual(proposal.url, "https://forum.dash.org/t/block-size")
        XCTAssertEqual(proposal.passPercent, 60)
        XCTAssertEqual(proposal.minimumTurnoutPercent, 10)
        XCTAssertEqual(proposal.endDateMs, endDate)
        XCTAssertEqual(proposal.endDate, Date(timeIntervalSince1970: TimeInterval(endDate) / 1000))
        XCTAssertEqual(proposal.createdAt, Date(timeIntervalSince1970: 1_700_000_000))
    }

    /// The FFI writes a proposal without a url as `"url": null`; a missing key
    /// reads the same.
    func testShouldDecodeAProposalWithoutAUrl() throws {
        let nullURL = try decode(DecisionProposal.self, proposalJSON(url: #""url":null,"#))
        XCTAssertNil(nullURL.url)

        let absentURL = try decode(DecisionProposal.self, proposalJSON(url: ""))
        XCTAssertNil(absentURL.url)
        XCTAssertEqual(absentURL.title, "Raise the block size?")
    }

    func testShouldDecodeNullSystemFieldsAsAbsent() throws {
        let json = """
            {"id":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",\
            "ownerId":"CQgw5i7y2Y4Ns6rTgV6cq7MhQ7b4oJxEf1vsaFgM2KXJ",\
            "revision":null,"createdAt":null,"updatedAt":null,\
            "title":"t","description":"d","url":null,\
            "passPercent":50,"minimumTurnoutPercent":0,"endDate":\(endDate)}
            """
        let proposal = try decode(DecisionProposal.self, json)

        XCTAssertNil(proposal.revision)
        XCTAssertNil(proposal.createdAtMs)
        XCTAssertNil(proposal.createdAt)
        XCTAssertNil(proposal.updatedAtMs)
    }

    func testShouldDecodeAListingInItsOrder() throws {
        let second = proposalJSON(url: "")
            .replacingOccurrences(
                of: "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
                with: "11111111111111111111111111111111"
            )
        let listing = try decode(
            [DecisionProposal].self,
            "[\(proposalJSON(url: #""url":null,"#)),\(second)]"
        )

        XCTAssertEqual(
            listing.map(\.id),
            ["4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF", "11111111111111111111111111111111"]
        )
        XCTAssertEqual(try decode([DecisionProposal].self, "[]"), [])
    }

    func testShouldRefuseAProposalMissingARequiredField() {
        let json = proposalJSON(url: "").replacingOccurrences(of: #""passPercent":60,"#, with: "")

        XCTAssertThrowsError(try decode(DecisionProposal.self, json)) { error in
            guard case SDKError.serializationError = error else {
                return XCTFail("expected a serialization error, got \(error)")
            }
        }
    }

    // MARK: - Poll state

    func testShouldDecodeAStartedPollWithItsTallies() throws {
        let json = """
            {"votePoll":\(fractionPollJSON),\
            "status":"started","startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},\
            "yesVotingPower":5,"noVotingPower":1,"abstainVotingPower":4}
            """
        let state = try decode(YesNoVotePollState.self, json)

        XCTAssertEqual(
            state.votePoll.purpose,
            .decisionProposal(documentId: "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF")
        )
        XCTAssertEqual(state.votePoll.supermajorityNumerator, 60)
        XCTAssertEqual(state.votePoll.supermajorityDenominator, 100)
        XCTAssertEqual(
            state.votePoll.minimumVotingPower,
            .fractionOfTotal(numerator: 10, denominator: 100, rounding: .up)
        )
        XCTAssertEqual(
            state.status,
            .started(startBlock: PlatformBlockInfoFixture.block(height: 7))
        )
        XCTAssertNil(state.result)
        XCTAssertEqual(state.yesVotingPower, 5)
        XCTAssertEqual(state.noVotingPower, 1)
        XCTAssertEqual(state.abstainVotingPower, 4)
    }

    /// Once finished the poll has no `startBlock`, its record is in `result`,
    /// and the top-level tallies are 0 (the votes are removed).
    func testShouldDecodeAFinishedPollWithItsResult() throws {
        let json = """
            {"votePoll":\(fractionPollJSON),\
            "status":"finished",\
            "result":{"passed":true,"yesVotingPower":30,"noVotingPower":10,\
            "abstainVotingPower":8,"requiredVotingPower":20,\
            "startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},\
            "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}},\
            "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
            """
        let state = try decode(YesNoVotePollState.self, json)
        let result = try XCTUnwrap(state.result)

        XCTAssertTrue(result.passed)
        XCTAssertEqual(result.yesVotingPower, 30)
        XCTAssertEqual(result.noVotingPower, 10)
        XCTAssertEqual(result.abstainVotingPower, 8)
        XCTAssertEqual(result.requiredVotingPower, 20)
        XCTAssertEqual(result.startBlock, PlatformBlockInfoFixture.block(height: 7))
        XCTAssertEqual(result.finalizationBlock, PlatformBlockInfoFixture.block(height: 9))
        XCTAssertEqual(result.finalizationBlock.time, Date(timeIntervalSince1970: 9))
        XCTAssertEqual(state.status, .finished(result))
        XCTAssertEqual(state.yesVotingPower, 0)
    }

    func testShouldDecodeAnAbsoluteMinimumVotingPower() throws {
        let json = """
            {"votePoll":{"$type":"yesNoVotePoll",\
            "purpose":{"$type":"decisionProposal","documentId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"},\
            "supermajorityNumerator":2,"supermajorityDenominator":3,\
            "minimumVotingPower":{"$type":"absolute","votingPower":400}},\
            "status":"started","startBlock":{"height":7,"coreHeight":42,"timeMs":7000,"epoch":3},\
            "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
            """
        let state = try decode(YesNoVotePollState.self, json)

        XCTAssertEqual(state.votePoll.minimumVotingPower, .absolute(votingPower: 400))
    }

    func testShouldDecodeEveryRounding() throws {
        for (name, rounding) in [
            ("down", VotingPowerRounding.down),
            ("up", .up),
            ("downPlusOne", .downPlusOne),
            ("upMinusOne", .upMinusOne),
        ] {
            let minimum = try decode(
                YesNoMinimumVotingPower.self,
                #"{"$type":"fractionOfTotal","numerator":1,"denominator":3,"rounding":"\#(name)"}"#
            )
            XCTAssertEqual(minimum, .fractionOfTotal(numerator: 1, denominator: 3, rounding: rounding))
        }
    }

    func testShouldRefuseAnUnknownPollStatus() {
        let json = """
            {"votePoll":\(fractionPollJSON),"status":"paused",\
            "yesVotingPower":0,"noVotingPower":0,"abstainVotingPower":0}
            """

        XCTAssertThrowsError(try decode(YesNoVotePollState.self, json))
    }

    func testShouldRefuseAnUnknownMinimumVotingPowerType() {
        XCTAssertThrowsError(
            try decode(YesNoMinimumVotingPower.self, #"{"$type":"quadratic","votingPower":1}"#)
        )
    }

    // MARK: - Masternode answer and vote choice

    func testShouldDecodeAMasternodeAnswerForEveryChoice() throws {
        for choice in YesNoAbstainVoteChoice.allCases {
            let vote = try decode(
                MasternodeYesNoVote.self,
                #"{"choice":"\#(choice.rawValue)","timesVoted":2}"#
            )
            XCTAssertEqual(vote, try decode(
                MasternodeYesNoVote.self,
                #"{"timesVoted":2,"choice":"\#(choice.rawValue)"}"#
            ))
            XCTAssertEqual(vote.choice, choice)
            XCTAssertEqual(vote.timesVoted, 2)
        }
        XCTAssertThrowsError(
            try decode(MasternodeYesNoVote.self, #"{"choice":"lock","timesVoted":1}"#)
        )
    }

    /// The `vote_choice` byte must agree with `YesNoVoteChoiceFFI` in
    /// `rs-sdk-ffi`: yes 0, no 1, abstain 2. Any other byte is refused there.
    func testShouldEncodeTheVoteChoicesAsTheFFIBytes() {
        XCTAssertEqual(YesNoAbstainVoteChoice.yes.ffiTag, 0)
        XCTAssertEqual(YesNoAbstainVoteChoice.no.ffiTag, 1)
        XCTAssertEqual(YesNoAbstainVoteChoice.abstain.ffiTag, 2)
        XCTAssertEqual(YesNoAbstainVoteChoice.allCases.map(\.rawValue), ["yes", "no", "abstain"])
    }

    // MARK: - Params

    func testShouldWriteTheEndDateInMilliseconds() {
        let params = DecisionProposalParams(
            title: "t",
            description: "d",
            passPercent: 60,
            minimumTurnoutPercent: 10,
            endDate: Date(timeIntervalSince1970: 1_701_209_600.123_9)
        )

        XCTAssertEqual(params.endDateMs, 1_701_209_600_123)
        XCTAssertNil(params.url)

        let beforeEpoch = DecisionProposalParams(
            title: "t",
            description: "d",
            passPercent: 60,
            minimumTurnoutPercent: 10,
            endDate: Date(timeIntervalSince1970: -1)
        )
        XCTAssertEqual(beforeEpoch.endDateMs, 0)
    }
}

/// Blocks as the payloads above write them: core height 42, epoch 3, a second
/// of block time per height.
private enum PlatformBlockInfoFixture {
    static func block(height: UInt64) -> PlatformBlockInfo {
        // Decoded rather than built, as `PlatformBlockInfo` has no public
        // memberwise initializer.
        let json = #"{"height":\#(height),"coreHeight":42,"timeMs":\#(height * 1000),"epoch":3}"#
        return try! DecisionProposalJSON.decode(PlatformBlockInfo.self, from: json)
    }
}
