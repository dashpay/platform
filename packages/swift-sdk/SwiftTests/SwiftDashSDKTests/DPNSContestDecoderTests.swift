import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the `dash_sdk_contested_resource_get_vote_state` →
/// `DPNSContestVoteState` boundary.
///
/// `SDK.decodeVoteState(json:normalizedLabel:)` is pure and `nonisolated`, so
/// every branch is reachable without an SDK handle or a network. The payload
/// shapes below mirror what `rs-sdk-ffi`'s
/// `contested_resource/queries/vote_state.rs` actually emits.
final class DPNSContestDecoderTests: XCTestCase {

  // MARK: - Helpers

  private func decode(_ json: [String: Any], label: String = "a11ce") -> DPNSContestVoteState {
    SDK.decodeVoteState(json: json, normalizedLabel: label)
  }

  private func contender(_ id: String, _ votes: Int, label: String? = nil) -> [String: Any] {
    var entry: [String: Any] = ["identity_id": id, "vote_count": votes, "document": NSNull()]
    if let label { entry["label"] = label }
    return entry
  }

  // MARK: - Outcome branches

  func testOngoingWhenWinnerInfoAbsent() {
    // While voting is open, drive-abci attaches no winner info at all.
    let state = decode([
      "abstain_vote_tally": 2,
      "lock_vote_tally": 1,
      "contenders": [contender("Ab1", 5), contender("Cd2", 3)],
    ])

    XCTAssertEqual(state.outcome, .ongoing)
    XCTAssertFalse(state.isResolved)
    XCTAssertNil(state.resolvedAt)
  }

  func testNoWinnerIsResolved() {
    // `Contenders.winner` is `Some` only for a *finished* poll, so `NoWinner`
    // means "settled without awarding the name" — not "still running". A
    // regression here would let `dpnsContestIsOpen` wave a vote into a poll
    // that can no longer accept it.
    let state = decode([
      "abstain_vote_tally": 0,
      "lock_vote_tally": 0,
      "winner_info": "NoWinner",
      "block_info": ["height": 1200, "core_height": 900, "timestamp": 1_700_000_000_000],
      "contenders": [contender("Ab1", 4)],
    ])

    XCTAssertEqual(state.outcome, .noWinner)
    XCTAssertTrue(state.isResolved)
    XCTAssertEqual(state.resolvedAt, Date(timeIntervalSince1970: 1_700_000_000))
  }

  func testLockedIsResolved() {
    let state = decode([
      "abstain_vote_tally": 1,
      "lock_vote_tally": 9,
      "winner_info": "Locked",
      "block_info": ["height": 10, "core_height": 5, "timestamp": 1_600_000_000_000],
      "contenders": [contender("Ab1", 2)],
    ])

    XCTAssertEqual(state.outcome, .locked)
    XCTAssertTrue(state.isResolved)
    XCTAssertEqual(state.resolvedAt, Date(timeIntervalSince1970: 1_600_000_000))
  }

  func testWonByIdentityCarriesTheWinner() {
    let state = decode([
      "abstain_vote_tally": 0,
      "lock_vote_tally": 0,
      "winner_info": ["type": "WonByIdentity", "identity_id": "WinnerId123"],
      "block_info": ["height": 42, "core_height": 21, "timestamp": 1_650_000_000_000],
      "contenders": [contender("WinnerId123", 12), contender("LoserId456", 3)],
    ])

    XCTAssertEqual(state.outcome, .wonBy("WinnerId123"))
    XCTAssertTrue(state.isResolved)
    XCTAssertEqual(state.resolvedAt, Date(timeIntervalSince1970: 1_650_000_000))
  }

  func testUnknownWinnerStringFallsBackToOngoing() {
    // An unrecognized discriminant must not be reported as a settled
    // outcome — treating it as ongoing keeps the pre-flight conservative in
    // the read direction and lets Platform reject the vote authoritatively.
    let state = decode([
      "winner_info": "SomethingNewFromAFutureRelease",
      "contenders": [contender("Ab1", 1)],
    ])

    XCTAssertEqual(state.outcome, .ongoing)
    XCTAssertFalse(state.isResolved)
  }

  func testMalformedWonByIdentityFallsBackToOngoing() {
    // `type` present but `identity_id` missing: no winner can be named, so
    // the decoder must not invent one.
    let state = decode([
      "winner_info": ["type": "WonByIdentity"],
      "contenders": [contender("Ab1", 1)],
    ])

    XCTAssertEqual(state.outcome, .ongoing)
  }

  // MARK: - resolvedAt

  func testResolvedAtNilWhenBlockInfoAbsent() {
    let state = decode([
      "winner_info": "Locked",
      "contenders": [contender("Ab1", 1)],
    ])

    XCTAssertTrue(state.isResolved)
    XCTAssertNil(state.resolvedAt, "a resolved outcome with no block info has no known time")
  }

  func testResolvedAtNilWhenTimestampIsZero() {
    let state = decode([
      "winner_info": "Locked",
      "block_info": ["height": 1, "core_height": 1, "timestamp": 0],
      "contenders": [contender("Ab1", 1)],
    ])

    XCTAssertNil(state.resolvedAt, "epoch 0 is a missing timestamp, not 1970")
  }

  func testResolvedAtIgnoredForOngoingContest() {
    let state = decode([
      "block_info": ["height": 1, "core_height": 1, "timestamp": 1_700_000_000_000],
      "contenders": [contender("Ab1", 1)],
    ])

    XCTAssertEqual(state.outcome, .ongoing)
    XCTAssertNil(state.resolvedAt)
  }

  // MARK: - Missing fields and tally conversion

