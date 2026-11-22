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

    func test_capturedRequest_isUnaffectedByLaterFormChanges() throws {
        // The tap captures 0.001 DASH from the shielded form.
        var amountText = "0.001"
        var fundingIsShielded = true
        let request = try XCTUnwrap(ShieldedTopUpRequest.capture(
            fundingIsShielded: fundingIsShielded,
            amountText: amountText,
            walletId: walletId,
            identityId: identityId
        ))
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

    // MARK: - Exact credit amounts

    /// One credit in the formatter's own locale-aware output.
    private var oneCreditDash: String { formatTokenAmount(1, decimals: 11) + " DASH" }

    func test_oneCredit_withAFixedLocale_isTheExactLiteral() {
        let posix = Locale(identifier: "en_US_POSIX")
        XCTAssertEqual(formatCreditsAsDash(1, locale: posix), "0.00000000001 DASH")
        XCTAssertEqual(formatCreditsAsDash(100_000_000_001, locale: posix), "1.00000000001 DASH")
        XCTAssertEqual(formatCreditsAsDash(0, locale: posix), "0 DASH")
    }

    func test_oneCredit_isShownExactlyNotAsZero() {
        XCTAssertEqual(formatCreditsAsDash(1), oneCreditDash)
        XCTAssertNotEqual(formatCreditsAsDash(1), formatCreditsAsDash(0))
    }

    func test_topUpResults_showAOneCreditAmountExactly() {
        XCTAssertTrue(ShieldedTopUpSubmitCheck.completedText(amount: 1).contains("Requested: \(oneCreditDash)."))
        XCTAssertTrue(ShieldedTopUpSubmitCheck.unconfirmedText(amount: 1, pendingListedBelow: true)
            .contains("The top-up of \(oneCreditDash) "))
    }

    func test_recoveryRowAmount_showsAOneCreditAmountExactly() {
        XCTAssertEqual(ShieldedIdentityDebitRecoveryDisplay.amountText(1), oneCreditDash)
    }

    func test_identityBalanceRow_showsAOneCreditChange() {
        // After a one-credit top-up the balance row must read differently.
        XCTAssertNotEqual(
            formatIdentityBalanceAsDash(100_000_000_000),
            formatIdentityBalanceAsDash(100_000_000_001)
        )
        XCTAssertEqual(formatIdentityBalanceAsDash(1), oneCreditDash)
    }

    func test_identityBalanceRow_clampsANegativeBalanceToZero() {
        XCTAssertEqual(formatIdentityBalanceAsDash(-5), formatCreditsAsDash(0))
        XCTAssertEqual(formatIdentityBalanceAsDash(Int64.min), formatCreditsAsDash(0))
    }

    func test_activityAmount_oneCredit_isExactWithEachDirectionsSign() {
        XCTAssertEqual(PersistentShieldedActivity.signedAmountText(amount: 1, direction: 0), "+" + oneCreditDash)
        XCTAssertEqual(PersistentShieldedActivity.signedAmountText(amount: 1, direction: 1), "-" + oneCreditDash)
        XCTAssertEqual(PersistentShieldedActivity.signedAmountText(amount: 1, direction: 2), oneCreditDash)
        XCTAssertNotEqual(
            PersistentShieldedActivity.signedAmountText(amount: 1, direction: 1),
            PersistentShieldedActivity.signedAmountText(amount: 0, direction: 1)
        )
    }

    func test_amountsOneCreditApart_readDifferently() {
        // One DASH and one DASH plus one credit.
        XCTAssertNotEqual(formatCreditsAsDash(100_000_000_000), formatCreditsAsDash(100_000_000_001))
    }

    func test_unconfirmedText_withPendingRows_pointsAtThemWithoutClaimingOne() {
        let text = ShieldedTopUpSubmitCheck.unconfirmedText(amount: 1, pendingListedBelow: true)
        XCTAssertTrue(text.contains("Do not retry"))
        XCTAssertTrue(text.contains("Pending top-ups for this identity are listed below."))
        // Rows are not matched to this submission, so none is named as it.
        XCTAssertFalse(text.contains("It is listed"))
    }

    func test_unconfirmedText_withoutPendingRows_promisesNoVisibleEntry() {
        let text = ShieldedTopUpSubmitCheck.unconfirmedText(amount: 1, pendingListedBelow: false)
        XCTAssertTrue(text.contains("Do not retry"))
        XCTAssertFalse(text.contains("listed"))
    }

    // MARK: - Check Now outcome

    func test_checkNow_normalReturn_isRequestedWithAnUnknownOutcome() {
        let status = ShieldedCheckNowStatus.after(.requested)
        XCTAssertEqual(status, .requested)
        XCTAssertFalse(status.isFailure)
        let text = status.text
        XCTAssertTrue(text?.contains("Sync requested") == true)
        XCTAssertTrue(text?.contains("cannot tell whether the scan for this wallet succeeded") == true)
        // A normal return is never reported as a finished or successful check.
        XCTAssertFalse(text?.lowercased().contains("finished") == true)
        XCTAssertFalse(text?.lowercased().contains("success") == true)
    }

    func test_checkNow_failedPass_showsThisChecksError() {
        let status = ShieldedCheckNowStatus.after(.failed("network down"))
        XCTAssertEqual(status, .failed("network down"))
        XCTAssertTrue(status.isFailure)
        XCTAssertTrue(status.text?.contains("network down") == true)
    }

    func test_checkNow_passAlreadyRunningOrNothingBound_didNotRun() {
        XCTAssertTrue(ShieldedCheckNowStatus.after(.alreadyRunning).isFailure)
        let notStarted = ShieldedCheckNowStatus.after(.notStarted("No wallet manager is bound."))
        XCTAssertEqual(notStarted, .didNotRun("No wallet manager is bound."))
        XCTAssertTrue(notStarted.text?.contains("did not run") == true)
    }

    func test_checkNow_idleOrChecking_showsNothing() {
        XCTAssertNil(ShieldedCheckNowStatus.idle.text)
        XCTAssertNil(ShieldedCheckNowStatus.checking.text)
    }

    // MARK: - Read scope (wallet manager replacement)

    func test_readScope_sameManagerAndWallet_appliesTheRead() {
        let manager = NSObject()
        let scope = ShieldedReadScope(manager: ObjectIdentifier(manager), walletId: walletId)
        let applied = ShieldedSpendableCredits.applying(
            ready(credits: 5), startedFor: scope, current: scope, previous: .unknown(reason: nil)
        )
        XCTAssertEqual(applied, .known(5))
    }

    func test_readScope_replacedManager_discardsTheOldRead() {
        let oldManager = NSObject()
        let newManager = NSObject()
        let started = ShieldedReadScope(manager: ObjectIdentifier(oldManager), walletId: walletId)
        let current = ShieldedReadScope(manager: ObjectIdentifier(newManager), walletId: walletId)
        // The view reset to unknown on the swap; the old manager's read must not undo that.
        let applied = ShieldedSpendableCredits.applying(
            ready(credits: 5), startedFor: started, current: current, previous: .unknown(reason: nil)
        )
        XCTAssertEqual(applied, .unknown(reason: nil))
    }

    func test_readScope_otherWalletOrNoWallet_discardsTheRead() {
        let manager = NSObject()
        let started = ShieldedReadScope(manager: ObjectIdentifier(manager), walletId: walletId)
        let otherWallet = ShieldedReadScope(manager: ObjectIdentifier(manager), walletId: identityId)
        XCTAssertEqual(
            ShieldedSpendableCredits.applying(ready(credits: 5), startedFor: started, current: otherWallet, previous: .known(1)),
            .known(1)
        )
        XCTAssertEqual(
            ShieldedSpendableCredits.applying(ready(credits: 5), startedFor: started, current: nil, previous: .known(1)),
            .known(1)
        )
    }

    func test_readScope_invalidatedRead_keepsThePreviousValue() {
        let manager = NSObject()
        let scope = ShieldedReadScope(manager: ObjectIdentifier(manager), walletId: walletId)
        XCTAssertEqual(
            ShieldedSpendableCredits.applying(nil, startedFor: scope, current: scope, previous: .known(1)),
            .known(1)
        )
    }

    // MARK: - Shielded selection recheck

    func test_selection_sourceGoneWhileSubmitting_isKept() {
        XCTAssertFalse(ShieldedSpendableCredits.known(0).dropsShieldedSelection(isToppingUp: true))
    }

    func test_selection_sourceGoneWhenSubmissionEnds_isDropped() {
        XCTAssertTrue(ShieldedSpendableCredits.known(0).dropsShieldedSelection(isToppingUp: false))
        XCTAssertTrue(ShieldedSpendableCredits.unknown(reason: nil).dropsShieldedSelection(isToppingUp: false))
    }

    func test_selection_sourceStillOfferedWhenSubmissionEnds_isKept() {
        XCTAssertFalse(ShieldedSpendableCredits.known(5).dropsShieldedSelection(isToppingUp: false))
    }

    // MARK: - Submit-time balance read

    private var request: ShieldedTopUpRequest {
        ShieldedTopUpRequest(walletId: walletId, identityId: identityId, amount: 1_000)
    }

    private func readyRead(_ credits: UInt64) -> ShieldedSpendableCredits.Read {
        ready(credits: credits)
    }

    func test_submitCheck_freshAffordableRead_sends() {
        XCTAssertEqual(
            ShieldedTopUpSubmitCheck.decide(request: request, minimumFee: 10, submitRead: readyRead(2_000)),
            .send
        )
    }

    func test_submitCheck_freshReadTooLow_refuses() {
        guard case .refuse(let message) = ShieldedTopUpSubmitCheck.decide(
            request: request, minimumFee: 10, submitRead: readyRead(1_009)
        ) else { return XCTFail("expected a refusal") }
        XCTAssertTrue(message.contains("Nothing was sent"))
    }

    func test_submitCheck_failedReadAfterAGoodDisplayRead_makesNoPayment() {
        // The screen showed an affordable balance earlier...
        let display = ShieldedSpendableCredits.after(readyRead(2_000), previous: .unknown(reason: nil))
        XCTAssertTrue(request.isAffordable(minimumFee: 10, spendable: display.credits))
        // ...but the submit read fails: no payment, whatever the display says.
        guard case .refuse(let message) = ShieldedTopUpSubmitCheck.decide(
            request: request, minimumFee: 10, submitRead: .failed("busy")
        ) else { return XCTFail("expected a refusal") }
        XCTAssertTrue(message.contains("Could not read the shielded balance"))
        XCTAssertTrue(message.contains("nothing was sent"))
    }

    func test_submitCheck_invalidatedReadAfterAGoodDisplayRead_makesNoPayment() {
        guard case .refuse(let message) = ShieldedTopUpSubmitCheck.decide(
            request: request, minimumFee: 10, submitRead: nil
        ) else { return XCTFail("expected a refusal") }
        XCTAssertTrue(message.contains("Could not read the shielded balance"))
    }

    func test_submitCheck_unboundOrIncompleteRead_makesNoPayment() {
        for read: ShieldedSpendableCredits.Read in [.state(.unbound), .state(.restoreIncomplete)] {
            guard case .refuse = ShieldedTopUpSubmitCheck.decide(
                request: request, minimumFee: 10, submitRead: read
            ) else { return XCTFail("expected a refusal for \(read)") }
        }
    }

    func test_submitCheck_unresolvedFee_makesNoPayment() {
        guard case .refuse = ShieldedTopUpSubmitCheck.decide(
            request: request, minimumFee: nil, submitRead: readyRead(2_000)
        ) else { return XCTFail("expected a refusal") }
    }

    // MARK: - Top-up refusals

    func test_topUpRefusal_protocolVersionNotActive_isReadable() {
        let message = ShieldedTopUpSubmitCheck.refusalMessage(.protocolVersionNotActive("x"))
        XCTAssertTrue(message?.contains("has not activated") == true)
        XCTAssertTrue(message?.contains("Nothing was sent") == true)
    }

    func test_topUpRefusal_sdkPinnedBelowVersion_isReadable() {
        let message = ShieldedTopUpSubmitCheck.refusalMessage(.sdkPinnedBelowProtocolVersion("x"))
        XCTAssertTrue(message?.contains("pinned below") == true)
        XCTAssertTrue(message?.contains("Nothing was sent") == true)
    }

    func test_topUpRefusal_otherError_usesItsOwnDescription() {
        XCTAssertNil(ShieldedTopUpSubmitCheck.refusalMessage(.walletOperation("x")))
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

    // MARK: - After a shielded Clear

    func test_afterClear_spendableIsUnknownAndNotOffered() {
        let spendable = ShieldedSpendableCredits.afterClear
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
        XCTAssertNotNil(spendable.unavailableReason)
    }

    func test_afterClear_failedReadKeepsItUnknown() {
        // The pre-Clear value is gone, so a failed read has nothing to keep.
        let spendable = ShieldedSpendableCredits.after(.failed("busy"), previous: .afterClear)
        XCTAssertNil(spendable.credits)
        XCTAssertFalse(spendable.offersShieldedSource)
    }

    func test_afterClear_aSuccessfulReadMakesItKnownAgain() {
        XCTAssertEqual(ShieldedSpendableCredits.after(ready(credits: 5), previous: .afterClear), .known(5))
    }

    func test_readCoordinator_readStartedBeforeAClear_neverAppliesAfterIt() throws {
        var reads = RecoveryReadCoordinator()
        let beforeClear = try XCTUnwrap(reads.request())
        // A follow-up was also requested before the Clear.
        XCTAssertNil(reads.request())
        reads.reset()
        // The pre-Clear read finishes after the reset: it does not apply and
        // does not start the old follow-up.
        let outcome = reads.finish(beforeClear)
        XCTAssertFalse(outcome.apply)
        XCTAssertNil(outcome.next)
        // The read started after the Clear applies.
        let afterClear = try XCTUnwrap(reads.request())
        XCTAssertTrue(reads.finish(afterClear).apply)
    }

    func test_readState_afterReset_isNotLoaded() {
        var state = RecoveryReadState<Int>()
        state.applySuccess([1])
        state = RecoveryReadState<Int>()
        XCTAssertEqual(state.status, .notLoaded)
        XCTAssertTrue(state.records.isEmpty)
        XCTAssertFalse(state.isKnownEmpty)
    }

    // MARK: - Recovery record labels

    func test_recoveryDisplay_offersStopRetryingOnlyForActiveRecords() {
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.retrying))
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.parked))
        XCTAssertFalse(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.unknown))
    }

    func test_recoveryDisplay_unknownStatus_neverReadsAsFailed() {
        for identityId in [identityId, nil] as [Data?] {
            let title = ShieldedIdentityDebitRecoveryDisplay.statusTitle(.unknown, identityId: identityId)
            let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.unknown, identityId: identityId)
            XCTAssertFalse(title.lowercased().contains("fail"))
            XCTAssertFalse(detail.lowercased().contains("fail"))
        }
    }

    func test_recoveryDisplay_stopWarning_statesTheRisks() {
        let warning = ShieldedIdentityDebitRecoveryDisplay.stopRetryingWarning(identityId: identityId)
        XCTAssertTrue(warning.contains("does not cancel"))
        XCTAssertTrue(warning.contains("may already have executed"))
        XCTAssertTrue(warning.contains("additional debit"))
    }

    // MARK: - Readable record labels (unchanged)

    func test_readableRecord_retrying_promisesResendOnSync() {
        XCTAssertEqual(
            ShieldedIdentityDebitRecoveryDisplay.statusTitle(.retrying, identityId: identityId),
            "Retrying"
        )
        XCTAssertTrue(
            ShieldedIdentityDebitRecoveryDisplay.statusDetail(.retrying, identityId: identityId)
                .contains("resends this signed shield on each shielded sync")
        )
    }

    func test_readableRecord_parkedAndUnknown_keepTheirTitles() {
        XCTAssertEqual(
            ShieldedIdentityDebitRecoveryDisplay.statusTitle(.parked, identityId: identityId),
            "Waiting for the note"
        )
        XCTAssertEqual(
            ShieldedIdentityDebitRecoveryDisplay.statusTitle(.unknown, identityId: identityId),
            "Outcome unknown"
        )
    }

    func test_readableRecord_notInTheWallet_showsAShortId() {
        let label = ShieldedIdentityDebitRecoveryDisplay.identityLabel(identityId: identityId, identities: [])
        XCTAssertEqual(label, "020202020202…")
    }

    // MARK: - Unreadable record labels

    func test_unreadableRecord_activeStatuses_makeNoResendPromise() {
        for status in [ShieldedIdentityDebitRecoveryStatus.retrying, .parked] {
            let title = ShieldedIdentityDebitRecoveryDisplay.statusTitle(status, identityId: nil)
            let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(status, identityId: nil)
            XCTAssertEqual(title, "Unreadable record")
            XCTAssertNotEqual(title, "Retrying")
            XCTAssertFalse(detail.contains("resends"))
            XCTAssertTrue(detail.contains("identity that funded this shield is unknown"))
            XCTAssertTrue(detail.contains("cannot be resent"))
            XCTAssertTrue(detail.contains("blocks new identity shields in this wallet"))
        }
    }

    func test_unreadableRecord_showsAnUnknownIdentity() {
        XCTAssertEqual(
            ShieldedIdentityDebitRecoveryDisplay.identityLabel(identityId: nil, identities: []),
            "Unknown identity"
        )
    }

    func test_unreadableRecord_canStillBeStopped() {
        // Stopping depends on the status only, not on whether the record reads.
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.retrying))
        XCTAssertTrue(ShieldedIdentityDebitRecoveryDisplay.canStopRetrying(.parked))
    }

    func test_unreadableRecord_stopWarning_keepsTheCancelWarningAndNamesNoIdentity() {
        let warning = ShieldedIdentityDebitRecoveryDisplay.stopRetryingWarning(identityId: nil)
        XCTAssertTrue(warning.contains("does not cancel"))
        XCTAssertTrue(warning.contains("may already have executed"))
        XCTAssertTrue(warning.contains("identity it belongs to is unknown"))
        XCTAssertTrue(warning.contains("additional debit"))
        XCTAssertFalse(warning.contains("this identity"))
    }

    // MARK: - Section footer

    /// The footer is the same for every set of records, so it must not make
    /// a promise that a stopped or unreadable record would contradict.
    private func assertFooterMakesNoStatusPromise(file: StaticString = #filePath, line: UInt = #line) {
        let footer = ShieldedIdentityDebitRecoveryDisplay.footerText
        XCTAssertFalse(footer.contains("resend"), file: file, line: line)
        XCTAssertFalse(footer.contains("blocks"), file: file, line: line)
        XCTAssertFalse(footer.contains("until you stop"), file: file, line: line)
        XCTAssertTrue(footer.contains("checks each shield for its note"), file: file, line: line)
        XCTAssertTrue(footer.contains("each row shows"), file: file, line: line)
    }

    func test_footer_withOneStoppedReadableRecord_promisesNoResend() {
        assertFooterMakesNoStatusPromise()
        // The row itself says retry was stopped, not that it is resent.
        let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.unknown, identityId: identityId)
        XCTAssertTrue(detail.contains("Automatic retry was stopped"))
        XCTAssertFalse(detail.contains("resends"))
    }

    func test_footer_withOneStoppedUnreadableRecord_claimsNoBlock() {
        assertFooterMakesNoStatusPromise()
        // A stopped unreadable record no longer blocks; its row does not say it does.
        let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.unknown, identityId: nil)
        XCTAssertFalse(detail.contains("blocks"))
        XCTAssertTrue(detail.contains("unknown"))
    }

    func test_footer_withOneActiveUnreadableRecord_leavesTheBlockToTheRow() {
        assertFooterMakesNoStatusPromise()
        // The active unreadable row carries the block and no-resend specifics.
        let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.retrying, identityId: nil)
        XCTAssertTrue(detail.contains("blocks new identity shields in this wallet"))
        XCTAssertTrue(detail.contains("cannot be resent"))
    }

    func test_footer_withOneParkedReadableRecord_promisesNoResend() {
        assertFooterMakesNoStatusPromise()
        let detail = ShieldedIdentityDebitRecoveryDisplay.statusDetail(.parked, identityId: identityId)
        XCTAssertFalse(detail.contains("resends"))
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

    func test_readCoordinator_requestsWhileInFlight_coalesceIntoOneFollowUp() throws {
        var reads = RecoveryReadCoordinator()
        let first = try XCTUnwrap(reads.request())
        // Two more requests while the first is in flight start nothing.
        XCTAssertNil(reads.request())
        XCTAssertNil(reads.request())
        // The first finishes and applies; exactly one follow-up starts.
        let outcome = reads.finish(first)
        XCTAssertTrue(outcome.apply)
        let followUp = try XCTUnwrap(outcome.next)
        XCTAssertGreaterThan(followUp, first)
        // The follow-up finishes with nothing more to run.
        let last = reads.finish(followUp)
        XCTAssertTrue(last.apply)
        XCTAssertNil(last.next)
        XCTAssertNil(reads.inFlight)
    }

    func test_readCoordinator_completionsInReverseOrder_onlyNewestApplies() throws {
        var reads = RecoveryReadCoordinator()
        // A read is in flight when Stop Retrying starts a superseding one.
        let before = try XCTUnwrap(reads.request())
        let after = reads.startSuperseding()
        // The newer read finishes first and applies.
        XCTAssertTrue(reads.finish(after).apply)
        // The older read finishes last and must not overwrite it.
        let late = reads.finish(before)
        XCTAssertFalse(late.apply)
        XCTAssertNil(late.next)
    }

    func test_readCoordinator_olderReadFinishingFirst_doesNotApplyOrEndTheNewer() throws {
        var reads = RecoveryReadCoordinator()
        let before = try XCTUnwrap(reads.request())
        let after = reads.startSuperseding()
        XCTAssertFalse(reads.finish(before).apply)
        // The superseding read is still the one in flight.
        XCTAssertEqual(reads.inFlight, after)
        XCTAssertNil(reads.request())
        XCTAssertTrue(reads.finish(after).apply)
    }

    func test_readCoordinator_reset_discardsReadsOfThePreviousWallet() throws {
        var reads = RecoveryReadCoordinator()
        let oldWalletRead = try XCTUnwrap(reads.request())
        reads.reset()
        // A read for the new wallet starts at once.
        let newWalletRead = try XCTUnwrap(reads.request())
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
