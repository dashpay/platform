import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Coverage for how one local balance read changes `ShieldedService`'s
/// per-wallet cache, whether a failed read is retried, and how the
/// network summary adds Rust's per-wallet totals. Pure functions: no
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

    func testShouldStoreAReadySnapshotOverThePreviousEntry() {
        let previous = snapshot(ordinary: 10, accountZero: 10)
        let fresh = snapshot(ordinary: 150, accountZero: 100)
        XCTAssertEqual(
            ShieldedService.cachedLocalBalance(after: .success(.ready(fresh)), previous: previous),
            fresh)
        XCTAssertEqual(
            ShieldedService.cachedLocalBalance(after: .success(.ready(fresh)), previous: nil),
            fresh)
    }

    func testShouldRemoveTheEntryWhenTheWalletIsUnbound() {
        XCTAssertNil(ShieldedService.cachedLocalBalance(
            after: .success(.unbound), previous: snapshot(ordinary: 10, accountZero: 10)))
    }

    func testShouldKeepThePreviousEntryWhenRestoreIsIncomplete() {
        let previous = snapshot(ordinary: 10, accountZero: 10)
        XCTAssertEqual(
            ShieldedService.cachedLocalBalance(after: .success(.restoreIncomplete), previous: previous),
            previous)
        XCTAssertNil(ShieldedService.cachedLocalBalance(after: .success(.restoreIncomplete), previous: nil))
    }

    func testShouldKeepThePreviousEntryWhenTheReadThrows() {
        let previous = snapshot(ordinary: 10, accountZero: 10)
        let errors: [any Error] = [
            PlatformWalletError.walletOperation("busy"),
            CancellationError(),
            ShieldedLocalBalanceReadError.bindingChanged,
        ]
        for error in errors {
            XCTAssertEqual(
                ShieldedService.cachedLocalBalance(after: .failure(error), previous: previous),
                previous, "\(error)")
            XCTAssertNil(ShieldedService.cachedLocalBalance(after: .failure(error), previous: nil))
        }
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

    func testShouldAddKnownWalletTotalsAndCountUnknownAsZero() {
        XCTAssertEqual(ShieldedService.sumKnownBalances([150, nil, 50]), 200)
        XCTAssertEqual(ShieldedService.sumKnownBalances([0, nil]), 0)
    }

    func testShouldReportNoTotalWhenNoWalletIsKnownOrTheSumOverflows() {
        XCTAssertNil(ShieldedService.sumKnownBalances([UInt64?]()))
        XCTAssertNil(ShieldedService.sumKnownBalances([nil, nil]))
        XCTAssertNil(ShieldedService.sumKnownBalances([UInt64.max, 1]))
        XCTAssertEqual(ShieldedService.sumKnownBalances([UInt64.max, 0]), UInt64.max)
    }
}
