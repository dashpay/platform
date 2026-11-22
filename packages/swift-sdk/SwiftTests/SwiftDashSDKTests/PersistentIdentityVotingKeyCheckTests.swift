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
            let fetched = try XCTUnwrap(
                context.fetch(FetchDescriptor<PersistentIdentity>()).first
            )
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
}
