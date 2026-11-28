import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// `PaymentFailure` is the app's one mapping from Rust's payment class to
/// the text and action a failed payment screen shows. These pin the mapping
/// and that each payment flow's failed state is built from it and restarts
/// only as it allows.
final class PaymentFailureTests: XCTestCase {
    private final class Flag: @unchecked Sendable { var value = false }

    private static let held = AssetLockBuiltOutPoint(
        rawBytes: Data(repeating: 7, count: 32) + Data([1, 0, 0, 0])
    )!
    private static let other = AssetLockBuiltOutPoint(
        rawBytes: Data(repeating: 9, count: 32) + Data([0, 0, 0, 0])
    )!
    private static let retryable = PlatformPaymentClass(delivery: .notSent, isRetryable: true)
    private static let blocked = PlatformPaymentClass(delivery: .notSent, isRetryable: false)
    private static let lockHeld = PlatformPaymentClass(delivery: .lockHeld(held), isRetryable: false)

    func testShouldMapEachClassToOneActionAndText() {
        let cases: [(PlatformPaymentClass?, PaymentFailure.Action, PaymentFailure.Recovery)] = [
            (Self.retryable, .retry, .retry),
            (Self.blocked, .wait, .wait),
            (Self.lockHeld, .resume(Self.held), .resume(Self.held)),
            (.maybeSent, .wait, .reconcile),
            (nil, .wait, .unconfirmed),
        ]
        for (payment, action, recovery) in cases {
            let failure = PaymentFailure(payment: payment, detail: "detail")
            XCTAssertEqual(failure.action, action, "\(String(describing: payment))")
            XCTAssertTrue(failure.message.hasPrefix(recovery.suggestion), failure.message)
            XCTAssertTrue(failure.message.hasSuffix("detail"), failure.message)
            if action != .retry {
                XCTAssertFalse(failure.message.lowercased().contains("try again"), failure.message)
            }
        }
        let resume = PaymentFailure(payment: Self.lockHeld, detail: "d")
        XCTAssertEqual(resume.heldLock, Self.held)
        XCTAssertTrue(resume.message.contains(Self.held.outPointHex))
    }

    /// Every class maps to one recovery and one displayed suggestion: a retry
    /// only for a retryable `.notSent`, a wait for a non-retryable one, a
    /// resume naming the held lock, a reconcile for `.maybeSent`, and an
    /// unconfirmed outcome (never a retry) when nothing classified it.
    func testShouldDecideTheRecoveryFromThePaymentClassAlone() {
        let cases: [(PlatformPaymentClass?, PaymentFailure.Recovery, String)] = [
            (Self.retryable, .retry, "Nothing was sent. You can try again."),
            (Self.blocked, .wait, "Nothing was sent. This request cannot go through as it is."),
            (Self.lockHeld, .resume(Self.held),
             "The funds are held in an asset lock. Resume that lock to finish; do not start a new payment."),
            (.maybeSent, .reconcile,
             "The payment may have gone through. Wait for the wallet to sync before doing anything else; "
                + "do not send it again."),
            (nil, .unconfirmed,
             "Submission result is unconfirmed. Check history and reconcile the original operation "
                + "before submitting again."),
        ]
        for (payment, recovery, text) in cases {
            XCTAssertEqual(PaymentFailure.Recovery(payment), recovery, "\(String(describing: payment))")
            XCTAssertEqual(PaymentFailure.Recovery(payment).suggestion, text)
        }
    }

    /// A held invitation voucher is reclaimed, not resumed; every other
    /// class reads as it always does (← Kotlin app `PaymentFailure`).
    func testShouldTellAnInvitationVoucherToBeReclaimed() {
        let resume = PaymentFailure.Recovery.resume(Self.held)
        XCTAssertTrue(resume.invitationVoucherSuggestion.contains("Reclaim the voucher"))
        XCTAssertFalse(resume.invitationVoucherSuggestion.contains("Resume"))
        for other in [PaymentFailure.Recovery.retry, .wait, .reconcile, .unconfirmed] {
            XCTAssertEqual(other.invitationVoucherSuggestion, other.suggestion)
        }
    }

