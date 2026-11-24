import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how the shielded tip sheet reads the wallet's answers. The wallet
/// parses the amount, reviews the recipient and decides each refusal; these
/// tests drive `ShieldedTipFlow` with closures in place of the native calls
/// and check what it shows and offers: a tip that may have gone through (18,
/// 20) is never offered again, an earlier tip still pending (84) offers a
/// shielded sync and another review, every other error is "not sent" with
/// the wallet's message, an unresolved review keeps Send off, and the
/// paying identity's balance shows the wallet's state.
@MainActor
final class ShieldedTipFlowTests: XCTestCase {

    private let recipient = ShieldedTipRecipient(
        identityId: Data(repeating: 1, count: 32), address: Data(repeating: 2, count: 43))
    private let previous = ShieldedTipRecipient(
        identityId: Data(repeating: 3, count: 32), address: Data(repeating: 4, count: 43))

    // XCTest makes a fresh instance for each test, so these start empty.
    private var sent: [ShieldedTipReviewed] = []
    private var reviewedNames: [String] = []
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
        refusal: String = "the wallet refused this amount",
        review: ShieldedTipReview? = nil,
        reviewError: Error? = nil,
        sendError: Error? = nil
    ) -> ShieldedTipFlow {
        let reviewResult = review ?? self.review()
        return ShieldedTipFlow(
            username: username,
            actions: ShieldedTipFlow.Actions(
                parseAmount: { [unowned self] text in
                    parsedTexts.append(text)
                    if text == "1" { return 100_000_000_000 }
                    throw PlatformWalletError.invalidParameter(refusal)
                },
                review: { [unowned self] name in
                    reviewedNames.append(name)
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
        XCTAssertFalse(flow.offersReviewAgain)
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

    func testShouldOfferASyncAndAnotherReviewAfterAnEarlierTipIsPending() async {
        let error = PlatformWalletError.shieldedTipPending("an earlier payment is unresolved")
        let flow = makeFlow(sendError: error)
        await reviewAndSend(flow)

        XCTAssertEqual(flow.phase, .finished(.earlierTipPending(error.localizedDescription)))
        XCTAssertTrue(flow.offersShieldedSync)
        XCTAssertTrue(flow.offersReviewAgain)
        XCTAssertFalse(flow.canSend)
        XCTAssertFalse(flow.inputsLocked)
        XCTAssertTrue(flow.canReview)
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
        XCTAssertFalse(flow.offersReviewAgain)
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
            "This wallet has not tipped this name before.")
    }

    func testShouldDescribeAnUnchangedRecipientWithoutAWarning() {
        XCTAssertFalse(ShieldedTipPresentation.changeIsWarning(.unchanged))
        XCTAssertNil(ShieldedTipPresentation.previousRecipient(.unchanged))
        XCTAssertEqual(
            ShieldedTipPresentation.changeText(.unchanged),
            "Same recipient as the last tip from this wallet to this name.")
    }

    func testShouldWarnOfAChangedRecipientAndShowThePreviousOne() async {
        let change = ShieldedTipRecipientChange.changed(previous: previous)
        XCTAssertTrue(ShieldedTipPresentation.changeIsWarning(change))
        XCTAssertEqual(ShieldedTipPresentation.previousRecipient(change), previous)

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
        XCTAssertTrue(flow.offersReviewAgain)

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

    func testShouldDropAReviewThatFinishesAfterTheInputsChanged() async {
        let gate = ReviewGate()
        let flow = ShieldedTipFlow(
            username: "alice",
            actions: ShieldedTipFlow.Actions(
                parseAmount: { _ in 1 },
                review: { [recipient, gate] _ in
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

        state.record(.unverified)
        XCTAssertTrue(state.offersPrepare)
        XCTAssertTrue(state.text.hasPrefix("Not verified yet."))

        state.record(.unavailable(account: 3))
        XCTAssertFalse(state.offersPrepare)
        XCTAssertTrue(state.text.contains("Not known yet (account 3)"))
        XCTAssertTrue(state.text.contains("Not known is not zero."))

        state.record(.available(account: 3, credits: 150_000_000_000))
        XCTAssertFalse(state.offersPrepare)
        XCTAssertEqual(state.text, "\(formatCreditsAsDash(150_000_000_000)) (account 3)")
        XCTAssertFalse(state.isFailure)
    }

    func testShouldKeepTheLastBalanceWhileTheStoreIsBusy() {
        var state = ShieldedTipBalanceState()
        state.record(.available(account: 1, credits: 5))
        state.record(error: PlatformWalletError.walletOperation("the shielded store is busy"))

        XCTAssertEqual(state.shown, .balance(.available(account: 1, credits: 5)))
        XCTAssertTrue(state.readsAgain)

        state.record(.available(account: 1, credits: 7))
        XCTAssertFalse(state.readsAgain)
        XCTAssertEqual(state.shown, .balance(.available(account: 1, credits: 7)))
    }

    func testShouldShowOtherBalanceErrorsWithTheirMessage() {
        var state = ShieldedTipBalanceState()
        let error = PlatformWalletError.identityNotFound("no such identity in this wallet")
        state.record(error: error)

        XCTAssertEqual(state.shown, .failed(error.localizedDescription))
        XCTAssertTrue(state.isFailure)
        XCTAssertFalse(state.readsAgain)
    }
}

/// Holds a review open until the test resumes it.
@MainActor
private final class ReviewGate {
    var pending: CheckedContinuation<Void, Never>?
}
