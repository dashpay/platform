import XCTest

@testable import SwiftDashSDK

/// The voting key rules are rs-sdk's and tested there. These only show that
/// the Swift wrappers carry a few cases across the FFI as Rust returns them.
final class MasternodeVotingKeyRulesTests: XCTestCase {
    private let invalid = MasternodeVotingKeyCheck.invalid(message: "No voting identity exists.")
    private let undetermined = MasternodeVotingKeyCheck.undetermined(message: "timeout.")

    func testShouldRefuseAnInvalidKeyAndRecordItOnlyOnTheSameStoredKey() {
        let onSameKey = invalid.outcome(slot: .sameKey(recorded: .valid))
        XCTAssertFalse(onSameKey.storeKey)
        XCTAssertEqual(onSameKey.record, invalid, "the record is the check, with its message")

        let onDifferentKey = invalid.outcome(slot: .differentKey)
        XCTAssertFalse(onDifferentKey.storeKey)
        XCTAssertNil(onDifferentKey.record)
    }

    func testShouldStoreAndRecordAValidKey() {
        for slot: MasternodeVotingKeySlot in [.empty, .differentKey, .sameKey(recorded: invalid)] {
            XCTAssertEqual(
                MasternodeVotingKeyCheck.valid.outcome(slot: slot),
                MasternodeVotingKeyCheckOutcome(storeKey: true, record: .valid),
                "\(slot)"
            )
        }
    }

    func testShouldKeepAProvenRecordOfTheSameKeyWhenTheCheckCouldNotRun() {
        let kept = undetermined.outcome(slot: .sameKey(recorded: .valid))
        XCTAssertTrue(kept.storeKey)
        XCTAssertNil(kept.record, "nil keeps what is recorded")

        let recorded = undetermined.outcome(slot: .empty)
        XCTAssertTrue(recorded.storeKey)
        XCTAssertEqual(recorded.record, undetermined)
    }

    func testShouldOfferEveryKeyButARefusedOneAndCheckOnlyUnprovenOnes() {
        for (recorded, canVote, checkBeforeUse): (MasternodeVotingKeyCheck?, Bool, Bool) in [
            (nil, true, true),
            (undetermined, true, true),
            (.valid, true, false),
            (invalid, false, false),
        ] {
            let use = MasternodeRecordedVotingKeyUse(recorded: recorded)
            XCTAssertEqual(use.canVote, canVote, "\(String(describing: recorded))")
            XCTAssertEqual(use.checkBeforeUse, checkBeforeUse, "\(String(describing: recorded))")
        }
    }
}
