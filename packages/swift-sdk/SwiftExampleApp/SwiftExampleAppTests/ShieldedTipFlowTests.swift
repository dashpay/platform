import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how the shielded tip sheet reads the wallet's answers. The wallet
/// parses the amount, reviews the recipient and decides each refusal; these
/// tests drive `ShieldedTipFlow` with closures in place of the native calls
/// and check what it shows and offers: a tip that may have gone through (18,
/// 20) or that met an unresolved earlier payment (85) is never offered
/// again, and only a shielded sync is offered after it; every other error is
/// "not sent" with the wallet's message; an unresolved review keeps Send off
/// and steers nobody to send again; a tip from a contact passes the
/// contact's identity id to the review and the send, and a name that moved
/// to another identity (86) is explained; and the paying identity's balance shows
/// the wallet's state, keeps the last one through a busy store and reads
/// again, shows every other read error (`walletOperation` included) without
/// a retry, and ignores a read that finishes after a newer one.
@MainActor
final class ShieldedTipFlowTests: XCTestCase {

    private let recipient = ShieldedTipRecipient(
        identityId: Data(repeating: 1, count: 32), address: Data(repeating: 2, count: 43))
    private let previous = ShieldedTipRecipient(
        identityId: Data(repeating: 3, count: 32), address: Data(repeating: 4, count: 43))

    // XCTest makes a fresh instance for each test, so these start empty.
    private var sent: [ShieldedTipReviewed] = []
    private var reviewedNames: [String] = []
    private var reviewedContacts: [Data?] = []
    private var parsedTexts: [String] = []

    // MARK: - Helpers

    private func review(
        _ change: ShieldedTipRecipientChange = .new, unresolved: Bool = false
    ) -> ShieldedTipReview {
        ShieldedTipReview(recipient: recipient, change: change, unresolved: unresolved)
    }

    /// A flow whose parser accepts "1" as 100 billion credits and refuses
    /// anything else with `refusal`.
    private func makeFlow(
        username: String? = "alice",
        contactIdentityId: Data? = nil,
        refusal: String = "the wallet refused this amount",
        review: ShieldedTipReview? = nil,
        reviewError: Error? = nil,
        sendError: Error? = nil
    ) -> ShieldedTipFlow {
        let reviewResult = review ?? self.review()
        return ShieldedTipFlow(
            username: username,
            contactIdentityId: contactIdentityId,
            actions: ShieldedTipFlow.Actions(
                parseAmount: { [unowned self] text in
                    parsedTexts.append(text)
                    if text == "1" { return 100_000_000_000 }
                    throw PlatformWalletError.invalidParameter(refusal)
                },
                review: { [unowned self] name, contactId in
                    reviewedNames.append(name)
                    reviewedContacts.append(contactId)
                    if let reviewError { throw reviewError }
                    return reviewResult
                },
                send: { [unowned self] tip in
                    sent.append(tip)
                    if let sendError { throw sendError }
                }
            )
        )
    }

    /// Review a one-DASH tip and send it.
    private func reviewAndSend(_ flow: ShieldedTipFlow) async {
        flow.amountText = "1"
        await flow.review()
        await flow.send()
    }

    // MARK: - Send outcomes

