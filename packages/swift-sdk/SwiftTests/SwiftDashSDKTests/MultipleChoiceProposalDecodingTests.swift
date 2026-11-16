import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the multiple choice proposal JSON the `rs-sdk-ffi`
/// `dash_sdk_multiple_choice_proposal_*` reads return, decoded by
/// `DecisionProposalJSON` exactly as the `SDK` wrappers decode it, and for the
/// `vote_choice` byte and `choice_id` `dash_sdk_multiple_choice_proposal_cast_vote`
/// takes.
///
/// The payloads mirror the shapes the FFI's
/// `packages/rs-sdk-ffi/src/decision_proposal/multiple_choice/json.rs` writes
/// (and its tests pin). No SDK handle, no network.
final class MultipleChoiceProposalDecodingTests: XCTestCase {
    private let createdAt: UInt64 = 1_700_000_000_000
    private let proposalId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    private let ownerId = "CQgw5i7y2Y4Ns6rTgV6cq7MhQ7b4oJxEf1vsaFgM2KXJ"
    private let choiceId = "8wtF3Sji4LR7gRmmSPXuQbqWf9GmgPJyDukTEBpHmh5K"
    private let otherChoiceId = "11111111111111111111111111111111"

    private func decode<T: Decodable>(_ type: T.Type, _ json: String) throws -> T {
        try DecisionProposalJSON.decode(type, from: json)
    }

    private func proposalJSON(descriptionDocument: String, url: String, open: Bool = true) -> String {
        """
        {"id":"\(proposalId)","ownerId":"\(ownerId)",\
        "revision":1,"createdAt":\(createdAt),"updatedAt":\(createdAt),\
        "title":"Which block size?",\
        "descriptionDocument":\(descriptionDocument),"url":\(url),"open":\(open)}
        """
    }

    private var descriptionDocumentJSON: String {
        #"{"id":"\#(otherChoiceId)","contractId":"\#(ownerId)","documentType":"post"}"#
    }

    private func choiceJSON(id: String, description: String) -> String {
        """
        {"id":"\(id)","ownerId":"\(ownerId)","proposalId":"\(proposalId)",\
        "revision":null,"createdAt":\(createdAt),"updatedAt":null,\
        "title":"Double it","description":\(description)}
        """
    }

    // MARK: - Proposal

