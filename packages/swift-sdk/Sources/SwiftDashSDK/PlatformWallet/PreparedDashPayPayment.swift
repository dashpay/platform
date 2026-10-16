import Foundation
import DashSDKFFI

/// A DashPay contact payment signed in Rust and not broadcast yet.
///
/// Its funding inputs stay reserved until it is passed to exactly one of
/// `ManagedPlatformWallet.sendPreparedDashPayPayment(_:)` or
/// `ManagedPlatformWallet.abandonPreparedDashPayPayment(_:)`. Releasing the
/// object without either abandons it.
///
/// `@unchecked Sendable`: every stored property is immutable except the
/// native handle, which only `take()` changes, under `lock`.
public final class PreparedDashPayPayment: @unchecked Sendable {
    private let lock = NSLock()
    private var nativeHandle: Handle
    /// The amount the contact receives, in duffs.
    public let amountDuffs: UInt64
    /// The exact network fee of the signed transaction, in duffs.
    public let feeDuffs: UInt64
    /// What leaves the wallet: amount plus fee, in duffs.
    public let totalDuffs: UInt64

    init(handle: Handle, amountDuffs: UInt64, feeDuffs: UInt64, totalDuffs: UInt64) {
        self.nativeHandle = handle
        self.amountDuffs = amountDuffs
        self.feeDuffs = feeDuffs
        self.totalDuffs = totalDuffs
    }

    deinit {
        guard nativeHandle != 0 else { return }
        // Abandoning takes the wallet-manager lock to release the inputs;
        // the last reference is often dropped on the main thread, which must
        // not wait on it.
        let handle = nativeHandle
        DispatchQueue.global(qos: .utility).async {
            platform_wallet_prepared_dashpay_payment_free(handle)
        }
    }

    /// Hand the native handle over to a send or an abandon, once.
    func take() throws -> Handle {
        lock.lock()
        defer { lock.unlock() }
        guard nativeHandle != 0 else {
            throw PlatformWalletError.unknown("PreparedDashPayPayment already sent or abandoned")
        }
        let value = nativeHandle
        nativeHandle = 0
        return value
    }
}
