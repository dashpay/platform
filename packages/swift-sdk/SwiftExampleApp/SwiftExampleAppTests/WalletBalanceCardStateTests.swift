import XCTest
@testable import SwiftExampleApp

final class WalletBalanceCardStateTests: XCTestCase {
    func testAllZeroBalancesAreEmpty() {
        XCTAssertTrue(
            WalletBalanceCardState.isEmpty(
                confirmedCore: 0,
                unconfirmedCore: 0,
                immatureCore: 0,
                platform: 0,
                shielded: 0
            )
        )
    }

    func testImmatureOnlyCoreFundsAreNotEmpty() {
        XCTAssertFalse(
            WalletBalanceCardState.isEmpty(
                confirmedCore: 0,
                unconfirmedCore: 0,
                immatureCore: 50_000_000,
                platform: 0,
                shielded: 0
            )
        )
    }
}
