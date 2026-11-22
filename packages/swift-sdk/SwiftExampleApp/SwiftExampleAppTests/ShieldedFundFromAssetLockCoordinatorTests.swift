import XCTest
@testable import SwiftExampleApp

/// Pins that dismissing a funding's sheet removes only that funding. A
/// completed controller's sheet can stay open while `startFunding` hands
/// its slot to a new operation (a Max to the same own address, say). The
/// SDK rebuild guard reads `hasInFlightFundings`, so dropping the newer,
/// still running controller would let a rebuild stop SPV under it.
@MainActor
final class ShieldedFundFromAssetLockCoordinatorTests: XCTestCase {

    private struct TimedOut: Error {}

    /// A funding body held open until the test opens it.
    @MainActor
    private final class Gate {
        private var continuation: CheckedContinuation<Void, Never>?
        private var isOpen = false

        func hold() async {
            guard !isOpen else { return }
            await withCheckedContinuation { continuation = $0 }
        }

        /// Also covers a body that has not reached `hold()` yet.
        func open() {
            isOpen = true
            continuation?.resume()
            continuation = nil
        }
    }

    /// Wait for `condition`, failing instead of hanging.
    private func waitUntil(_ condition: () -> Bool) async throws {
        let deadline = ContinuousClock.now + .seconds(10)
        while !condition() {
            guard ContinuousClock.now < deadline else { throw TimedOut() }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    func testShouldKeepANewerInFlightFundingWhenAStaleSheetIsDismissed() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xA1, count: 32)
        let recipient = Data(repeating: 0x5C, count: 43)

        guard case .started(let first) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:first",
            body: {}
        ) else {
            return XCTFail("the first funding did not start")
        }
        try await waitUntil { first.phase == .completed }

        let gate = Gate()
        guard case .started(let second) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:second",
            body: { await gate.hold() }
        ) else {
            gate.open()
            return XCTFail("the second funding did not start")
        }
        XCTAssertFalse(second === first)
        XCTAssertTrue(coordinator.controller(walletId: walletId, recipientRaw43: recipient) === second)
        XCTAssertEqual(second.phase, .inFlight)

        // The stale sheet's Done: the slot now belongs to the second funding.
        coordinator.dismiss(first)
        XCTAssertTrue(coordinator.controller(walletId: walletId, recipientRaw43: recipient) === second)
        XCTAssertTrue(coordinator.hasInFlightFundings)

        gate.open()
        try await waitUntil { second.phase == .completed }
        XCTAssertFalse(coordinator.hasInFlightFundings)

        coordinator.dismiss(second)
        XCTAssertNil(coordinator.controller(walletId: walletId, recipientRaw43: recipient))
    }
}
