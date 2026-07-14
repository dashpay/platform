import XCTest
@testable import SwiftExampleApp

/// Pins the launch-time Core SPV auto-start gate matrix — the pure
/// decision extracted from `SwiftExampleAppApp.autoStartCoreSpvIfNeeded`.
/// The interesting invariants are the two asymmetric side effects: a
/// no-wallets launch must NOT latch (so a later call can still start
/// once wallets exist), while an already-running client MUST latch (the
/// auto-start is considered handled).
final class CoreSpvAutoStartTests: XCTestCase {

    // MARK: - Latch gate wins first

    func testAlreadyLatchedSkipsRegardlessOfOtherInputs() {
        // Even with wallets present and SPV stopped (the "would start"
        // combination), a set latch short-circuits to a no-op.
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: true,
            hasWallets: true,
            spvRunning: false
        )
        XCTAssertEqual(decision, .skipAlreadyLatched)
        XCTAssertFalse(decision.shouldStart)
        // Nothing to re-latch — the latch is already set.
        XCTAssertFalse(decision.shouldLatch)
    }

    // MARK: - No-wallets gate does not latch

    func testNoWalletsSkipsWithoutLatching() {
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: false,
            hasWallets: false,
            spvRunning: false
        )
        XCTAssertEqual(decision, .skipNoWallets)
        XCTAssertFalse(decision.shouldStart)
        // Key invariant: do NOT latch, so auto-start stays eligible
        // if wallets appear later this launch.
        XCTAssertFalse(decision.shouldLatch)
    }

    func testNoWalletsPrecedesRunningGateAndStillDoesNotLatch() {
        // The wallet gate is checked before the running gate: with no
        // wallets we skip-without-latch even though a running client
        // would otherwise map to `.latchOnly`.
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: false,
            hasWallets: false,
            spvRunning: true
        )
        XCTAssertEqual(decision, .skipNoWallets)
        XCTAssertFalse(decision.shouldLatch)
        XCTAssertFalse(decision.shouldStart)
    }

    // MARK: - Already-running gate latches without starting

    func testAlreadyRunningLatchesButDoesNotStart() {
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: false,
            hasWallets: true,
            spvRunning: true
        )
        XCTAssertEqual(decision, .latchOnly)
        // Latch (auto-start handled) but never double-start a running
        // client.
        XCTAssertTrue(decision.shouldLatch)
        XCTAssertFalse(decision.shouldStart)
    }

    // MARK: - The only start path

    func testWalletsPresentAndStoppedStartsAndLatches() {
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: false,
            hasWallets: true,
            spvRunning: false
        )
        XCTAssertEqual(decision, .startAndLatch)
        XCTAssertTrue(decision.shouldStart)
        XCTAssertTrue(decision.shouldLatch)
    }
}