    /// The class is read from the error itself, never from its case: a
    /// typically-retryable case without a class waits, and an
    /// "unconfirmed" case Rust classed retryable offers the retry.
    func testShouldReadTheClassFromTheError() {
        XCTAssertEqual(
            PaymentFailure(PlatformWalletError.shieldedBroadcastFailed("refused")).action,
            .wait
        )
        XCTAssertEqual(
            PaymentFailure(PlatformWalletError.walletOperation("x", payment: Self.retryable)).action,
            .retry
        )
        XCTAssertEqual(PaymentFailure(SDKError.timeout("timeout")).action, .wait)
    }

    /// Every SDK error is a payment failure, with or without Rust's class: a
    /// wallet error with no class (the FFI's `Unknown` delivery decodes to
    /// none) is unconfirmed and holds its slot. Only the screen's own input
    /// check (not an SDK error) shows as its own message (← Kotlin
    /// `PaymentFailure.ofPayment`).
    func testShouldPresentEverySdkErrorAsAPaymentFailure() {
        XCTAssertEqual(
            PaymentFailure.ofPayment(PlatformWalletError.walletOperation("lost", payment: .maybeSent)),
            PaymentFailure(payment: .maybeSent, detail: "lost")
        )
        XCTAssertEqual(PaymentFailure.ofPayment(SDKError.timeout("timeout"))?.action, .wait)
        let classless = PaymentFailure.ofPayment(PlatformWalletError.invalidParameter("Enter an amount"))
        XCTAssertEqual(classless?.recovery, .unconfirmed)
        XCTAssertEqual(classless?.holdsSlot, true)
        XCTAssertEqual(classless?.releasableAfterReconciliation, true)
        XCTAssertNil(PaymentFailure.ofPayment(CocoaError(.validationNumberTooSmall)))
    }

    /// A screen may start a fresh payment only when its last failure does
    /// not hold its slot: refused after `.maybeSent`, `.lockHeld` and an
    /// unclassified failure, allowed after any `.notSent` one and with none.
    func testShouldPermitAFreshPaymentOnlyAfterAFailureThatReleasesItsSlot() {
        let table: [(PlatformPaymentClass?, Bool)] = [
            (.maybeSent, false),
            (Self.lockHeld, false),
            (nil, false),
            (Self.retryable, true),
            (Self.blocked, true),
        ]
        for (payment, permits) in table {
            for voucher in [false, true] {
                let failure = PaymentFailure(payment: payment, detail: "", invitationVoucher: voucher)
                XCTAssertEqual(
                    PaymentFailure.permitsFreshPayment(after: failure), permits,
                    "\(String(describing: payment)) voucher: \(voucher)"
                )
            }
        }
        XCTAssertTrue(PaymentFailure.permitsFreshPayment(after: nil))
    }

    /// A screen keeps its failure across an error that is not an SDK error
    /// (it says nothing about the payment), and takes any SDK error's own,
    /// so a later `.notSent` result releases the hold (← Kotlin
    /// `PaymentFailure.keptAfter`).
    func testShouldKeepAHoldAcrossAnErrorThatIsNotAPaymentFailure() {
        let held = PaymentFailure(payment: .maybeSent, detail: "d")
        XCTAssertEqual(PaymentFailure.keptAfter(held, error: CocoaError(.validationNumberTooSmall)), held)
        XCTAssertNil(PaymentFailure.keptAfter(nil, error: CocoaError(.validationNumberTooSmall)))
        let notSent = PaymentFailure.keptAfter(
            held,
            error: PlatformWalletError.walletOperation("refused", payment: Self.retryable)
        )
        XCTAssertEqual(notSent?.recovery, .retry)
        XCTAssertTrue(PaymentFailure.permitsFreshPayment(after: notSent))
        let voucher = PaymentFailure.keptAfter(
            nil,
            error: PlatformWalletError.walletOperation("held", payment: Self.lockHeld),
            invitationVoucher: true
        )
        XCTAssertEqual(voucher?.invitationVoucher, true)
    }

