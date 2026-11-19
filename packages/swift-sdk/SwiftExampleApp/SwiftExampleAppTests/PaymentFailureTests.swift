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
        let cases: [(PlatformPaymentClass?, PaymentFailure.Action, PlatformPaymentRecovery)] = [
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
            XCTAssertEqual(failure.allowsRetry, action == .retry)
            if action != .retry {
                XCTAssertFalse(failure.message.lowercased().contains("try again"), failure.message)
            }
        }
        let resume = PaymentFailure(payment: Self.lockHeld, detail: "d")
        XCTAssertEqual(resume.heldLock, Self.held)
        XCTAssertTrue(resume.message.contains(Self.held.outPointHex))
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

    /// Only a payment that was attempted is a payment failure: an SDK error
    /// (with or without Rust's class), never the screen's own input check or
    /// a wallet error Rust gave no class (a Swift-side check before any
    /// native call), which show as their own message (← Kotlin
    /// `PaymentFailure.ofPayment`).
    func testShouldPresentOnlyAnAttemptedPaymentAsAPaymentFailure() {
        XCTAssertEqual(
            PaymentFailure.ofPayment(PlatformWalletError.walletOperation("lost", payment: .maybeSent)),
            PaymentFailure(payment: .maybeSent, detail: "lost")
        )
        XCTAssertEqual(PaymentFailure.ofPayment(SDKError.timeout("timeout"))?.action, .wait)
        XCTAssertNil(PaymentFailure.ofPayment(PlatformWalletError.invalidParameter("Enter an amount")))
        XCTAssertNil(PaymentFailure.ofPayment(CocoaError(.validationNumberTooSmall)))
    }

    /// A failed invitation create holding its voucher lock tells the user to
    /// reclaim the voucher, not to resume the lock: the operation says it is
    /// a voucher, the class is unchanged.
    func testShouldReclaimAnInvitationVoucherInsteadOfResumingIt() {
        let voucher = PaymentFailure(payment: Self.lockHeld, detail: "d", invitationVoucher: true)
        XCTAssertEqual(voucher.action, .resume(Self.held))
        XCTAssertFalse(voucher.permitsRestart(resuming: nil), "never fund a new invitation")
        XCTAssertTrue(voucher.message.hasPrefix(PlatformPaymentRecovery.resume(Self.held).invitationVoucherSuggestion))
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

    func testShouldRestartOnlyAsTheClassAllows() {
        let retry = PaymentFailure(payment: Self.retryable, detail: "")
        XCTAssertTrue(retry.permitsRestart(resuming: nil))
        XCTAssertTrue(retry.permitsRestart(resuming: Self.other))

        let resume = PaymentFailure(payment: Self.lockHeld, detail: "")
        XCTAssertFalse(resume.permitsRestart(resuming: nil), "never fund a new payment")
        XCTAssertFalse(resume.permitsRestart(resuming: Self.other))
        XCTAssertTrue(resume.permitsRestart(resuming: Self.held))

        for payment in [Self.blocked, .maybeSent, nil] {
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

    /// Only a failure whose class allows a fresh registration is dismissed;
    /// one holding a lock or waiting for an earlier payment keeps its slot.
    @MainActor
    func testRegistrationFailureIsDismissedOnlyWhenItsClassAllowsARetry() async throws {
        let coordinator = RegistrationCoordinator()
        let walletId = Data(repeating: 0xA3, count: 32)
        let cases: [(UInt32, PlatformPaymentClass, Bool)] = [
            (1, Self.retryable, true),
            (2, Self.blocked, false),
            (3, Self.lockHeld, false),
            (4, .maybeSent, false),
        ]
        for (index, payment, dismissable) in cases {
            let controller = coordinator.startRegistration(walletId: walletId, identityIndex: index) {
                throw PlatformWalletError.walletOperation("refused", payment: payment)
            }
            try await Self.waitUntil { if case .failed = controller.phase { return true }; return false }
            XCTAssertEqual(coordinator.dismiss(walletId: walletId, identityIndex: index), dismissable)
            XCTAssertEqual(
                coordinator.controller(walletId: walletId, identityIndex: index) == nil,
                dismissable
            )
        }
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
        shield.submit(operationId: "shield:2") { shieldRan.value = true }
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
