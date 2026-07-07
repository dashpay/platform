import Foundation
import DashSDKFFI

/// Direction of a DashPay payment from the owner's perspective.
/// Mirrors the FFI `DashpayPaymentDirectionFFI` discriminants.
public enum DashPayPaymentDirection: UInt8, Sendable, Equatable {
    /// The owner sent this payment to the counterparty.
    case sent = 0
    /// The owner received this payment from the counterparty.
    case received = 1
}

/// Core-chain status of a DashPay payment. Mirrors the FFI
/// `DashpayPaymentStatusFFI` discriminants.
public enum DashPayPaymentStatus: UInt8, Sendable, Equatable {
    /// Broadcast but not yet confirmed.
    case pending = 0
    /// Confirmed on Core chain.
    case confirmed = 1
    /// Broadcast failed or the transaction was dropped.
    case failed = 2
}

/// One DashPay payment-history entry — a Swift-owned copy of the
/// Rust-side `PaymentEntry` row on a `ManagedIdentity`
/// (`dashpay_payments`, keyed by txid).
///
/// The source `PaymentEntry` carries **no timestamp field** (the
/// underlying model keys history by txid and does not record a
/// wall-clock time), so none is surfaced here — ordering is by txid /
/// arrival, matching the Rust map.
///
/// Read via `ManagedIdentity.getDashPayPayments()` /
/// `ManagedPlatformWallet.getDashPayPayments(identityId:)`, persisted
/// into `PersistentDashpayPayment` rows by
/// `PlatformWalletManager.refreshDashPayPayments(walletId:identityId:)`.
public struct DashPayPayment: Sendable, Equatable {
    /// The other identity in this payment. Whether they are the
    /// sender or the receiver is encoded in `direction`.
    public let counterpartyId: Identifier
    /// Amount in duffs. Always positive; `direction` carries the sign.
    public let amountDuffs: UInt64
    /// Payment direction from the owner's perspective.
    public let direction: DashPayPaymentDirection
    /// Core-chain status.
    public let status: DashPayPaymentStatus
    /// Transaction id (hex), the Rust `dashpay_payments` map key.
    public let txid: String
    /// Sender memo, when present. `nil` mirrors the source `Option`
    /// being `None`.
    public let memo: String?

    public init(
        counterpartyId: Identifier,
        amountDuffs: UInt64,
        direction: DashPayPaymentDirection,
        status: DashPayPaymentStatus,
        txid: String,
        memo: String? = nil
    ) {
        self.counterpartyId = counterpartyId
        self.amountDuffs = amountDuffs
        self.direction = direction
        self.status = status
        self.txid = txid
        self.memo = memo
    }

    /// Copy a `DashpayPaymentFFI` row into a Swift-owned value. The
    /// caller retains ownership of the FFI struct and is responsible
    /// for freeing the array afterward with
    /// `dashpay_payment_array_free` — this initializer only *reads*
    /// the pointers.
    ///
    /// Unknown direction / status discriminants fall back to `.sent` /
    /// `.pending` rather than failing — a newer Rust enum case must
    /// not make the whole history unreadable.
    init(ffi: DashpayPaymentFFI) {
        var counterparty = ffi.counterparty_id
        self.counterpartyId = Swift.withUnsafeBytes(of: &counterparty) { Data($0) }
        self.amountDuffs = ffi.amount_duffs
        self.direction = DashPayPaymentDirection(rawValue: ffi.direction) ?? .sent
        self.status = DashPayPaymentStatus(rawValue: ffi.status) ?? .pending
        if let txidPtr = ffi.txid {
            self.txid = String(cString: txidPtr)
        } else {
            // `txid` is documented always non-null; degrade to an
            // empty string defensively rather than trapping.
            self.txid = ""
        }
        if let memoPtr = ffi.memo {
            self.memo = String(cString: memoPtr)
        } else {
            self.memo = nil
        }
    }
}