    /// A failed invitation create holding its voucher lock tells the user to
    /// reclaim the voucher, not to resume the lock: the operation says it is
    /// a voucher, the class is unchanged.
    func testShouldReclaimAnInvitationVoucherInsteadOfResumingIt() {
        let voucher = PaymentFailure(payment: Self.lockHeld, detail: "d", invitationVoucher: true)
        XCTAssertEqual(voucher.action, .resume(Self.held))
        XCTAssertFalse(voucher.permitsRestart(resuming: nil), "never fund a new invitation")
        XCTAssertTrue(voucher.message.hasPrefix(PaymentFailure.Recovery.resume(Self.held).invitationVoucherSuggestion))
        XCTAssertTrue(voucher.message.contains("Reclaim the voucher"), voucher.message)
        XCTAssertFalse(voucher.message.contains("Resume that lock"), voucher.message)
        XCTAssertTrue(voucher.message.contains(Self.held.outPointHex))
        XCTAssertEqual(voucher.title, "Funds held in the invitation voucher")

        // Every other class reads the same for an invitation.
        for payment in [Self.retryable, Self.blocked, .maybeSent, nil] {
            XCTAssertEqual(
                PaymentFailure(payment: payment, detail: "d", invitationVoucher: true).message,
                PaymentFailure(payment: payment, detail: "d").message
            )
        }
    }

    /// A failure that may have left (`.maybeSent`), holds a lock
    /// (`.lockHeld`) or has no class holds its slot; any `.notSent` one,
    /// retryable or not, releases it.
    func testShouldHoldTheSlotUnlessNothingWasSent() {
        let table: [(PlatformPaymentClass?, Bool)] = [
            (Self.retryable, false),
            (Self.blocked, false),
            (Self.lockHeld, true),
            (.maybeSent, true),
            (nil, true),
        ]
        for (payment, holds) in table {
            let failure = PaymentFailure(payment: payment, detail: "")
            XCTAssertEqual(failure.holdsSlot, holds, "\(String(describing: payment))")
            XCTAssertEqual(
                PaymentFailure(payment: payment, detail: "", invitationVoucher: true).holdsSlot,
                holds
            )
        }
    }

    /// Only a hold without a lock to resume (`.maybeSent`, no class) is
    /// released after the user reconciled it; a held lock is resumed and a
    /// `.notSent` failure never held its slot.
    func testShouldReleaseAfterReconciliationOnlyAHoldWithoutALock() {
        let table: [(PlatformPaymentClass?, Bool)] = [
            (Self.retryable, false),
            (Self.blocked, false),
            (Self.lockHeld, false),
            (.maybeSent, true),
            (nil, true),
        ]
        for (payment, releasable) in table {
            XCTAssertEqual(
                PaymentFailure(payment: payment, detail: "").releasableAfterReconciliation,
                releasable,
                "\(String(describing: payment))"
            )
            XCTAssertEqual(
                PaymentFailure(payment: payment, detail: "", invitationVoucher: true)
                    .releasableAfterReconciliation,
                releasable
            )
        }
    }

    func testShouldRestartOnlyAsTheClassAllows() {
        // Nothing left the wallet: any new request, retryable or not.
        for payment in [Self.retryable, Self.blocked] {
            let released = PaymentFailure(payment: payment, detail: "")
            XCTAssertTrue(released.permitsRestart(resuming: nil))
            XCTAssertTrue(released.permitsRestart(resuming: Self.other))
        }

        let resume = PaymentFailure(payment: Self.lockHeld, detail: "")
        XCTAssertFalse(resume.permitsRestart(resuming: nil), "never fund a new payment")
        XCTAssertFalse(resume.permitsRestart(resuming: Self.other))
        XCTAssertTrue(resume.permitsRestart(resuming: Self.held))

        for payment in [PlatformPaymentClass.maybeSent, nil] {
            let wait = PaymentFailure(payment: payment, detail: "")
            XCTAssertFalse(wait.permitsRestart(resuming: nil))
            XCTAssertFalse(wait.permitsRestart(resuming: Self.held))
        }
    }

    // MARK: - Each flow's failed state

    @MainActor
    private static func waitUntil(_ condition: () -> Bool) async throws {
        for _ in 0..<300 {
            if condition() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("condition not reached in time")
    }

    /// A registration that may have left keeps its slot: a fresh
    /// registration on it does not run, and its failed state is the
    /// helper's.
    @MainActor
    func testRegistrationFailureFollowsTheClass() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xA1, count: 32)
        let controller = coordinator.startRegistration(walletId: walletId, identityIndex: 3) {
            throw PlatformWalletError.walletOperation("lost", payment: .maybeSent)
        }
        try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
        guard case .failed(let failure) = controller.phase else { return XCTFail() }
        XCTAssertEqual(failure, PaymentFailure(payment: .maybeSent, detail: "lost"))
        XCTAssertFalse(controller.phase.permitsSubmit(resuming: nil))

