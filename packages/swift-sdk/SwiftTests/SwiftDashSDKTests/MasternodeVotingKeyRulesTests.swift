import XCTest

@testable import SwiftDashSDK

/// The voting key rules are rs-sdk's and tested there. These only show that
/// the Swift wrappers carry a few cases across the FFI as Rust returns them.
final class MasternodeVotingKeyRulesTests: XCTestCase {
    private let invalid = MasternodeVotingKeyCheck.invalid(message: "No voting identity exists.")
    private let undetermined = MasternodeVotingKeyCheck.undetermined(message: "timeout.")
    private let checkedAddress = Data(repeating: 0x07, count: 20)
    private let otherAddress = Data(repeating: 0x09, count: 20)

    private func recorded(_ check: MasternodeVotingKeyCheck, _ address: Data) -> MasternodeRecordedVotingKey? {
        MasternodeRecordedVotingKey(check: check, votingAddress: address)
    }

    func testShouldRefuseAnInvalidKeyAndRecordItOnlyOnTheSameStoredKey() {
        let onSameKey = invalid.outcome(
            votingAddress: checkedAddress,
            slot: .sameKey(recorded: recorded(.valid, checkedAddress))
        )
        XCTAssertFalse(onSameKey.storeKey)
        XCTAssertEqual(onSameKey.record, invalid, "the record is the check, with its message")

        let onDifferentKey = invalid.outcome(votingAddress: checkedAddress, slot: .differentKey)
        XCTAssertFalse(onDifferentKey.storeKey)
        XCTAssertNil(onDifferentKey.record)
    }

    func testShouldStoreAndRecordAValidKey() {
        for slot: MasternodeVotingKeySlot in [
            .empty, .differentKey, .sameKey(recorded: recorded(invalid, checkedAddress)),
        ] {
            XCTAssertEqual(
                MasternodeVotingKeyCheck.valid.outcome(votingAddress: checkedAddress, slot: slot),
                MasternodeVotingKeyCheckOutcome(storeKey: true, record: .valid),
                "\(slot)"
            )
        }
    }

    func testShouldKeepAProvenRecordOnlyOfTheSameKeyWhenTheCheckCouldNotRun() {
        let kept = undetermined.outcome(
            votingAddress: checkedAddress,
            slot: .sameKey(recorded: recorded(.valid, checkedAddress))
        )
        XCTAssertTrue(kept.storeKey)
        XCTAssertNil(kept.record, "nil leaves the record unchanged")

        let ofAnotherKey = undetermined.outcome(
            votingAddress: checkedAddress,
            slot: .sameKey(recorded: recorded(.valid, otherAddress))
        )
        XCTAssertTrue(ofAnotherKey.storeKey)
        XCTAssertEqual(ofAnotherKey.record, undetermined, "a record of another key is not kept")

        let recordedInEmptySlot = undetermined.outcome(votingAddress: checkedAddress, slot: .empty)
        XCTAssertTrue(recordedInEmptySlot.storeKey)
        XCTAssertEqual(recordedInEmptySlot.record, undetermined)
    }

    func testShouldOfferEveryKeyButARefusedOneAndCheckOnlyUnprovenOnes() {
        for (recorded, canVote, checkBeforeUse): (MasternodeRecordedVotingKey?, Bool, Bool) in [
            (nil, true, true),
            (recorded(undetermined, checkedAddress), true, true),
            (recorded(.valid, checkedAddress), true, false),
            (recorded(invalid, checkedAddress), false, false),
        ] {
            for stored in [nil, checkedAddress] {
                let use = MasternodeRecordedVotingKeyUse(recorded: recorded, storedVotingAddress: stored)
                XCTAssertEqual(use.canVote, canVote, "\(String(describing: recorded))")
                XCTAssertEqual(use.checkBeforeUse, checkBeforeUse, "\(String(describing: recorded))")
            }
        }
    }

    /// With the stored key's address known, a record of another key counts
    /// as never checked; without it, the record is taken as it is.
    func testShouldReadARecordOfAnotherKeyOnlyWhenTheStoredAddressIsUnknown() {
        let refusedElsewhere = recorded(invalid, otherAddress)
        let known = MasternodeRecordedVotingKeyUse(recorded: refusedElsewhere, storedVotingAddress: checkedAddress)
        XCTAssertTrue(known.canVote)
        XCTAssertTrue(known.checkBeforeUse)

        let unknown = MasternodeRecordedVotingKeyUse(recorded: refusedElsewhere, storedVotingAddress: nil)
        XCTAssertFalse(unknown.canVote)
        XCTAssertFalse(unknown.checkBeforeUse)
    }

    func testShouldRefuseARecordWhoseAddressIsNotTwentyBytes() {
        XCTAssertNil(MasternodeRecordedVotingKey(check: .valid, votingAddress: Data(repeating: 1, count: 19)))
    }
}