    func testShouldDecodeAProposalWithEveryField() throws {
        let proposal = try decode(
            MultipleChoiceProposal.self,
            proposalJSON(
                descriptionDocument: descriptionDocumentJSON,
                url: #""https://forum.dash.org/t/block-size""#
            )
        )

        XCTAssertEqual(proposal.id, proposalId)
        XCTAssertEqual(proposal.ownerId, ownerId)
        XCTAssertEqual(proposal.revision, 1)
        XCTAssertEqual(proposal.createdAtMs, createdAt)
        XCTAssertEqual(proposal.updatedAtMs, createdAt)
        XCTAssertEqual(proposal.createdAt, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertEqual(proposal.title, "Which block size?")
        XCTAssertEqual(proposal.descriptionDocument?.id, otherChoiceId)
        XCTAssertEqual(proposal.descriptionDocument?.contractId, ownerId)
        XCTAssertEqual(proposal.descriptionDocument?.documentType, "post")
        XCTAssertEqual(proposal.url, "https://forum.dash.org/t/block-size")
        XCTAssertTrue(proposal.open)
    }

    func testShouldDecodeAnAbsentDescriptionDocumentAndUrlAsNil() throws {
        let proposal = try decode(
            MultipleChoiceProposal.self,
            proposalJSON(descriptionDocument: "null", url: "null", open: false)
        )

        XCTAssertNil(proposal.descriptionDocument)
        XCTAssertNil(proposal.url)
        XCTAssertFalse(proposal.open)
    }

    func testShouldDecodeNullSystemFieldsAsAbsent() throws {
        let json = """
            {"id":"\(proposalId)","ownerId":"\(ownerId)",\
            "revision":null,"createdAt":null,"updatedAt":null,\
            "title":"t","descriptionDocument":null,"url":"https://dash.org","open":true}
            """
        let proposal = try decode(MultipleChoiceProposal.self, json)

        XCTAssertNil(proposal.revision)
        XCTAssertNil(proposal.createdAtMs)
        XCTAssertNil(proposal.createdAt)
        XCTAssertNil(proposal.updatedAtMs)
        XCTAssertNil(proposal.updatedAt)
    }

    func testShouldDecodeAListingInItsOrder() throws {
        let first = proposalJSON(descriptionDocument: "null", url: #""https://dash.org""#)
        let second = first.replacingOccurrences(of: proposalId, with: otherChoiceId)
        let listing = try decode([MultipleChoiceProposal].self, "[\(first),\(second)]")

        XCTAssertEqual(listing.map(\.id), [proposalId, otherChoiceId])
        XCTAssertEqual(try decode([MultipleChoiceProposal].self, "[]"), [])
    }

    func testShouldRefuseAProposalMissingARequiredField() {
        let json = proposalJSON(descriptionDocument: "null", url: "null")
            .replacingOccurrences(of: #","open":true"#, with: "")

        XCTAssertThrowsError(try decode(MultipleChoiceProposal.self, json)) { error in
            guard case SDKError.serializationError = error else {
                return XCTFail("expected a serialization error, got \(error)")
            }
        }
    }

    // MARK: - Choice

    func testShouldDecodeAChoiceWithEveryField() throws {
        let choice = try decode(
            ProposalChoice.self,
            choiceJSON(id: choiceId, description: #""Twice the size it is today.""#)
        )

        XCTAssertEqual(choice.id, choiceId)
        XCTAssertEqual(choice.ownerId, ownerId)
        XCTAssertEqual(choice.proposalId, proposalId)
        XCTAssertNil(choice.revision)
        XCTAssertEqual(choice.createdAtMs, createdAt)
        XCTAssertEqual(choice.createdAt, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertNil(choice.updatedAtMs)
        XCTAssertEqual(choice.title, "Double it")
        XCTAssertEqual(choice.description, "Twice the size it is today.")
    }

    func testShouldDecodeAChoiceWithoutADescription() throws {
        let choice = try decode(ProposalChoice.self, choiceJSON(id: choiceId, description: "null"))

        XCTAssertNil(choice.description)
        XCTAssertEqual(choice.title, "Double it")
    }

    // MARK: - Vote

    func testShouldDecodeARunningVoteWithItsChoicesAndTallies() throws {
        let json = """
            {"choices":[\
            {"id":"\(otherChoiceId)","choice":null,"voteTally":null},\
            {"id":"\(choiceId)","choice":\(choiceJSON(id: choiceId, description: "null")),"voteTally":12}],\
            "abstainVoteTally":3,"noneOfTheChoicesVoteTally":1,"result":null}
            """
        let vote = try decode(MultipleChoiceProposalVote.self, json)

        XCTAssertEqual(vote.choices.map(\.id), [otherChoiceId, choiceId])
        XCTAssertNil(vote.choices[0].choice)
        XCTAssertNil(vote.choices[0].voteTally)
        XCTAssertEqual(vote.choices[1].choice?.id, choiceId)
        XCTAssertEqual(vote.choices[1].choice?.title, "Double it")
        XCTAssertEqual(vote.choices[1].voteTally, 12)
        XCTAssertEqual(vote.abstainVoteTally, 3)
        XCTAssertEqual(vote.noneOfTheChoicesVoteTally, 1)
        XCTAssertNil(vote.result)
    }

    func testShouldDecodeHowAFinishedVoteEnded() throws {
        for (outcomeName, outcome, keptChoiceId) in [
            ("keptChoice", MultipleChoiceProposalVote.Outcome.keptChoice, #""\#(choiceId)""#),
            ("noneOfTheChoices", .noneOfTheChoices, "null"),
            ("noWinner", .noWinner, "null"),
        ] {
            let json = """
                {"choices":[{"id":"\(choiceId)","choice":null,"voteTally":9}],\
                "abstainVoteTally":null,"noneOfTheChoicesVoteTally":4,\
                "result":{"outcome":"\(outcomeName)","keptChoiceId":\(keptChoiceId),\
                "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}}}
                """
            let vote = try decode(MultipleChoiceProposalVote.self, json)
            let result = try XCTUnwrap(vote.result, outcomeName)

            XCTAssertEqual(result.outcome, outcome)
            XCTAssertEqual(result.keptChoiceId, outcome == .keptChoice ? choiceId : nil)
            XCTAssertEqual(result.finalizationBlock.height, 9)
            XCTAssertEqual(result.finalizationBlock.coreHeight, 42)
            XCTAssertEqual(result.finalizationBlock.timeMs, 9000)
            XCTAssertEqual(result.finalizationBlock.epoch, 3)
            XCTAssertEqual(result.finalizationBlock.time, Date(timeIntervalSince1970: 9))
            XCTAssertEqual(vote.choices.first?.voteTally, 9)
            XCTAssertNil(vote.choices.first?.choice)
            XCTAssertNil(vote.abstainVoteTally)
            XCTAssertEqual(vote.noneOfTheChoicesVoteTally, 4)
        }
    }

    func testShouldDecodeAVoteWithNoChoiceAsEmpty() throws {
        let vote = try decode(
            MultipleChoiceProposalVote.self,
            #"{"choices":[],"abstainVoteTally":null,"noneOfTheChoicesVoteTally":null,"result":null}"#
        )

        XCTAssertEqual(vote.choices, [])
        XCTAssertNil(vote.abstainVoteTally)
        XCTAssertNil(vote.noneOfTheChoicesVoteTally)
        XCTAssertNil(vote.result)
    }

    func testShouldRefuseAnUnknownOutcome() {
        let json = """
            {"choices":[],"abstainVoteTally":null,"noneOfTheChoicesVoteTally":null,\
            "result":{"outcome":"locked","keptChoiceId":null,\
            "finalizationBlock":{"height":9,"coreHeight":42,"timeMs":9000,"epoch":3}}}
            """

        XCTAssertThrowsError(try decode(MultipleChoiceProposalVote.self, json))
    }

    // MARK: - Masternode answer and vote choice

    func testShouldDecodeAMasternodeAnswerForEveryChoice() throws {
        XCTAssertEqual(
            try decode(MultipleChoiceVoteChoice.self, #"{"choice":"choice","choiceId":"\#(choiceId)"}"#),
            .choice(id: choiceId)
        )
        XCTAssertEqual(
            try decode(MultipleChoiceVoteChoice.self, #"{"choice":"noneOfTheChoices","choiceId":null}"#),
            .noneOfTheChoices
        )
        XCTAssertEqual(
            try decode(MultipleChoiceVoteChoice.self, #"{"choice":"abstain","choiceId":null}"#),
            .abstain
        )
    }

    func testShouldRefuseAMasternodeAnswerThatCannotBeRead() {
        // A vote for a choice that names none, and an unknown answer.
        XCTAssertThrowsError(
            try decode(MultipleChoiceVoteChoice.self, #"{"choice":"choice","choiceId":null}"#)
        )
        XCTAssertThrowsError(
            try decode(MultipleChoiceVoteChoice.self, #"{"choice":"lock","choiceId":null}"#)
        )
    }

    /// The `vote_choice` byte must agree with `MultipleChoiceVoteChoiceFFI` in
    /// `rs-sdk-ffi`: a choice 0, none of the choices 1, abstain 2. The FFI
    /// requires `choice_id` for 0 and a null one otherwise.
    func testShouldEncodeTheVoteChoicesAsTheFFIBytesAndChoiceId() {
        XCTAssertEqual(MultipleChoiceVoteChoice.choice(id: choiceId).ffiTag, 0)
        XCTAssertEqual(MultipleChoiceVoteChoice.noneOfTheChoices.ffiTag, 1)
        XCTAssertEqual(MultipleChoiceVoteChoice.abstain.ffiTag, 2)

        XCTAssertEqual(MultipleChoiceVoteChoice.choice(id: choiceId).choiceId, choiceId)
        XCTAssertNil(MultipleChoiceVoteChoice.noneOfTheChoices.choiceId)
        XCTAssertNil(MultipleChoiceVoteChoice.abstain.choiceId)
    }

    // MARK: - Credits

    func testShouldReadTheFeeAndFundAsDecimalCredits() throws {
        XCTAssertEqual(try DecisionProposalJSON.credits(from: "20000000000"), 20_000_000_000)
        XCTAssertEqual(try DecisionProposalJSON.credits(from: "18446744073709551615"), UInt64.max)
        for notCredits in ["", "-1", "1.5", "\"20000000000\"", "18446744073709551616"] {
            XCTAssertThrowsError(try DecisionProposalJSON.credits(from: notCredits), notCredits) { error in
                guard case SDKError.serializationError = error else {
                    return XCTFail("expected a serialization error, got \(error)")
                }
            }
        }
    }
}