        let ranAgain = Flag()
        let again = coordinator.startRegistration(walletId: walletId, identityIndex: 3) {
            ranAgain.value = true
            return Data()
        }
        XCTAssertTrue(again === controller)
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertFalse(ranAgain.value, "no fresh registration from a failure that may have left")
        guard case .failed = controller.phase else { return XCTFail("failed state was replaced") }

        // Dismissing the failure would drop the slot's gate: refused, so a
        // fresh registration on the index still does not run.
        XCTAssertFalse(coordinator.dismiss(walletId: walletId, identityIndex: 3))
        XCTAssertTrue(coordinator.controller(walletId: walletId, identityIndex: 3) === controller)
        _ = coordinator.startRegistration(walletId: walletId, identityIndex: 3) {
            ranAgain.value = true
            return Data()
        }
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertFalse(ranAgain.value, "no fresh registration after a refused dismiss")
    }

    /// Only a failure that releases its slot (any `.notSent`) is dismissed;
    /// one holding a lock, one that may have left, and one with no class
    /// keep it.
    @MainActor
    func testRegistrationFailureIsDismissedOnlyWhenNothingWasSent() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xA3, count: 32)
        let cases: [(UInt32, PlatformPaymentClass?, Bool)] = [
            (1, Self.retryable, true),
            (2, Self.blocked, true),
            (3, Self.lockHeld, false),
            (4, .maybeSent, false),
            (5, nil, false),
        ]
        for (index, payment, dismissable) in cases {
            let controller = coordinator.startRegistration(walletId: walletId, identityIndex: index) {
                throw Self.failure(payment)
            }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(coordinator.dismiss(walletId: walletId, identityIndex: index), dismissable)
            XCTAssertEqual(
                coordinator.controller(walletId: walletId, identityIndex: index) == nil,
                dismissable
            )
        }
    }

    /// A registration refused before anything was sent, even one Rust does
    /// not allow to retry as it is (an earlier payment still settling, too
    /// little in the asset lock), does not keep its index: a corrected new
    /// request on the same slot runs.
    @MainActor
    func testRegistrationNotSentFailureLetsANewRequestRun() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xA4, count: 32)
        let blocked = Self.blocked
        let controller = coordinator.startRegistration(walletId: walletId, identityIndex: 6) {
            throw PlatformWalletError.walletOperation("insufficient funds", payment: blocked)
        }
        try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }

        let newId = Data(repeating: 0x6B, count: 32)
        let again = coordinator.startRegistration(walletId: walletId, identityIndex: 6) { newId }
        try await Self.waitUntil {
            if case .completed(let id) = again.phase { return id == newId }
            return false
        }
        XCTAssertTrue(coordinator.controller(walletId: walletId, identityIndex: 6) === again)
    }

    /// A Platform-address funding's Dismiss drops the controller only when
    /// its failure releases the slot; otherwise the slot stays and a fresh
    /// funding of the same recipient does not run.
    @MainActor
    func testAddressFundingFailureIsDismissedOnlyWhenNothingWasSent() async throws {
        let coordinator = AddressFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xB1, count: 32)
        let cases: [(UInt8, PlatformPaymentClass?, Bool)] = [
            (1, Self.retryable, true),
            (2, Self.blocked, true),
            (3, Self.lockHeld, false),
            (4, .maybeSent, false),
            (5, nil, false),
        ]
        for (tag, payment, dismissable) in cases {
            let recipient = Data(repeating: tag, count: 20)
            let controller = coordinator.startFunding(
                walletId: walletId, platformAccountIndex: 0, recipientHash: recipient, recipientType: 0
            ) {
                throw Self.failure(payment)
            }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(
                coordinator.dismiss(walletId: walletId, platformAccountIndex: 0, recipientHash: recipient),
                dismissable,
                "\(String(describing: payment))"
            )
            let kept = coordinator.controller(
                walletId: walletId, platformAccountIndex: 0, recipientHash: recipient
            )
            XCTAssertEqual(kept == nil, dismissable)

            let ranAgain = Flag()
            let again = coordinator.startFunding(
                walletId: walletId, platformAccountIndex: 0, recipientHash: recipient, recipientType: 0
            ) {
                ranAgain.value = true
                return 1
            }
            if dismissable {
                try await Self.waitUntil { ranAgain.value }
                XCTAssertFalse(again === controller)
            } else {
                try await Task.sleep(nanoseconds: 50_000_000)
                XCTAssertFalse(ranAgain.value, "no fresh lock after a held failure")
                XCTAssertTrue(again === controller)
            }
        }
    }

    /// The shielded funding's Dismiss follows the same rule.
    @MainActor
    func testShieldedFundingFailureIsDismissedOnlyWhenNothingWasSent() async throws {
        let coordinator = ShieldedFundFromAssetLockCoordinator()
        let walletId = Data(repeating: 0xB2, count: 32)
        let cases: [(UInt8, PlatformPaymentClass?, Bool)] = [
            (1, Self.retryable, true),
            (2, Self.blocked, true),
            (3, Self.lockHeld, false),
            (4, .maybeSent, false),
            (5, nil, false),
        ]
        for (tag, payment, dismissable) in cases {
            let recipient = Data(repeating: tag, count: 43)
            let started = coordinator.startFunding(
                walletId: walletId, recipientRaw43: recipient, operationId: "fresh:\(tag)"
            ) {
                throw Self.failure(payment)
            }
            guard case .started(let controller, _) = started else { return XCTFail("funding did not start") }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(
                coordinator.dismiss(controller),
                dismissable,
                "\(String(describing: payment))"
            )
            XCTAssertEqual(
                coordinator.controller(walletId: walletId, recipientRaw43: recipient) == nil,
                dismissable
            )

            let ranAgain = Flag()
            _ = coordinator.startFunding(
                walletId: walletId, recipientRaw43: recipient, operationId: "fresh:\(tag):2"
            ) {
                ranAgain.value = true
            }
            if dismissable {
                try await Self.waitUntil { ranAgain.value }
            } else {
                try await Task.sleep(nanoseconds: 50_000_000)
                XCTAssertFalse(ranAgain.value, "no fresh lock after a held failure")
            }
        }
    }

    /// Each coordinator's `release` drops a failure that holds its slot
    /// without a lock (after which a fresh payment runs) and refuses any
    /// other, keeping its controller: a held lock is resumed instead.
    @MainActor
    func testCoordinatorsReleaseOnlyAHoldWithoutALock() async throws {
        let cases: [(UInt8, PlatformPaymentClass?, Bool)] = [
            (1, Self.retryable, false),
            (2, Self.lockHeld, false),
            (3, .maybeSent, true),
            (4, nil, true),
        ]

        let registrations = RegistrationCoordinator()
        let walletId = Data(repeating: 0xC1, count: 32)
        for (tag, payment, releasable) in cases {
            let index = UInt32(tag)
            let controller = registrations.startRegistration(walletId: walletId, identityIndex: index) {
                throw Self.failure(payment)
            }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(
                registrations.release(walletId: walletId, identityIndex: index),
                releasable,
                "\(String(describing: payment))"
            )
            XCTAssertEqual(registrations.controller(walletId: walletId, identityIndex: index) == nil, releasable)
            if releasable {
                let ran = Flag()
                _ = registrations.startRegistration(walletId: walletId, identityIndex: index) {
                    ran.value = true
                    return Data()
                }
                try await Self.waitUntil { ran.value }
            }
        }

        let addresses = AddressFundFromAssetLockCoordinator()
        for (tag, payment, releasable) in cases {
            let recipient = Data(repeating: tag, count: 20)
            let controller = addresses.startFunding(
                walletId: walletId, platformAccountIndex: 0, recipientHash: recipient, recipientType: 0
            ) {
                throw Self.failure(payment)
            }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(
                addresses.release(walletId: walletId, platformAccountIndex: 0, recipientHash: recipient),
                releasable,
                "\(String(describing: payment))"
            )
            XCTAssertEqual(
                addresses.controller(walletId: walletId, platformAccountIndex: 0, recipientHash: recipient) == nil,
                releasable
            )
            if releasable {
                let ran = Flag()
                _ = addresses.startFunding(
                    walletId: walletId, platformAccountIndex: 0, recipientHash: recipient, recipientType: 0
                ) {
                    ran.value = true
                    return 1
                }
                try await Self.waitUntil { ran.value }
            }
        }

        let shielded = ShieldedFundFromAssetLockCoordinator()
        for (tag, payment, releasable) in cases {
            let recipient = Data(repeating: tag, count: 43)
            let started = shielded.startFunding(
                walletId: walletId, recipientRaw43: recipient, operationId: "release:\(tag)"
            ) {
                throw Self.failure(payment)
            }
            guard case .started(let controller, _) = started else { return XCTFail("funding did not start") }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(
                shielded.release(walletId: walletId, recipientRaw43: recipient),
                releasable,
                "\(String(describing: payment))"
            )
            XCTAssertEqual(
                shielded.controller(walletId: walletId, recipientRaw43: recipient) == nil,
                releasable
            )
            if releasable {
                let ran = Flag()
                _ = shielded.startFunding(
                    walletId: walletId, recipientRaw43: recipient, operationId: "release:\(tag):2"
                ) {
                    ran.value = true
                }
                try await Self.waitUntil { ran.value }
            }
        }
    }

    /// The error a payment of `payment`'s class fails with; `nil` stands
    /// for an error Rust gave no class (an SDK timeout).
    private static func failure(_ payment: PlatformPaymentClass?) -> Error {
        guard let payment else { return SDKError.timeout("timeout") }
        return PlatformWalletError.walletOperation("refused", payment: payment)
    }

    /// A registration holding a lock restarts only through that lock's
    /// resume; a fresh registration on the slot does not run.
    @MainActor
    func testRegistrationHoldingALockRestartsOnlyByResumingIt() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xA2, count: 32)
        let lockHeld = Self.lockHeld
        let controller = coordinator.startRegistration(walletId: walletId, identityIndex: 4) {
            throw PlatformWalletError.walletOperation("proof wait timed out", payment: lockHeld)
        }
        try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
        XCTAssertFalse(controller.phase.permitsSubmit(resuming: nil))
        XCTAssertFalse(controller.phase.permitsSubmit(resuming: Self.other))

        let resumedId = Data(repeating: 0x5A, count: 32)
        _ = coordinator.startRegistration(walletId: walletId, identityIndex: 4, resuming: Self.held) {
            resumedId
        }
        try await Self.waitUntil {
            if case .completed(let id) = controller.phase { return id == resumedId }
            return false
        }
    }

    /// The shielded and Platform-address funding flows build their failed
    /// state from the helper and refuse a fresh funding after one that may
    /// have left.
    @MainActor
    func testAssetLockFundingFailuresFollowTheClass() async throws {
        let shield = ShieldedFundFromAssetLockController(
            walletId: Data(repeating: 1, count: 32),
            recipientRaw43: Data(repeating: 2, count: 43)
        )
        shield.submit(operationId: "shield:1") {
            throw PlatformWalletError.walletOperation("lost", payment: .maybeSent)
        }
        try await Self.waitUntil { if case .failed = shield.phase { return true }; return false }
        guard case .failed(let shieldFailure) = shield.phase else { return XCTFail() }
        XCTAssertEqual(shieldFailure.action, .wait)
        let shieldRan = Flag()
        let scheduled = shield.submit(operationId: "shield:2") { shieldRan.value = true }
        XCTAssertFalse(scheduled, "the failed phase refused a fresh shield")
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertFalse(shieldRan.value)

        let address = AddressFundFromAssetLockController(
            walletId: Data(repeating: 3, count: 32),
            platformAccountIndex: 0,
            recipientHash: Data(repeating: 4, count: 20),
            recipientType: 0
        )
        let lockHeld = Self.lockHeld
        address.submit {
            throw PlatformWalletError.walletOperation("proof wait timed out", payment: lockHeld)
        }
        try await Self.waitUntil { if case .failed = address.phase { return true }; return false }
        guard case .failed(let addressFailure) = address.phase else { return XCTFail() }
        XCTAssertEqual(addressFailure.action, .resume(Self.held))
        XCTAssertFalse(address.phase.permitsSubmit(resuming: nil))
        address.submit(resuming: Self.held) { 42 }
        try await Self.waitUntil {
            if case .completed(let balance) = address.phase { return balance == 42 }
            return false
        }
    }
}
