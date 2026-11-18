import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// The network summary's total over Rust's pushed per-wallet balances. A
/// wallet counts only through `ShieldedBalanceUpdate.ordinaryBalance`, so
/// an unbound wallet, a ledger not established yet, or a wallet with no
/// update yet makes the total partial.
final class NetworkShieldedTotalTests: XCTestCase {

    private func update(_ state: ShieldedBalanceState) -> ShieldedBalanceUpdate {
        ShieldedBalanceUpdate(walletId: Data(repeating: 1, count: 32), revision: 1, state: state)
    }

    private func ready(_ credits: UInt64, established: Bool = true) -> ShieldedBalanceUpdate {
        update(.ready(ShieldedReadyBalance(
            ordinarySpendableCredits: credits,
            ordinaryBalanceEstablished: established,
            accounts: [:])))
    }

    private func total(_ updates: [ShieldedBalanceUpdate?]) -> NetworkShieldedTotal {
        NetworkShieldedTotal(updates.map { $0?.ordinaryBalance })
    }

    func testShouldBeCompleteWhenEveryWalletHasAnEstablishedBalance() {
        XCTAssertEqual(total([ready(150), ready(50)]), .complete(200))
        XCTAssertEqual(total([ready(0)]), .complete(0))
        XCTAssertEqual(total([]), .complete(0))
    }

    func testShouldBePartialWithAnUnboundWallet() {
        XCTAssertEqual(total([ready(150), update(.unbound)]), .partial(150))
        XCTAssertEqual(total([update(.unbound)]), .partial(0))
    }

    func testShouldBePartialWithAWalletWhoseBalanceIsNotEstablished() {
        XCTAssertEqual(total([ready(150), ready(0, established: false)]), .partial(150))
        XCTAssertEqual(total([ready(150), update(.restoreIncomplete)]), .partial(150))
        XCTAssertEqual(total([ready(150), nil]), .partial(150))
    }

    func testShouldReportOverflowWhenTheKnownBalancesDoNotFit() {
        XCTAssertEqual(total([ready(UInt64.max), ready(1)]), .overflow)
        XCTAssertEqual(total([ready(UInt64.max), ready(0)]), .complete(UInt64.max))
    }
}
