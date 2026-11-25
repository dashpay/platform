import SwiftDashSDK
import XCTest
@testable import SwiftExampleApp

/// Pins that dismissing a funding's sheet never removes a running funding.
/// A completed controller's sheet can stay open while `startFunding` hands
/// its slot to a new operation (a Max to the same own address, say), and a
/// failed sheet can stay open while a retry puts the SAME controller back
/// in flight. The SDK rebuild guard reads `hasInFlightFundings`, so
/// dropping a running controller would let a rebuild stop SPV under it.
/// Also pins that `startFunding` reports whether it scheduled the body.
@MainActor
final class ShieldedFundFromAssetLockCoordinatorTests: XCTestCase {

    private struct TimedOut: Error {}
    /// A refusal Rust classed as nothing sent and retryable: the failure
    /// releases its slot, so a retry or another operation may start and a
    /// stale sheet's Dismiss drops it.
    private static func refused() -> Error {
        PlatformWalletError.walletOperation(
            "refused", payment: PlatformPaymentClass(delivery: .notSent, isRetryable: true))
    }

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

    private func isFailed(_ controller: ShieldedFundFromAssetLockController) -> Bool {
        if case .failed = controller.phase { return true }
        return false
    }

    func testShouldKeepANewerInFlightFundingWhenAStaleSheetIsDismissed() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xA1, count: 32)
        let recipient = Data(repeating: 0x5C, count: 43)

        guard case .started(let first, _) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:first",
            body: {}
        ) else {
            return XCTFail("the first funding did not start")
        }
        try await waitUntil { first.phase == .completed }

        let gate = Gate()
        guard case .started(let second, _) = coordinator.startFunding(
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

    func testShouldKeepARetriedFundingWhenItsFailedSheetIsDismissed() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xA2, count: 32)
        let recipient = Data(repeating: 0x5D, count: 43)

        guard case .started(let first, _) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:retried",
            body: { throw Self.refused() }
        ) else {
            return XCTFail("the first attempt did not start")
        }
        try await waitUntil { self.isFailed(first) }

        let gate = Gate()
        guard case .started(let retry, _) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:retried",
            body: { await gate.hold() }
        ) else {
            gate.open()
            return XCTFail("the retry did not start")
        }
        XCTAssertTrue(retry === first)
        XCTAssertEqual(first.phase, .inFlight)

        // The first attempt's failed sheet taps Dismiss after the retry
        // started: the controller is running again, so it stays.
        coordinator.dismiss(first)
        XCTAssertTrue(coordinator.controller(walletId: walletId, recipientRaw43: recipient) === first)
        XCTAssertTrue(coordinator.hasInFlightFundings)

        gate.open()
        try await waitUntil { first.phase == .completed }
        XCTAssertFalse(coordinator.hasInFlightFundings)

        coordinator.dismiss(first)
        XCTAssertNil(coordinator.controller(walletId: walletId, recipientRaw43: recipient))
    }

    func testShouldReportBodyScheduledOnlyWhenStartFundingStartedTheBody() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xA3, count: 32)
        let recipient = Data(repeating: 0x5E, count: 43)
        let gate = Gate()

        guard case .started(let controller, let firstScheduled) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "drain:op",
            body: {
                await gate.hold()
                throw Self.refused()
            }
        ) else {
            gate.open()
            return XCTFail("the first attempt did not start")
        }
        XCTAssertTrue(firstScheduled)

        // A re-tap of the in-flight operation only re-presents it.
        guard case .started(let reTapped, let reTapScheduled) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "drain:op",
            body: {}
        ) else {
            gate.open()
            return XCTFail("the in-flight re-tap was not re-presented")
        }
        XCTAssertTrue(reTapped === controller)
        XCTAssertFalse(reTapScheduled)

        gate.open()
        try await waitUntil { self.isFailed(controller) }

        // A retry on the failed slot reuses the same controller and does
        // run its body.
        guard case .started(let retried, let retryScheduled) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "drain:op",
            body: {}
        ) else {
            return XCTFail("the retry did not start")
        }
        XCTAssertTrue(retried === controller)
        XCTAssertTrue(retryScheduled)
        try await waitUntil { controller.phase == .completed }

        // A re-tap after success re-presents the terminal state only.
        guard case .started(_, let afterSuccessScheduled) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "drain:op",
            body: {}
        ) else {
            return XCTFail("the completed re-tap was not re-presented")
        }
        XCTAssertFalse(afterSuccessScheduled)
    }

    func testShouldGiveADifferentOperationAFreshControllerWhenTheSlotHoldsAFailedOne() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xA4, count: 32)
        let recipient = Data(repeating: 0x5F, count: 43)

        guard case .started(let failedA, _) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:a",
            body: { throw Self.refused() }
        ) else {
            return XCTFail("operation A did not start")
        }
        try await waitUntil { self.isFailed(failedA) }

        let gate = Gate()
        guard case .started(let b, let bScheduled) = coordinator.startFunding(
            walletId: walletId,
            recipientRaw43: recipient,
            operationId: "shield:b",
            body: { await gate.hold() }
        ) else {
            gate.open()
            return XCTFail("operation B did not start")
        }
        XCTAssertTrue(bScheduled)
        XCTAssertFalse(b === failedA)
        XCTAssertEqual(b.operationId, "shield:b")
        XCTAssertEqual(b.phase, .inFlight)
        XCTAssertTrue(coordinator.controller(walletId: walletId, recipientRaw43: recipient) === b)
        // A's sheet still observes A, so it must still show A's failure.
        XCTAssertTrue(isFailed(failedA))
        XCTAssertEqual(failedA.operationId, "shield:a")

        // A's stale sheet taps Dismiss: the slot belongs to B now.
        coordinator.dismiss(failedA)
        XCTAssertTrue(coordinator.controller(walletId: walletId, recipientRaw43: recipient) === b)
        XCTAssertTrue(coordinator.hasInFlightFundings)

        gate.open()
        try await waitUntil { b.phase == .completed }
        XCTAssertTrue(isFailed(failedA))
        XCTAssertEqual(failedA.operationId, "shield:a")
    }
}
