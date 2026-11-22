import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pure logic behind the shielded identity screens: the Type 22 top-up
/// submit gate (`ShieldedTopUpAmountGate`), the spendable-credits state the
/// top-up screen derives from the wallet's local snapshot
/// (`ShieldedSpendableCredits`), the Type 21 recovery-record labels
/// (`ShieldedIdentityDebitRecoveryDisplay`) and the record read state
/// (`RecoveryReadState`).
final class ShieldedIdentityFlowsPresentationTests: XCTestCase {

    // MARK: - Top-up gate

    func test_topUpGate_withAffordableAmount_isOpen() {
        XCTAssertTrue(ShieldedTopUpAmountGate.canSubmit(amount: 100, minimumFee: 10, spendable: 1_000))
    }

    func test_topUpGate_withAmountPlusFeeExactlyAtSpendable_isOpen() {
        XCTAssertTrue(ShieldedTopUpAmountGate.canSubmit(amount: 990, minimumFee: 10, spendable: 1_000))
    }

    func test_topUpGate_withAmountPlusFeeAboveSpendable_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 991, minimumFee: 10, spendable: 1_000))
    }

    func test_topUpGate_withWholeSpendableAsAmount_isClosed() {
        // The fee is spent on top of the amount, so the whole balance never fits.
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 1_000, minimumFee: 10, spendable: 1_000))
    }

    func test_topUpGate_withZeroOrMissingAmount_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 0, minimumFee: 10, spendable: 1_000))
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: nil, minimumFee: 10, spendable: 1_000))
    }

    func test_topUpGate_withUnresolvedFee_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 100, minimumFee: nil, spendable: 1_000))
    }

    func test_topUpGate_withUnknownSpendable_isClosed() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(amount: 100, minimumFee: 10, spendable: nil))
    }

    func test_topUpGate_whenAmountPlusFeeOverflows_isClosedWithoutTrapping() {
        XCTAssertFalse(ShieldedTopUpAmountGate.canSubmit(
            amount: UInt64.max,
            minimumFee: 1,
            spendable: UInt64.max
        ))
    }

    // MARK: - Captured top-up request

    private let walletId = Data(repeating: 0x01, count: 32)
    private let identityId = Data(repeating: 0x02, count: 32)

    func test_capture_shieldedForm_capturesAmountInCredits() {
        let request = ShieldedTopUpRequest.capture(
            fundingIsShielded: true,
            amountText: "0.001",
            walletId: walletId,
            identityId: identityId
        )
        XCTAssertEqual(request, ShieldedTopUpRequest(
            walletId: walletId,
            identityId: identityId,
            amount: 100_000_000
        ))
    }

    func test_capture_otherFundingSource_capturesNothing() {
        XCTAssertNil(ShieldedTopUpRequest.capture(
            fundingIsShielded: false,
            amountText: "0.001",
            walletId: walletId,
            identityId: identityId
        ))
    }

    func test_capture_withoutWalletOrPositiveAmount_capturesNothing() {
        XCTAssertNil(ShieldedTopUpRequest.capture(
            fundingIsShielded: true, amountText: "0.001", walletId: nil, identityId: identityId
        ))
        XCTAssertNil(ShieldedTopUpRequest.capture(
            fundingIsShielded: true, amountText: "0", walletId: walletId, identityId: identityId
        ))
        XCTAssertNil(ShieldedTopUpRequest.capture(
            fundingIsShielded: true, amountText: "abc", walletId: walletId, identityId: identityId
        ))
    }

    func test_capturedRequest_isUnaffectedByLaterFormChanges() {
        // The tap captures 0.001 DASH from the shielded form.
        var amountText = "0.001"
        var fundingIsShielded = true
        let request = ShieldedTopUpRequest.capture(
            fundingIsShielded: fundingIsShielded,
            amountText: amountText,
            walletId: walletId,
            identityId: identityId
        )!
        // While the balance is re-read, the user picks a Platform Payment
        // account, which replaces the amount with that account's balance.
        fundingIsShielded = false
        amountText = "25"
        XCTAssertFalse(fundingIsShielded)
        XCTAssertEqual(amountText, "25")
        // The request that is checked and sent is still the captured one.
        XCTAssertEqual(request.amount, 100_000_000)
        XCTAssertTrue(request.isAffordable(minimumFee: 10, spendable: 100_000_010))
    }

    func test_capturedRequest_isCheckedAgainstTheRefreshedBalance() {
        let request = ShieldedTopUpRequest(walletId: walletId, identityId: identityId, amount: 1_000)
        // Affordable at capture time...
        XCTAssertTrue(request.isAffordable(minimumFee: 10, spendable: 2_000))
        // ...but the refreshed spendable balance (notes reserved meanwhile) is too low.
        XCTAssertFalse(request.isAffordable(minimumFee: 10, spendable: 1_009))
        // An unknown refreshed balance or fee never lets it through.
        XCTAssertFalse(request.isAffordable(minimumFee: 10, spendable: nil))
        XCTAssertFalse(request.isAffordable(minimumFee: nil, spendable: 2_000))
    }

    // MARK: - Spendable shielded credits

    private func ready(
        account: UInt32 = 0,
        credits: UInt64,
        source: ShieldedBalanceSource = .scannedThisSession
    ) -> ShieldedSpendableCredits.Read {
        .state(.ready(ShieldedLocalBalanceSnapshot(accounts: [
            account: ShieldedLocalAccountBalance(
                spendableCredits: credits,
                lastScannedIndex: 10,
                source: source
            ),
        ])))
    }

    private let notRead = ShieldedSpendableCredits.unknown(reason: nil)

    func test_spendable_readyWithCredits_offersTheShieldedSource() {
        let spendable = ShieldedSpendableCredits.after(ready(credits: 500), previous: notRead)
        XCTAssertEqual(spendable, .known(500))
        XCTAssertEqual(spendable.credits, 500)
        XCTAssertTrue(spendable.offersShieldedSource)
        XCTAssertNil(spendable.unavailableReason)
    }

    func test_spendable_readyWithZero_isAKnownZeroNotOffered() {
        let spendable = ShieldedSpendableCredits.after(ready(credits: 0), previous: notRead)
        XCTAssertEqual(spendable, .known(0))
        XCTAssertEqual(spendable.credits, 0)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertNil(spendable.unavailableReason)
    }

    func test_spendable_readyWithRestoredLedger_isKnown() {
        let spendable = ShieldedSpendableCredits.after(
            ready(credits: 7, source: .restored),
            previous: notRead
        )
        XCTAssertEqual(spendable, .known(7))
    }

    func test_spendable_readyWithNoHistory_isUnknownNotZero() {
        let spendable = ShieldedSpendableCredits.after(
            ready(credits: 0, source: .noHistory),
            previous: notRead
        )
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertNotNil(spendable.unavailableReason)
    }

    func test_spendable_readyWithoutAccountZero_isUnknown() {
        let spendable = ShieldedSpendableCredits.after(
            ready(account: 1, credits: 500),
            previous: notRead
        )
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertNotNil(spendable.unavailableReason)
    }

    func test_spendable_unbound_isUnknownWithoutAReason() {
        let spendable = ShieldedSpendableCredits.after(.state(.unbound), previous: .known(500))
        XCTAssertEqual(spendable, .unknown(reason: nil))
        XCTAssertFalse(spendable.offersShieldedSource)
    }

    func test_spendable_restoreIncomplete_isUnknownWithAReason() {
        let spendable = ShieldedSpendableCredits.after(.state(.restoreIncomplete), previous: .known(500))
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertNotNil(spendable.unavailableReason)
    }

    func test_spendable_failedReadAfterAGoodValue_keepsTheGoodValue() {
        let spendable = ShieldedSpendableCredits.after(.failed("busy"), previous: .known(500))
        XCTAssertEqual(spendable, .known(500))
        XCTAssertTrue(spendable.offersShieldedSource)
    }

    func test_spendable_failedReadAfterAKnownZero_keepsTheZero() {
        let spendable = ShieldedSpendableCredits.after(.failed("busy"), previous: .known(0))
        XCTAssertEqual(spendable, .known(0))
    }

    func test_spendable_failedReadWithoutAGoodValue_isUnknownWithTheError() {
        let spendable = ShieldedSpendableCredits.after(.failed("busy"), previous: notRead)
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertTrue(spendable.unavailableReason?.contains("busy") == true)
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

    // MARK: - Recovery read state

    func test_readState_beforeAnyRead_isNotKnownEmpty() {
        let state = RecoveryReadState<Int>()
        XCTAssertEqual(state.status, .notLoaded)
        XCTAssertFalse(state.isKnownEmpty)
        XCTAssertNil(state.failureMessage)
    }

    func test_readState_successfulEmptyRead_isKnownEmpty() {
        var state = RecoveryReadState<Int>()
        state.applySuccess([])
        XCTAssertTrue(state.isKnownEmpty)
    }

    func test_readState_failedFirstRead_isNotKnownEmpty() {
        var state = RecoveryReadState<Int>()
        state.applyFailure("unbound")
        XCTAssertFalse(state.isKnownEmpty)
        XCTAssertEqual(state.failureMessage, "unbound")
        XCTAssertTrue(state.records.isEmpty)
    }

    func test_readState_failedRefresh_keepsLastRecordsAndReportsTheError() {
        var state = RecoveryReadState<Int>()
        state.applySuccess([1, 2])
        state.applyFailure("busy")
        XCTAssertEqual(state.records, [1, 2])
        XCTAssertEqual(state.failureMessage, "busy")
        XCTAssertFalse(state.isKnownEmpty)
    }

    func test_readState_failedRefreshAfterEmptyRead_isNoLongerKnownEmpty() {
        var state = RecoveryReadState<Int>()
        state.applySuccess([])
        state.applyFailure("busy")
        XCTAssertFalse(state.isKnownEmpty)
    }

    // MARK: - Recovery read ordering

    func test_readCoordinator_firstRequest_startsARead() {
        var reads = RecoveryReadCoordinator()
        let generation = reads.request()
        XCTAssertNotNil(generation)
        XCTAssertEqual(reads.inFlight, generation)
    }

    func test_readCoordinator_requestsWhileInFlight_coalesceIntoOneFollowUp() {
        var reads = RecoveryReadCoordinator()
        let first = reads.request()!
        // Two more requests while the first is in flight start nothing.
        XCTAssertNil(reads.request())
        XCTAssertNil(reads.request())
        // The first finishes and applies; exactly one follow-up starts.
        let outcome = reads.finish(first)
        XCTAssertTrue(outcome.apply)
        let followUp = outcome.next!
        XCTAssertGreaterThan(followUp, first)
        // The follow-up finishes with nothing more to run.
        let last = reads.finish(followUp)
        XCTAssertTrue(last.apply)
        XCTAssertNil(last.next)
        XCTAssertNil(reads.inFlight)
    }

    func test_readCoordinator_completionsInReverseOrder_onlyNewestApplies() {
        var reads = RecoveryReadCoordinator()
        // A read is in flight when Stop Retrying starts a superseding one.
        let before = reads.request()!
        let after = reads.startSuperseding()
        // The newer read finishes first and applies.
        XCTAssertTrue(reads.finish(after).apply)
        // The older read finishes last and must not overwrite it.
        let late = reads.finish(before)
        XCTAssertFalse(late.apply)
        XCTAssertNil(late.next)
    }

    func test_readCoordinator_olderReadFinishingFirst_doesNotApplyOrEndTheNewer() {
        var reads = RecoveryReadCoordinator()
        let before = reads.request()!
        let after = reads.startSuperseding()
        XCTAssertFalse(reads.finish(before).apply)
        // The superseding read is still the one in flight.
        XCTAssertEqual(reads.inFlight, after)
        XCTAssertNil(reads.request())
        XCTAssertTrue(reads.finish(after).apply)
    }

    func test_readCoordinator_reset_discardsReadsOfThePreviousWallet() {
        var reads = RecoveryReadCoordinator()
        let oldWalletRead = reads.request()!
        reads.reset()
        // A read for the new wallet starts at once.
        let newWalletRead = reads.request()!
        XCTAssertNotEqual(oldWalletRead, newWalletRead)
        XCTAssertFalse(reads.finish(oldWalletRead).apply)
        XCTAssertTrue(reads.finish(newWalletRead).apply)
    }

    func test_readState_successAfterFailure_clearsTheError() {
        var state = RecoveryReadState<Int>()
        state.applySuccess([1])
        state.applyFailure("busy")
        state.applySuccess([])
        XCTAssertNil(state.failureMessage)
        XCTAssertTrue(state.isKnownEmpty)
    }
}