    func testShouldNeverOfferAResendAfterAnUnconfirmedShieldedSpend() async {
        let flow = makeFlow(sendError: PlatformWalletError.shieldedSpendUnconfirmed("not proved"))
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.mayHaveGoneThrough(
            PlatformWalletError.shieldedSpendUnconfirmed("not proved").localizedDescription)))
        XCTAssertFalse(flow.canSend)
        XCTAssertFalse(flow.canReview)
        XCTAssertFalse(flow.offersTipAgain)
        XCTAssertTrue(flow.inputsLocked)
        XCTAssertTrue(flow.offersShieldedSync)
    }

    func testShouldNeverOfferAResendAfterAnUnconfirmedBroadcast() async {
        let error = PlatformWalletError.transactionBroadcastUnconfirmed("not confirmed")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.mayHaveGoneThrough(error.localizedDescription)))
        XCTAssertFalse(flow.canSend)
        XCTAssertFalse(flow.canReview)
        XCTAssertTrue(flow.inputsLocked)
    }

    func testShouldKeepTheInputsOfATipThatMayHaveGoneThrough() async {
        let flow = makeFlow(sendError: PlatformWalletError.shieldedSpendUnconfirmed("not proved"))
        await reviewAndSend(flow)
        flow.amountText = "2"

        XCTAssertEqual(flow.phase, .finished(.mayHaveGoneThrough(
            PlatformWalletError.shieldedSpendUnconfirmed("not proved").localizedDescription)))
    }

    func testShouldOfferOnlyASyncAfterAnEarlierTipIsPending() async {
        let error = PlatformWalletError.shieldedTipPending("an earlier payment is unresolved")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.earlierTipPending(error.localizedDescription)))
        XCTAssertTrue(flow.offersShieldedSync)
        XCTAssertFalse(flow.canSend)
        XCTAssertFalse(flow.canReview)
        XCTAssertFalse(flow.offersTipAgain)
        XCTAssertTrue(flow.inputsLocked)
    }

    func testShouldKeepTheInputsAndOfferNoReviewAfterAnEarlierTipIsPending() async {
        let error = PlatformWalletError.shieldedTipPending("an earlier payment is unresolved")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)
        flow.amountText = "2"
        flow.memo = "again"
        await flow.review()
        await flow.send()

        XCTAssertEqual(flow.phase, .finished(.earlierTipPending(error.localizedDescription)))
        XCTAssertEqual(reviewedNames.count, 1)
        XCTAssertEqual(sent.count, 1)
    }

    func testShouldTellThatAPendingTipMayStillPayTheRecipient() {
        let outcome = ShieldedTipSendOutcome.earlierTipPending("refused")
        XCTAssertEqual(
            outcome.message,
            "Nothing was sent by this attempt. An earlier payment to this recipient has not "
                + "resolved and may pay them. Check shielded activity before tipping this "
                + "recipient again; the wallet stops refusing once a sync resolves it, whether "
                + "or not it paid.")
        XCTAssertFalse(outcome.message.localizedCaseInsensitiveContains("review again"))
        XCTAssertFalse(ShieldedTipPresentation.unresolvedMessage
            .localizedCaseInsensitiveContains("review again"))
        XCTAssertTrue(ShieldedTipPresentation.unresolvedMessage
            .hasSuffix(ShieldedTipPresentation.unresolvedCaution))
    }

    func testShouldShowAMissingIdentityAsNotSent() async {
        let error = PlatformWalletError.identityNotFound("no such identity in this wallet")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.notSent(error.localizedDescription)))
        XCTAssertFalse(flow.offersShieldedSync)
        XCTAssertTrue(flow.canReview)
    }

    func testShouldShowARecipientThatChangedSinceTheReviewAsNotSent() async {
        let error = PlatformWalletError.walletOperation("the recipient changed since the review")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.notSent(error.localizedDescription)))
        XCTAssertEqual(ShieldedTipSendOutcome.of(error).detail, error.localizedDescription)
        XCTAssertFalse(flow.offersShieldedSync)
        XCTAssertFalse(flow.inputsLocked)
        XCTAssertTrue(flow.canReview)
    }

    func testShouldShowAnyOtherErrorAsNotSentWithItsMessage() {
        let memoRefused = PlatformWalletError.invalidParameter("memo is longer than 32 bytes")
        XCTAssertEqual(ShieldedTipSendOutcome.of(memoRefused), .notSent(memoRefused.localizedDescription))

        struct Other: LocalizedError {
            var errorDescription: String? { "something else" }
        }
        XCTAssertEqual(ShieldedTipSendOutcome.of(Other()), .notSent("something else"))
    }

    func testShouldPassTheContactsIdentityIdToTheReviewAndTheSend() async {
        let contactId = Data(repeating: 9, count: 32)
        let flow = makeFlow(username: "bob", contactIdentityId: contactId)
        await reviewAndSend(flow)

        XCTAssertEqual(reviewedNames, ["bob"])
        XCTAssertEqual(reviewedContacts, [contactId])
        XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent.first?.contactIdentityId, contactId)
        XCTAssertEqual(sent.first?.username, "bob")
    }

    func testShouldPassNoContactForATypedUsername() async {
        let flow = makeFlow(username: nil)
        flow.username = "carol"
        await reviewAndSend(flow)

        XCTAssertEqual(reviewedContacts, [nil])
        XCTAssertEqual(sent.count, 1)
        XCTAssertNil(sent.first?.contactIdentityId)
    }

    func testShouldExplainANameThatMovedAwayFromTheContactWhenSending() async {
        let error = PlatformWalletError.shieldedTipRecipientMismatch("the name belongs to another identity")
        let flow = makeFlow(contactIdentityId: Data(repeating: 9, count: 32), sendError: error)
        await reviewAndSend(flow)

        guard case .finished(let outcome) = flow.phase else {
            return XCTFail("expected a finished send, got \(flow.phase)")
        }
        XCTAssertEqual(outcome, .recipientMismatch(error.localizedDescription))
        XCTAssertEqual(outcome.identifier, "dashpay.tip.outcome.recipientMismatch")
        XCTAssertEqual(outcome.detail, error.localizedDescription)
        XCTAssertEqual(
            outcome.message,
            "Nothing was sent. This name now belongs to another identity, not this contact. "
                + "To tip the name's new owner, start from the name with Tip a username on "
                + "your profile, not from this contact.")
        XCTAssertFalse(flow.offersShieldedSync)
        XCTAssertFalse(flow.offersTipAgain)
        XCTAssertFalse(flow.canSend)
    }

    func testShouldExplainANameThatMovedAwayFromTheContactWhenReviewing() async {
        let error = PlatformWalletError.shieldedTipRecipientMismatch("the name belongs to another identity")
        let flow = makeFlow(contactIdentityId: Data(repeating: 9, count: 32), reviewError: error)
        flow.amountText = "1"
        await flow.review()

        XCTAssertEqual(flow.phase, .reviewFailed(
            ShieldedTipPresentation.recipientMismatchMessage + " " + error.localizedDescription))
        XCTAssertTrue(ShieldedTipPresentation.recipientMismatchMessage
            .contains("now belongs to another identity, not this contact"))
        XCTAssertTrue(ShieldedTipPresentation.recipientMismatchMessage
            .contains("start from the name"))
        XCTAssertFalse(flow.canSend)
        XCTAssertTrue(sent.isEmpty)
    }

    func testShouldSendTheReviewedRecipientAmountAndMemoFromTheReview() async {
        let flow = makeFlow()
        flow.amountText = "1"
        flow.memo = "thanks"
        await flow.review()
        await flow.send()

        XCTAssertEqual(flow.phase, .finished(.submitted))
        XCTAssertEqual(sent, [ShieldedTipReviewed(
            username: "alice", amount: 100_000_000_000, memo: "thanks", review: review())])
        XCTAssertTrue(flow.inputsLocked)
        XCTAssertTrue(flow.offersTipAgain)
        XCTAssertFalse(flow.offersShieldedSync)
    }

    func testShouldPassAnEmptyMemoAsNoMemo() async {
        let flow = makeFlow()
        await reviewAndSend(flow)

        XCTAssertEqual(sent.count, 1)
        XCTAssertNil(sent.first?.memo)
    }

    func testShouldStartAnotherTipOnlyAfterOneWasSent() async {
        let flow = makeFlow()
        await reviewAndSend(flow)
        flow.tipAgain()

        XCTAssertEqual(flow.phase, .editing)
        XCTAssertEqual(flow.amountText, "")
        XCTAssertEqual(flow.amount, .empty)
        XCTAssertEqual(flow.username, "alice")
    }

    // MARK: - Review

    func testShouldDescribeANewRecipientWithoutAWarning() {
        XCTAssertFalse(ShieldedTipPresentation.changeIsWarning(.new))
        XCTAssertNil(ShieldedTipPresentation.previousRecipient(.new))
        XCTAssertEqual(
            ShieldedTipPresentation.changeText(.new),
            "No recipient was confirmed for this name from this wallet before.")
    }

    func testShouldDescribeAnUnchangedRecipientWithoutAWarning() {
        XCTAssertFalse(ShieldedTipPresentation.changeIsWarning(.unchanged))
        XCTAssertNil(ShieldedTipPresentation.previousRecipient(.unchanged))
        XCTAssertEqual(
            ShieldedTipPresentation.changeText(.unchanged),
            "Same recipient as the one last confirmed for this name from this wallet.")
    }

    func testShouldWarnOfAChangedRecipientAndShowThePreviousOne() async {
        let change = ShieldedTipRecipientChange.changed(previous: previous)
        XCTAssertTrue(ShieldedTipPresentation.changeIsWarning(change))
        XCTAssertEqual(ShieldedTipPresentation.previousRecipient(change), previous)
        XCTAssertEqual(
            ShieldedTipPresentation.changeText(change),
            "This name now pays a different identity or shielded address than the one last "
                + "confirmed for it from this wallet. Check with the recipient before sending.")

        // A changed recipient is a warning, not a refusal: the wallet sends
        // to the recipient the user reviewed.
        let flow = makeFlow(review: review(change))
        flow.amountText = "1"
        await flow.review()
        XCTAssertTrue(flow.canSend)
        await flow.send()
        XCTAssertEqual(sent.first?.review.recipient, recipient)
    }

    func testShouldKeepSendOffWhileTheReviewIsUnresolved() async {
        let flow = makeFlow(review: review(unresolved: true))
        flow.amountText = "1"
        await flow.review()

        XCTAssertNotNil(flow.reviewed)
        XCTAssertFalse(flow.canSend)
        XCTAssertTrue(flow.offersShieldedSync)
        XCTAssertFalse(flow.offersTipAgain)

        await flow.send()
        XCTAssertTrue(sent.isEmpty)
        XCTAssertEqual(flow.phase, .reviewed(ShieldedTipReviewed(
            username: "alice", amount: 100_000_000_000, memo: nil, review: review(unresolved: true))))
    }

    func testShouldShowTheWalletsReasonWhenAReviewFails() async {
        let error = PlatformWalletError.walletOperation("The username 'bob' was not found")
        let flow = makeFlow(username: nil, reviewError: error)
        flow.username = "bob"
        flow.amountText = "1"
        await flow.review()

        XCTAssertEqual(reviewedNames, ["bob"])
        XCTAssertEqual(flow.phase, .reviewFailed(error.localizedDescription))
        XCTAssertFalse(flow.canSend)
        XCTAssertTrue(flow.canReview)
    }

    func testShouldPassTheUsernameToTheWalletAsTyped() async {
        let flow = makeFlow(username: nil)
        flow.username = " Alice.dash "
        flow.amountText = "1"
        await flow.review()

        XCTAssertEqual(reviewedNames, [" Alice.dash "])
    }

    func testShouldNotReviewWithoutAUsernameOrAnAcceptedAmount() async {
        let flow = makeFlow(username: nil)
        flow.amountText = "1"
        XCTAssertFalse(flow.canReview)

        flow.username = "alice"
        flow.amountText = "abc"
        XCTAssertFalse(flow.canReview)
        await flow.review()
        XCTAssertTrue(reviewedNames.isEmpty)
    }

    func testShouldDropTheReviewWhenTheAmountOrMemoChanges() async {
        let flow = makeFlow()
        flow.amountText = "1"
        await flow.review()
        XCTAssertNotNil(flow.reviewed)

        flow.memo = "hi"
        XCTAssertEqual(flow.phase, .editing)
        XCTAssertFalse(flow.canSend)

        await flow.review()
        flow.amountText = "abc"
        XCTAssertEqual(flow.phase, .editing)
    }

    func testShouldNotStartASecondReviewOrSendWhileOneRuns() async {
        let reviewGate = ReviewGate()
        let sendGate = ReviewGate()
        let flow = ShieldedTipFlow(
            username: "alice",
            actions: ShieldedTipFlow.Actions(
                parseAmount: { _ in 1 },
                review: { [recipient, reviewGate] _, _ in
                    reviewGate.calls += 1
                    await withCheckedContinuation { reviewGate.pending = $0 }
                    return ShieldedTipReview(recipient: recipient, change: .new, unresolved: false)
                },
                send: { [sendGate] _ in
                    sendGate.calls += 1
                    await withCheckedContinuation { sendGate.pending = $0 }
                }
            )
        )
        flow.amountText = "1"

        // While the review runs: no second review, no send, and the sheet
        // cannot be closed (`isBusy` disables Done and swipe-to-dismiss).
        let reviewing = Task { await flow.review() }
        while reviewGate.pending == nil { await Task.yield() }
        XCTAssertTrue(flow.isBusy)
        XCTAssertFalse(flow.canReview)
        XCTAssertFalse(flow.canSend)
        await flow.review()
        await flow.send()
        XCTAssertEqual(reviewGate.calls, 1)
        XCTAssertEqual(sendGate.calls, 0)
        reviewGate.pending?.resume()
        await reviewing.value
        XCTAssertFalse(flow.isBusy)
        XCTAssertTrue(flow.canSend)

        // While the send runs: inputs locked, nothing can be reviewed or
        // sent again, and the sheet cannot be closed.
        let sending = Task { await flow.send() }
        while sendGate.pending == nil { await Task.yield() }
        XCTAssertTrue(flow.isBusy)
        XCTAssertTrue(flow.inputsLocked)
        XCTAssertFalse(flow.canReview)
        XCTAssertFalse(flow.canSend)
        await flow.send()
        await flow.review()
        flow.amountText = "2"
        XCTAssertEqual(sendGate.calls, 1)
        XCTAssertEqual(reviewGate.calls, 1)
        sendGate.pending?.resume()
        await sending.value

        XCTAssertFalse(flow.isBusy)
        XCTAssertEqual(flow.phase, .finished(.submitted))
        XCTAssertEqual(sendGate.calls, 1)
    }

    func testShouldDropAReviewThatFinishesAfterTheInputsChanged() async {
        let gate = ReviewGate()
        let flow = ShieldedTipFlow(
            username: "alice",
            actions: ShieldedTipFlow.Actions(
                parseAmount: { _ in 1 },
                review: { [recipient, gate] _, _ in
                    await withCheckedContinuation { gate.pending = $0 }
                    return ShieldedTipReview(recipient: recipient, change: .new, unresolved: false)
                },
                send: { _ in }
            )
        )
        flow.amountText = "1"
        let reviewing = Task { await flow.review() }
        while gate.pending == nil { await Task.yield() }
        XCTAssertEqual(flow.phase, .reviewing)

        flow.amountText = "2"
        gate.pending?.resume()
        await reviewing.value

        XCTAssertEqual(flow.phase, .editing)
        XCTAssertNil(flow.reviewed)
    }

    // MARK: - Amount

    func testShouldShowTheWalletsReasonForARefusedAmount() {
        let flow = makeFlow(refusal: "amount has digits finer than one credit")
        flow.amountText = "0.000000000001"

        XCTAssertEqual(flow.amount, .refused(
            PlatformWalletError.invalidParameter("amount has digits finer than one credit").localizedDescription))
        XCTAssertFalse(flow.canReview)
    }

    func testShouldShowTheAmountTheWalletParsed() {
        let flow = makeFlow()
        flow.amountText = "1"

        XCTAssertEqual(flow.amount, .parsed(100_000_000_000))
        XCTAssertEqual(parsedTexts, ["1"])
    }

    func testShouldNotAskTheWalletToParseAnEmptyAmount() {
        let flow = makeFlow()
        flow.amountText = "1"
        flow.amountText = ""

        XCTAssertEqual(flow.amount, .empty)
        XCTAssertEqual(parsedTexts, ["1"])
    }

    // MARK: - Paying identity's balance

    func testShouldShowEachIdentityBalanceState() {
        var state = ShieldedTipBalanceState()
        XCTAssertEqual(state.shown, .notRead)

        state.apply(.success(.unverified), for: state.beginRead())
        XCTAssertTrue(state.offersPrepare)
        XCTAssertTrue(state.text.hasPrefix("Not verified yet."))

        state.apply(.success(.unavailable(account: 3)), for: state.beginRead())
        XCTAssertFalse(state.offersPrepare)
        XCTAssertTrue(state.text.contains("Not known yet (account 3)"))
        XCTAssertTrue(state.text.contains("Not known is not zero."))

        state.apply(.success(.available(account: 3, credits: 150_000_000_000)), for: state.beginRead())
        XCTAssertFalse(state.offersPrepare)
        XCTAssertEqual(state.text, "\(formatCreditsAsDash(150_000_000_000)) (account 3)")
        XCTAssertFalse(state.isFailure)
        XCTAssertNil(state.busyText)
        XCTAssertNil(state.retryDelay)
    }

    func testShouldKeepTheLastBalanceAndReadAgainWhileTheStoreIsBusy() {
        var state = ShieldedTipBalanceState()
        state.apply(.success(.available(account: 1, credits: 5)), for: state.beginRead())
        state.apply(
            .failure(PlatformWalletError.shieldedBalanceBusy("a scan holds the store")),
            for: state.beginRead())

        XCTAssertEqual(state.shown, .balance(.available(account: 1, credits: 5)))
        XCTAssertEqual(state.text, "\(formatCreditsAsDash(5)) (account 1)")
        XCTAssertEqual(state.busyText, "The shielded store is busy. Reading again shortly.")
        XCTAssertEqual(state.retryDelay, .seconds(2))
        XCTAssertFalse(state.isFailure)

        state.apply(.success(.available(account: 1, credits: 7)), for: state.beginRead())
        XCTAssertNil(state.busyText)
        XCTAssertNil(state.retryDelay)
        XCTAssertEqual(state.shown, .balance(.available(account: 1, credits: 7)))
    }

    func testShouldShowAWalletOperationBalanceErrorAsAFailureWithoutRetrying() {
        var state = ShieldedTipBalanceState()
        state.apply(.success(.available(account: 1, credits: 5)), for: state.beginRead())
        let error = PlatformWalletError.walletOperation("unknown wallet")
        state.apply(.failure(error), for: state.beginRead())

        XCTAssertEqual(state.shown, .failed(error.localizedDescription))
        XCTAssertEqual(state.text, "Could not read the shielded balance: \(error.localizedDescription)")
        XCTAssertTrue(state.isFailure)
        XCTAssertNil(state.busyText)
        XCTAssertNil(state.retryDelay)
    }

    func testShouldShowOtherBalanceErrorsWithTheirMessage() {
        var state = ShieldedTipBalanceState()
        let error = PlatformWalletError.identityNotFound("no such identity in this wallet")
        state.apply(.failure(error), for: state.beginRead())

        XCTAssertEqual(state.shown, .failed(error.localizedDescription))
        XCTAssertTrue(state.isFailure)
        XCTAssertNil(state.busyText)
        XCTAssertNil(state.retryDelay)
    }

    func testShouldKeepPrepareOffAfterASuccessfulPrepareUntilAReadThatIsNotBusy() {
        var state = ShieldedTipBalanceState()
        let beforePrepare = state.beginRead()
        state.apply(.success(.unverified), for: beforePrepare)
        XCTAssertTrue(state.offersPrepare)

        let inFlight = state.beginRead()
        state.prepareSucceeded()
        XCTAssertFalse(state.offersPrepare)

        // A read started before the prepare no longer applies.
        XCTAssertFalse(state.apply(.success(.unverified), for: inFlight))
        XCTAssertFalse(state.offersPrepare)

        // A busy read keeps the old `.unverified` and keeps Prepare off.
        state.apply(.failure(PlatformWalletError.shieldedBalanceBusy("busy")), for: state.beginRead())
        XCTAssertEqual(state.shown, .balance(.unverified))
        XCTAssertFalse(state.offersPrepare)

        // A read that is not busy decides: still unverified offers it again.
        state.apply(.success(.unverified), for: state.beginRead())
        XCTAssertTrue(state.offersPrepare)

        state.prepareSucceeded()
        state.apply(.success(.unavailable(account: 4)), for: state.beginRead())
        XCTAssertFalse(state.offersPrepare)
        XCTAssertFalse(state.awaitingReadAfterPrepare)
    }

    func testShouldIgnoreABalanceReadThatFinishesAfterANewerOne() {
        var state = ShieldedTipBalanceState()
        let older = state.beginRead()
        let newer = state.beginRead()

        XCTAssertTrue(state.apply(.success(.available(account: 2, credits: 9)), for: newer))
        XCTAssertFalse(state.apply(.success(.unverified), for: older))
        XCTAssertFalse(state.apply(
            .failure(PlatformWalletError.shieldedBalanceBusy("busy")), for: older))

        XCTAssertEqual(state.shown, .balance(.available(account: 2, credits: 9)))
        XCTAssertNil(state.busyText)
        XCTAssertNil(state.retryDelay)
    }
}

/// Holds a review or a send open until the test resumes it.
@MainActor
private final class ReviewGate {
    var pending: CheckedContinuation<Void, Never>?
    /// How many times the gated action was called.
    var calls = 0
}
