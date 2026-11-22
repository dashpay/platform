import SwiftData
import XCTest

@testable import SwiftDashSDK

@MainActor
final class PersistentIdentityVotingKeyCheckTests: XCTestCase {
    /// A row stored before keys were checked reads as never checked, and each
    /// verdict reads back with its message after a save.
    func testShouldRoundTripEachVotingKeyCheckThroughTheStore() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x31, count: 32),
            identityType: .masternode,
            network: .testnet
        )
        context.insert(identity)
        try context.save()
        XCTAssertNil(identity.votingKeyCheck)
        XCTAssertNil(identity.votingKeyCheckRaw)

        for check: MasternodeVotingKeyCheck in [
            .invalid(message: "No voting identity exists."),
            .undetermined(message: "The voting key could not be checked against Platform: timeout."),
            .valid,
        ] {
            identity.votingKeyCheck = check
            try context.save()
            // A fresh context reads the columns back from the store rather
            // than returning the object this context already holds.
            let fresh = ModelContext(container)
            let fetched = try XCTUnwrap(
                fresh.fetch(FetchDescriptor<PersistentIdentity>()).first
            )
            XCTAssertFalse(fetched === identity)
            XCTAssertEqual(fetched.votingKeyCheck, check)
        }
        XCTAssertNil(identity.votingKeyCheckMessage, "a valid check keeps no message")

        identity.votingKeyCheck = nil
        XCTAssertNil(identity.votingKeyCheckRaw)
        XCTAssertNil(identity.votingKeyCheckMessage)
    }

    func testShouldReadAnUnknownStoredValueAsNeverChecked() {
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x32, count: 32),
            network: .testnet
        )
        identity.votingKeyCheckRaw = "a value from a later build"
        XCTAssertNil(identity.votingKeyCheck)
    }

    /// A check recorded with its key's address reads back with it from a
    /// fresh context; a check set without an address, or cleared, has none.
    func testShouldRoundTripTheCheckedVotingAddressThroughTheStore() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x33, count: 32),
            identityType: .masternode,
            network: .testnet
        )
        context.insert(identity)
        let address = Data(repeating: 0x07, count: 20)
        let refusal = MasternodeVotingKeyCheck.invalid(message: "No voting identity exists.")
        identity.setVotingKeyCheck(refusal, votingAddress: address)
        try context.save()

        let fresh = ModelContext(container)
        let fetched = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentIdentity>()).first)
        XCTAssertFalse(fetched === identity)
        XCTAssertEqual(fetched.votingKeyCheckAddress, address)
        XCTAssertEqual(
            fetched.recordedVotingKey,
            MasternodeRecordedVotingKey(check: refusal, votingAddress: address)
        )

        identity.votingKeyCheck = .valid
        XCTAssertNil(identity.votingKeyCheckAddress, "a check set without its address drops the old one")
        XCTAssertNil(identity.recordedVotingKey, "a check without its address is nothing recorded")

        identity.setVotingKeyCheck(.valid, votingAddress: address)
        identity.votingKeyCheck = nil
        XCTAssertNil(identity.votingKeyCheckRaw)
        XCTAssertNil(identity.votingKeyCheckMessage)
        XCTAssertNil(identity.votingKeyCheckAddress)
    }

    /// A refusal recorded for another key does not refuse the stored key once
    /// its address is read: Rust treats it as never checked.
    func testShouldTreatARecordOfAnotherKeyAsNeverChecked() {
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x34, count: 32),
            identityType: .masternode,
            network: .testnet
        )
        identity.setVotingKeyCheck(
            .invalid(message: "No voting identity exists."),
            votingAddress: Data(repeating: 0x09, count: 20)
        )
        let use = MasternodeRecordedVotingKeyUse(
            recorded: identity.recordedVotingKey,
            storedVotingAddress: Data(repeating: 0x07, count: 20)
        )
        XCTAssertTrue(use.canVote)
        XCTAssertTrue(use.checkBeforeUse)
    }
}
