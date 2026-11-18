import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Coverage for how one local balance read changes `ShieldedService`'s
/// per-wallet cache (a ready snapshot, a known unbound answer, or no
/// entry), whether a failed read is retried, and how the network summary
/// adds Rust's per-wallet totals. Pure functions: no
/// native code runs.
final class ShieldedLocalBalanceCacheTests: XCTestCase {

    private struct Busy: Error {}

    private func snapshot(ordinary: UInt64, accountZero: UInt64) -> ShieldedLocalBalanceSnapshot {
        ShieldedLocalBalanceSnapshot(ordinarySpendableCredits: ordinary, accounts: [
            0: ShieldedLocalAccountBalance(
                spendableCredits: accountZero, lastScannedIndex: 10, source: .restored),
            0x4000_0000: ShieldedLocalAccountBalance(
                spendableCredits: 900, lastScannedIndex: 10, source: .restored),
        ])
    }

    private typealias Entry = ShieldedService.LocalBalanceEntry

    func testShouldStoreAReadySnapshotOverThePreviousEntry() {
        let fresh = snapshot(ordinary: 150, accountZero: 100)
        let previousEntries: [Entry?] = [.ready(snapshot(ordinary: 10, accountZero: 10)), .unbound, nil]
        for previous in previousEntries {
            XCTAssertEqual(
                ShieldedService.cachedLocalBalance(after: .success(.ready(fresh)), previous: previous),
                .ready(fresh))
        }
    }

    func testShouldRememberAnUnboundAnswerAsKnown() {
        let previousEntries: [Entry?] = [.ready(snapshot(ordinary: 10, accountZero: 10)), .unbound, nil]
        for previous in previousEntries {
            XCTAssertEqual(
                ShieldedService.cachedLocalBalance(after: .success(.unbound), previous: previous),
                .unbound)
        }
    }

    func testShouldKeepAPreviousSnapshotWhenRestoreIsIncomplete() {
        let previous = Entry.ready(snapshot(ordinary: 10, accountZero: 10))
        XCTAssertEqual(
            ShieldedService.cachedLocalBalance(after: .success(.restoreIncomplete), previous: previous),
            previous)
        XCTAssertNil(ShieldedService.cachedLocalBalance(after: .success(.restoreIncomplete), previous: nil))
    }

    func testShouldForgetAnUnboundAnswerWhenRestoreIsIncomplete() {
        // Accounts are bound now, so "nothing bound" no longer holds.
        XCTAssertNil(ShieldedService.cachedLocalBalance(after: .success(.restoreIncomplete), previous: .unbound))
    }

    func testShouldKeepThePreviousEntryWhenTheReadThrows() {
        let errors: [any Error] = [
            PlatformWalletError.walletOperation("busy"),
            CancellationError(),
            ShieldedLocalBalanceReadError.bindingChanged,
        ]
        let previousEntries: [Entry?] = [.ready(snapshot(ordinary: 10, accountZero: 10)), .unbound, nil]
        for error in errors {
            for previous in previousEntries {
                XCTAssertEqual(
                    ShieldedService.cachedLocalBalance(after: .failure(error), previous: previous),
                    previous, "\(error)")
            }
        }
    }

    func testShouldForgetOnlyAnUnboundAnswerAfterABind() {
        let ready = Entry.ready(snapshot(ordinary: 10, accountZero: 10))
        XCTAssertEqual(ShieldedService.cachedLocalBalanceAfterBind(previous: ready), ready)
        XCTAssertNil(ShieldedService.cachedLocalBalanceAfterBind(previous: .unbound))
        XCTAssertNil(ShieldedService.cachedLocalBalanceAfterBind(previous: nil))
    }

    func testShouldRetryThrownErrorsButNotCancellationOrResults() {
        XCTAssertTrue(ShieldedService.shouldRetryLocalBalanceRead(
            after: .failure(PlatformWalletError.walletOperation("busy"))))
        XCTAssertTrue(ShieldedService.shouldRetryLocalBalanceRead(
            after: .failure(ShieldedLocalBalanceReadError.bindingChanged)))
        XCTAssertTrue(ShieldedService.shouldRetryLocalBalanceRead(after: .failure(Busy())))
        XCTAssertFalse(ShieldedService.shouldRetryLocalBalanceRead(after: .failure(CancellationError())))
        XCTAssertFalse(ShieldedService.shouldRetryLocalBalanceRead(
            after: .success(.ready(snapshot(ordinary: 1, accountZero: 1)))))
        XCTAssertFalse(ShieldedService.shouldRetryLocalBalanceRead(after: .success(.unbound)))
        XCTAssertFalse(ShieldedService.shouldRetryLocalBalanceRead(after: .success(.restoreIncomplete)))
    }

    func testShouldRetryABoundedNumberOfTimes() {
        XCTAssertEqual(
            ShieldedService.localBalanceRetryDelays,
            [.milliseconds(300), .seconds(1), .seconds(3)])
    }

    func testShouldReportACompleteTotalWhenEveryWalletIsKnown() {
        XCTAssertEqual(
            ShieldedService.networkShieldedTotal([
                .ready(snapshot(ordinary: 150, accountZero: 100)),
                .ready(snapshot(ordinary: 50, accountZero: 50)),
            ]),
            .complete(200))
        XCTAssertEqual(ShieldedService.networkShieldedTotal([Entry?]()), .complete(0))
    }

    func testShouldCountAnUnboundWalletAsKnownWithNothingToAdd() {
        XCTAssertEqual(
            ShieldedService.networkShieldedTotal([.ready(snapshot(ordinary: 150, accountZero: 100)), .unbound]),
            .complete(150))
        XCTAssertEqual(ShieldedService.networkShieldedTotal([Entry.unbound]), .complete(0))
    }

    func testShouldMarkTheTotalPartialWhileAWalletIsNotKnown() {
        XCTAssertEqual(
            ShieldedService.networkShieldedTotal([.ready(snapshot(ordinary: 150, accountZero: 100)), nil]),
            .partial(150))
        XCTAssertEqual(ShieldedService.networkShieldedTotal([nil, .unbound]), .partial(0))
    }

    func testShouldReportOverflowWhenTheKnownBalancesDoNotFit() {
        XCTAssertEqual(
            ShieldedService.networkShieldedTotal([
                .ready(snapshot(ordinary: UInt64.max, accountZero: 0)),
                .ready(snapshot(ordinary: 1, accountZero: 0)),
                nil,
            ]),
            .overflow)
        XCTAssertEqual(
            ShieldedService.networkShieldedTotal([.ready(snapshot(ordinary: UInt64.max, accountZero: 0)), .unbound]),
            .complete(UInt64.max))
    }
}
