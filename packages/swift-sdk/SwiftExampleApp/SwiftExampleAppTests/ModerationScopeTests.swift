import XCTest
@testable import SwiftExampleApp

/// Pins the gate of the contract moderation hub, its screens and the team
/// action detail: they read and show only while the connection the contract
/// was opened on is still the app's.
final class ModerationScopeTests: XCTestCase {
    private let contractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"

    /// A devnet edit keeps the network and can land on a chain holding the
    /// same contract id: the hub opened before it is closed all the same.
    func testShouldCloseTheHubAfterASameNetworkReplacementWithTheSameContractId() {
        let opened = ModerationScope(contractId: contractId, generation: 3)

        XCTAssertTrue(opened.isOpen(currentGeneration: 3, hasSDK: true))
        XCTAssertFalse(opened.isOpen(currentGeneration: 4, hasSDK: true))

        // Reopening the same contract on the new connection is a different scope
        let reopened = ModerationScope(contractId: contractId, generation: 4)
        XCTAssertNotEqual(opened, reopened)
        XCTAssertTrue(reopened.isOpen(currentGeneration: 4, hasSDK: true))
    }

    /// Without an SDK nothing is open.
    func testShouldCloseTheHubWithoutAnSDK() {
        let opened = ModerationScope(contractId: contractId, generation: 3)
        XCTAssertFalse(opened.isOpen(currentGeneration: 3, hasSDK: false))
    }

    /// The team action detail is a screen of the hub: the same scope closes it
    /// whole, its action, proposer, approvals and event included.
    func testShouldCloseTheTeamActionDetailWithItsHub() {
        let opened = ModerationScope(contractId: contractId, generation: 7)
        let afterReplacement = 8
        XCTAssertFalse(opened.isOpen(currentGeneration: afterReplacement, hasSDK: true))
    }
}
