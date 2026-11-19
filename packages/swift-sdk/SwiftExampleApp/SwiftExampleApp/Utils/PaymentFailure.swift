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
/// - `.lockHeld` resumes the named lock and never funds a new one;
/// - everything else waits: a non-retryable `.notSent` (an earlier payment
///   may still be settling), `.maybeSent` (sync settles it) and an
///   unclassified failure (unconfirmed, never retried).
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

    init(_ error: Error) {
        self.init(payment: Self.paymentClass(of: error), detail: error.localizedDescription)
    }

    init(payment: PlatformPaymentClass?, detail: String) {
        self.recovery = PlatformPaymentRecovery(payment)
        self.detail = detail
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

    var allowsRetry: Bool { action == .retry }

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
        case .resume: return "Funds held in an asset lock"
        case .reconcile: return "May have gone through"
        case .unconfirmed: return "Not confirmed"
        }
    }

    /// The class's text, the held lock when there is one, then the detail.
    var message: String {
        if let heldLock {
            return "\(recovery.suggestion) Asset lock \(heldLock.outPointHex) is listed with the "
                + "wallet's resumable asset locks. \(detail)"
        }
        return "\(recovery.suggestion) \(detail)"
    }

    /// Whether a failed attempt may be started again by an operation that
    /// resumes `resuming` (`nil` for a fresh one): any operation after a
    /// retryable failure, only the resume of the held lock after a held one,
    /// nothing after one that waits.
    func permitsRestart(resuming: AssetLockBuiltOutPoint?) -> Bool {
        switch action {
        case .retry: return true
        case .resume(let held): return resuming == held
        case .wait: return false
        }
    }
}

extension AssetLockBuiltOutPoint {
    /// The outpoint a resume call takes (`txid` in wire order), for naming
    /// the lock a restart resumes. `nil` for a txid that is not 32 bytes.
    static func resuming(txid: Data, vout: UInt32) -> AssetLockBuiltOutPoint? {
        AssetLockBuiltOutPoint(rawBytes: txid + withUnsafeBytes(of: vout.littleEndian) { Data($0) })
    }
}
