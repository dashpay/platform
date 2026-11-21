import Foundation
import SwiftDashSDK

/// How the app presents a failed payment: the text to show and the one
/// action Rust's class allows. Every payment screen builds its failed state
/// from this, so none of them decides "retry", "resume" or "wait" from an
/// error's case, code or text.
///
/// The class is the error's `payment` (delivery, `isRetryable`, the held
/// lock), read through the SDK's `PlatformPaymentRecovery`:
/// - a retryable `.notSent` offers a retry;
/// - `.lockHeld` resumes the named lock and never funds a new one (an
///   invitation's voucher lock is reclaimed instead, `invitationVoucher`);
/// - everything else waits: a non-retryable `.notSent` (it cannot go
///   through as it is, for the reason its message states), `.maybeSent`
///   (sync settles it) and an unclassified failure (unconfirmed, never
///   retried).
///
/// Whether the failure keeps its slot is `holdsSlot`: only a `.notSent` one
/// releases it.
struct PaymentFailure: Equatable {
    /// The one thing a screen may offer after the failure.
    enum Action: Equatable {
        /// Run the operation again (a fresh registration or funding).
        case retry
        /// Resume the tracked asset lock at this outpoint, through the
        /// screen's existing resume flow.
        case resume(AssetLockBuiltOutPoint)
        /// Offer nothing that pays again.
        case wait
    }

    let recovery: PlatformPaymentRecovery
    /// The error's own description, shown after the class's text.
    let detail: String
    /// Whether the operation that failed created an invitation, so a held
    /// lock is the invitation's voucher: its funds come back through the
    /// voucher's reclaim, not the generic resume (which refuses a voucher).
    /// Known from the operation, never from the error.
    let invitationVoucher: Bool

    init(_ error: Error, invitationVoucher: Bool = false) {
        self.init(
            payment: Self.paymentClass(of: error),
            detail: error.localizedDescription,
            invitationVoucher: invitationVoucher
        )
    }

    init(payment: PlatformPaymentClass?, detail: String, invitationVoucher: Bool = false) {
        self.recovery = PlatformPaymentRecovery(payment)
        self.detail = detail
        self.invitationVoucher = invitationVoucher
    }

    /// The failure a payment screen's `error` stands for, or `nil` when no
    /// payment was attempted, which the screen shows as its own message
    /// (← Kotlin `PaymentFailure.ofPayment`): the screen's own input check (not
    /// an SDK error), or a wallet error Rust gave no class. Rust classes every
    /// failure of a payment the wallet ran, so a wallet error without one is a
    /// refusal before any payment (a Swift-side input check, a native pointer
    /// or handle refusal), as `ErrorCategorizer` presents it too.
    static func ofPayment(_ error: Error, invitationVoucher: Bool = false) -> PaymentFailure? {
        switch error {
        case let wallet as PlatformWalletError where wallet.payment == nil:
            return nil
        case is PlatformWalletError, is ShieldedIdentityCreateUnconfirmedError, is SDKError:
            return PaymentFailure(error, invitationVoucher: invitationVoucher)
        default:
            return nil
        }
    }

    /// Rust's class of the error, `nil` when it carries none.
    static func paymentClass(of error: Error) -> PlatformPaymentClass? {
        switch error {
        case let wallet as PlatformWalletError: return wallet.payment
        case let unconfirmed as ShieldedIdentityCreateUnconfirmedError: return unconfirmed.payment
        default: return nil
        }
    }

    var action: Action {
        switch recovery {
        case .retry: return .retry
        case .resume(let outPoint): return .resume(outPoint)
        case .wait, .reconcile, .unconfirmed: return .wait
        }
    }

    /// The lock to resume, `nil` unless the action is `.resume`.
    var heldLock: AssetLockBuiltOutPoint? {
        if case .resume(let outPoint) = action { return outPoint }
        return nil
    }

    /// A short heading for the failed state.
    var title: String {
        switch recovery {
        case .retry: return "Failed"
        case .wait: return "Not sent"
        case .resume:
            return invitationVoucher ? "Funds held in the invitation voucher" : "Funds held in an asset lock"
        case .reconcile: return "May have gone through"
        case .unconfirmed: return "Not confirmed"
        }
    }

    /// The class's text, the held lock when there is one, then the detail.
    var message: String {
        if let heldLock, invitationVoucher {
            return "\(recovery.invitationVoucherSuggestion) Voucher lock \(heldLock.outPointHex) is "
                + "listed under Sent invitations. \(detail)"
        }
        if let heldLock {
            return "\(recovery.suggestion) Asset lock \(heldLock.outPointHex) is listed with the "
                + "wallet's resumable asset locks. \(detail)"
        }
        return "\(recovery.suggestion) \(detail)"
    }

    /// Whether the failure holds its slot (the screen's form, the
    /// coordinator's controller, its Dismiss) until it is resumed or the
    /// user leaves the flow. Every gate on a failed payment reads this.
    ///
    /// A failure that may have left the wallet (`.maybeSent`), holds an
    /// asset lock (`.lockHeld`) or carries no class holds it: a new request
    /// could pay a second time. Any `.notSent` failure, retryable or not,
    /// releases it: nothing left the wallet, so Dismiss and a corrected new
    /// request are allowed (Rust refuses again if something still blocks).
    var holdsSlot: Bool {
        switch recovery {
        case .retry, .wait: return false
        case .resume, .reconcile, .unconfirmed: return true
        }
    }

    /// Whether a failed attempt may be started again by an operation that
    /// resumes `resuming` (`nil` for a fresh one): any operation after a
    /// failure that releases its slot (`holdsSlot`), only the resume of the
    /// held lock after a held one, nothing after any other.
    func permitsRestart(resuming: AssetLockBuiltOutPoint?) -> Bool {
        guard holdsSlot else { return true }
        return heldLock.map { resuming == $0 } ?? false
    }
}

extension AssetLockBuiltOutPoint {
    /// The outpoint a resume call takes (`txid` in wire order), for naming
    /// the lock a restart resumes. `nil` for a txid that is not 32 bytes.
    static func resuming(txid: Data, vout: UInt32) -> AssetLockBuiltOutPoint? {
        AssetLockBuiltOutPoint(rawBytes: txid + withUnsafeBytes(of: vout.littleEndian) { Data($0) })
    }
}
