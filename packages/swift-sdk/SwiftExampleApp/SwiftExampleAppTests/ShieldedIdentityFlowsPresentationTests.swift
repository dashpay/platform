import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pure logic behind the shielded identity screens: the Type 22 top-up
/// submit gate (`ShieldedTopUpAmountGate`) and the labels for the Type 21
/// recovery records (`ShieldedIdentityDebitRecoveryDisplay`).
final class ShieldedIdentityFlowsPresentationTests: XCTestCase {

    // MARK: - Top-up gate

    func test_topUpGate_withAffordableAmount_isOpen() {
        XCTAssertTrue(ShieldedTopUpAmountGate.canSubmit(amount: 100, fee: 10, poolBalance: 1_000))
    }

    func test_topUpGate_withAmountPlusFeeExactlyAtBalance_isOpen() {
        XCTAssertTrue(ShieldedTopUpAmountGate.canSubmit(amount: 990, fee: 10, poolBalance: 1_000))
    }

    func test_topUpGate_withAmountPlusFeeAboveBalance_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 991, fee: 10, poolBalance: 1_000))
    }

    func test_topUpGate_withWholeBalanceAsAmount_isClosed() {
        // The fee is spent on top of the amount, so the whole balance never fits.
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 1_000, fee: 10, poolBalance: 1_000))
    }

    func test_topUpGate_withZeroOrMissingAmount_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 0, fee: 10, poolBalance: 1_000))
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: nil, fee: 10, poolBalance: 1_000))
    }

    func test_topUpGate_withUnresolvedFee_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 100, fee: nil, poolBalance: 1_000))
    }

    func test_topUpGate_whenAmountPlusFeeOverflows_isClosedWithoutTrapping() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(
            amount: UInt64.max,
            fee: 1,
            poolBalance: UInt64.max
        ))
    }

    // MARK: - Recovery record labels

    func test_recoveryDisplay_offersStopRetryingOnlyForActiveRecords() {
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.retrying))
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.parked))
        XCTAssertFalse(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.unknown))
    }

    func test_recoveryDisplay_unknownStatus_neverReadsAsFailed() {
        let title = ShieldedIdentityDebitRecoveryDisplay.statusTitle(.unknown)
        let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.unknown)
        XCTAssertFalse(title.lowercased().contains("fail"))
        XCTAssertFalse(detail.lowercased().contains("fail"))
    }

    func test_recoveryDisplay_stopWarning_statesTheRisks() {
        let warning = ShieldedIdentityDebitRecoveryDisplay.stopRetryingWarning
        XCTAssertTrue(warning.contains("does not cancel"))
        XCTAssertTrue(warning.contains("may already have executed"))
        XCTAssertTrue(warning.contains("additional debit"))
    }
}
