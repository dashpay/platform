import Foundation
import SwiftDashSDK

/// How the app presents a failed payment: the text to show and the one
/// action Rust's class allows. Every payment screen builds its failed state
/// from this, so none of them decides "retry", "resume" or "wait" from an
/// error's case, code or text.
///
/// The class is the error's `payment` (delivery, `isRetryable`, the held
/// lock), read through `Recovery`. The SDK only carries the class; this
/// mapping is the app's (← Kotlin app `PaymentFailure`):
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

    /// What the app may offer a person after a failed payment, read from
    /// Rust's class alone (`delivery`, `isRetryable` and the held lock),
    /// never from the error's case, code or text.
    enum Recovery: Equatable {
        /// Nothing left the wallet and Rust allows running the operation again.
        case retry
        /// Nothing left the wallet, and Rust does not allow running it again
        /// as it is. The reason is the error's own (its message).
        case wait
        /// The payment holds the tracked asset lock at this outpoint: resume
        /// it, never fund a new one.
        case resume(AssetLockBuiltOutPoint)
        /// The payment may have left the wallet: let sync settle it and never
        /// offer a second one.
        case reconcile
        /// No wallet error classified the failure: unconfirmed, never retried.
        case unconfirmed

        init(_ payment: PlatformPaymentClass?) {
            guard let payment else {
                self = .unconfirmed
                return
            }
            switch payment.delivery {
            case .notSent: self = payment.isRetryable ? .retry : .wait
            case .maybeSent: self = .reconcile
            case .lockHeld(let outPoint): self = .resume(outPoint)
            }
        }

        /// What happened to the payment and the one action the class allows.
        var suggestion: String {
            switch self {
            case .retry:
                return "Nothing was sent. You can try again."
            case .wait:
                return "Nothing was sent. This request cannot go through as it is."
            case .resume:
                return "The funds are held in an asset lock. Resume that lock to finish; do not start a new payment."
            case .reconcile:
                return "The payment may have gone through. Wait for the wallet to sync before doing anything else; "
                    + "do not send it again."
            case .unconfirmed:
                return "Submission result is unconfirmed. Check history and reconcile the original operation "
                    + "before submitting again."
            }
        }

        /// `suggestion` for a failed invitation create, whose held lock is
        /// the invitation's voucher: the generic resume refuses a voucher
        /// lock, and its funds come back through the voucher's reclaim.
        var invitationVoucherSuggestion: String {
            switch self {
            case .resume:
                return "The funds are held in the invitation's voucher lock. Reclaim the voucher from Sent "
                    + "invitations to get them back; do not create a new invitation."
            case .retry, .wait, .reconcile, .unconfirmed:
                return suggestion
            }
        }
    }

    let recovery: Recovery
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
        self.recovery = Recovery(payment)
        self.detail = detail
        self.invitationVoucher = invitationVoucher
    }

    /// The failure a payment screen's `error` stands for, or `nil` when no
    /// payment was attempted, which the screen shows as its own message
    /// (← Kotlin `PaymentFailure.ofPayment`): only the screen's own input
    /// check (not an SDK error). Every SDK error comes from a call that may
    /// have started the payment, so it is a failure; a wallet error with no
    /// class (the FFI's `Unknown` delivery decodes to none, and a paid call
    /// can come back that way) is unconfirmed and holds its slot, never a
    /// refusal before any payment.
    static func ofPayment(_ error: Error, invitationVoucher: Bool = false) -> PaymentFailure? {
        switch error {
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
        case let drain as WalletDrainAttemptError: return drain.payment
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

    /// Whether the failure may have left the wallet with no lock to resume
    /// (`holdsSlot` without `heldLock`), so the user releases its slot once
    /// they confirmed the original payment in the history. Never released
    /// automatically: only the history shows whether the payment went through.
    var releasableAfterReconciliation: Bool { holdsSlot && heldLock == nil }
}

extension AssetLockBuiltOutPoint {
    /// The outpoint a resume call takes (`txid` in wire order), for naming
    /// the lock a restart resumes. `nil` for a txid that is not 32 bytes.
    static func resuming(txid: Data, vout: UInt32) -> AssetLockBuiltOutPoint? {
        AssetLockBuiltOutPoint(rawBytes: txid + withUnsafeBytes(of: vout.littleEndian) { Data($0) })
    }
}