  func testEmptyPayloadDecodesToAnEmptyOngoingContest() {
    let state = decode([:])

    XCTAssertEqual(state.normalizedLabel, "a11ce")
    XCTAssertTrue(state.contenders.isEmpty)
    XCTAssertEqual(state.abstainVotes, 0)
    XCTAssertEqual(state.lockVotes, 0)
    XCTAssertEqual(state.outcome, .ongoing)
  }

  func testMissingTalliesDefaultToZero() {
    let state = decode(["contenders": [contender("Ab1", 7)]])

    XCTAssertEqual(state.abstainVotes, 0)
    XCTAssertEqual(state.lockVotes, 0)
    XCTAssertEqual(state.contenders.first?.voteTally, 7)
  }

  func testContenderWithoutIdentityIdIsDropped() {
    // Skipped rather than materialized with a placeholder id — a contender we
    // cannot identify must not become a votable target.
    let state = decode([
      "contenders": [contender("Ab1", 2), ["vote_count": 5]],
    ])

    XCTAssertEqual(state.contenders.count, 1)
    XCTAssertEqual(state.contenders.first?.identityId, "Ab1")
  }

  func testContenderWithoutVoteCountDefaultsToZero() {
    let state = decode(["contenders": [["identity_id": "Ab1"]]])

    XCTAssertEqual(state.contenders.first?.voteTally, 0)
  }

  func testLargeTallyConvertsWithoutTruncation() {
    let state = decode([
      "abstain_vote_tally": 4_294_967_295,
      "lock_vote_tally": 4_000_000_000,
      "contenders": [contender("Ab1", 3_000_000_000)],
    ])

    XCTAssertEqual(state.abstainVotes, UInt32.max)
    XCTAssertEqual(state.lockVotes, 4_000_000_000)
    XCTAssertEqual(state.contenders.first?.voteTally, 3_000_000_000)
  }

  func testContenderOrderIsPreserved() {
    let state = decode([
      "contenders": [contender("Cd2", 1), contender("Ab1", 9), contender("Ef3", 5)],
    ])

    XCTAssertEqual(state.contenders.map(\.identityId), ["Cd2", "Ab1", "Ef3"])
  }

  // MARK: - Derived helpers

  func testTotalVotesSumsContendersAbstainAndLock() {
    let contest = DPNSContest(
      normalizedLabel: "a11ce",
      endTime: nil,
      hasWinner: false,
      abstainVotes: 2,
      lockVotes: 3,
      contenders: [
        DPNSContender(identityId: "Ab1", voteTally: 5),
        DPNSContender(identityId: "Cd2", voteTally: 4),
      ])

    XCTAssertEqual(contest.totalVotes, 14)
    XCTAssertEqual(contest.leadingContender?.identityId, "Ab1")
  }

  func testLeadingContenderIsNilWithoutContenders() {
    let contest = DPNSContest(
      normalizedLabel: "a11ce",
      endTime: nil,
      hasWinner: false,
      abstainVotes: 0,
      lockVotes: 0,
      contenders: [])

    XCTAssertNil(contest.leadingContender)
    XCTAssertEqual(contest.totalVotes, 0)
  }

  // MARK: - Display labels

  func testContenderDisplayLabelIsDecoded() {
    // The FFI decodes each contender's `domain` document and hands back the
    // spelling they typed; the contest itself is keyed by the normalized form.
    let state = decode([
      "contenders": [contender("Ab1", 3, label: "pizza"), contender("Cd2", 1, label: "p1zza")],
    ])

    XCTAssertEqual(state.contenders.map(\.displayLabel), ["pizza", "p1zza"])
  }

  func testMissingLabelIsNilNotEmpty() {
    // A contender whose document could not be decoded has no display name —
    // callers fall back to the identity id rather than showing a blank.
    let state = decode(["contenders": [contender("Ab1", 3)]])

    XCTAssertNil(state.contenders.first?.displayLabel)
  }

  func testEmptyLabelIsTreatedAsAbsent() {
    let state = decode(["contenders": [contender("Ab1", 3, label: "")]])

    XCTAssertNil(state.contenders.first?.displayLabel)
  }

  func testRequestedLabelsDedupeAndPreserveOrder() {
    let contest = DPNSContest(
      normalizedLabel: "p1zza",
      endTime: nil,
      hasWinner: false,
      abstainVotes: 0,
      lockVotes: 0,
      contenders: [
        DPNSContender(identityId: "Ab1", voteTally: 3, displayLabel: "pizza"),
        DPNSContender(identityId: "Cd2", voteTally: 1, displayLabel: "p1zza"),
        DPNSContender(identityId: "Ef3", voteTally: 1, displayLabel: "pizza"),
      ])

    XCTAssertEqual(contest.requestedLabels, ["pizza", "p1zza"])
  }

  func testRequestedLabelsEmptyWhenNothingDecoded() {
    let contest = DPNSContest(
      normalizedLabel: "p1zza",
      endTime: nil,
      hasWinner: false,
      abstainVotes: 0,
      lockVotes: 0,
      contenders: [DPNSContender(identityId: "Ab1", voteTally: 3)])

    XCTAssertTrue(contest.requestedLabels.isEmpty,
                  "callers must fall back to the normalized label, not guess one")
  }

  // MARK: - Vote poll coordinates

  func testIndexValuesPutTheParentDomainFirst() {
    XCTAssertEqual(
      DPNSVotePoll.indexValues(normalizedLabel: "a11ce"),
      ["dash", "a11ce"])
  }

  func testIndexValuesDoNotNormalizeTheLabel() {
    // Normalization is the caller's job (and Rust's) — this helper must not
    // silently re-encode, or read and write paths could address different
    // polls.
    XCTAssertEqual(
      DPNSVotePoll.indexValues(normalizedLabel: "alice"),
      ["dash", "alice"])
  }
}
