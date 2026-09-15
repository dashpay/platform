import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// The app-owned shielded tip guard: one submission per network and wallet,
/// locked from the first tap, and unlocked only by a confirmed result. A
/// dismissed sheet reopening on the same wallet must find the same guard.
@MainActor
final class ShieldedTipSubmissionTests: XCTestCase {
    /// Parks a submission until the test releases it, like a native send in flight.
    @MainActor
    private final class Gate {
        private var continuation: CheckedContinuation<Void, Never>?
        private var opened = false
        func wait() async {
            if opened { return }
            await withCheckedContinuation { continuation = $0 }
        }
        func open() {
            opened = true
            continuation?.resume()
            continuation = nil
        }
    }

    @MainActor
    private final class Counter {
        var sends = 0
    }

    func testInFlightSendRefusesASecondSubmissionAndReopeningFindsTheSameGuard() async {
        let submissions = ShieldedTipSubmissions()
        let walletId = Data(repeating: 0x31, count: 32)
        let gate = Gate()
        let counter = Counter()

        let submission = submissions.forWallet(network: .testnet, walletId: walletId)
        let task = submission.submit {
            counter.sends += 1
            await gate.wait()
        }
        XCTAssertNotNil(task)
        XCTAssertTrue(submission.busy)

        // The sheet was dismissed and reopened: the owner hands back the same
        // guard, which is still locked, so the second tap never sends.
        let reopened = submissions.forWallet(network: .testnet, walletId: walletId)
        XCTAssertTrue(reopened === submission)
        XCTAssertNil(reopened.submit { counter.sends += 1 })

        gate.open()
        await task?.value
        XCTAssertEqual(counter.sends, 1)
        XCTAssertEqual(submission.status, .sent)
        XCTAssertTrue(submission.submitted)

        // Another payment needs an explicit action after the confirmed result.
        submission.startNewTip()
        XCTAssertEqual(submission.status, .ready)
        XCTAssertFalse(submission.submitted)

        // Another wallet on the same network has its own guard.
        let other = submissions.forWallet(network: .testnet, walletId: Data(repeating: 0x32, count: 32))
        XCTAssertFalse(other === submission)
    }

    func testUnconfirmedSpendStaysLockedAcrossReopenAndCannotStartANewTip() async {
        let submission = ShieldedTipSubmission()
        let task = submission.submit {
            throw PlatformWalletError.shieldedSpendUnconfirmed("relay accepted the transition")
        }
        await task?.value
        XCTAssertEqual(submission.status, .uncertain)
        XCTAssertTrue(submission.submitted)
        XCTAssertTrue(submission.message?.contains("may have been sent") == true)

        // Neither a fresh tap nor "send another" unlocks an uncertain outcome.
        XCTAssertNil(submission.submit {})
        submission.startNewTip()
        XCTAssertEqual(submission.status, .uncertain)
    }

    func testPreflightFailureReturnsToReadyWhileUnknownFailuresLock() async {
        let submission = ShieldedTipSubmission()
        await submission.submit {
            throw PlatformWalletError.walletOperation("insufficient shielded notes")
        }?.value
        XCTAssertEqual(submission.status, .ready)
        XCTAssertEqual(
            submission.message,
            PlatformWalletError.walletOperation("insufficient shielded notes").localizedDescription)

        struct Unknown: Error {}
        await submission.submit { throw Unknown() }?.value
        XCTAssertEqual(submission.status, .uncertain)
        XCTAssertNil(submission.submit {})
    }
}
