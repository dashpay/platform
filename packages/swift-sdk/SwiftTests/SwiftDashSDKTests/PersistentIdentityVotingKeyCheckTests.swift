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
        let address = try XCTUnwrap(MasternodeVotingKeyAddress(Data(repeating: 0x07, count: 20)))
        let refusal = MasternodeRecordedVotingKey(
            check: .invalid(message: "No voting identity exists."),
            votingAddress: address
        )
        identity.recordedVotingKey = refusal
        try context.save()

        let fresh = ModelContext(container)
        let fetched = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentIdentity>()).first)
        XCTAssertFalse(fetched === identity)
        XCTAssertEqual(fetched.votingKeyCheckAddress, address.data)
        XCTAssertEqual(fetched.recordedVotingKey, refusal)

        identity.votingKeyCheck = .valid
        XCTAssertNil(identity.votingKeyCheckAddress, "a check set without its address drops the old one")
        XCTAssertNil(identity.recordedVotingKey, "a check without its address is nothing recorded")

        identity.recordedVotingKey = MasternodeRecordedVotingKey(check: .valid, votingAddress: address)
        identity.recordedVotingKey = nil
        XCTAssertNil(identity.votingKeyCheckRaw)
        XCTAssertNil(identity.votingKeyCheckMessage)
        XCTAssertNil(identity.votingKeyCheckAddress)

        identity.votingKeyCheck = .valid
        identity.votingKeyCheckAddress = Data(repeating: 0x07, count: 19)
        XCTAssertNil(identity.recordedVotingKey, "a stored address that is not 20 bytes is no record")
    }

    /// A refusal recorded for another key does not refuse the stored key once
    /// its address is read: Rust treats it as never checked.
    func testShouldTreatARecordOfAnotherKeyAsNeverChecked() throws {
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x34, count: 32),
            identityType: .masternode,
            network: .testnet
        )
        identity.recordedVotingKey = MasternodeRecordedVotingKey(
            check: .invalid(message: "No voting identity exists."),
            votingAddress: try XCTUnwrap(MasternodeVotingKeyAddress(Data(repeating: 0x09, count: 20)))
        )
        let use = MasternodeRecordedVotingKeyUse(
            recorded: identity.recordedVotingKey,
            storedVotingAddress: MasternodeVotingKeyAddress(Data(repeating: 0x07, count: 20))
        )
        XCTAssertTrue(use.canVote)
        XCTAssertTrue(use.checkBeforeUse)
    }
}
