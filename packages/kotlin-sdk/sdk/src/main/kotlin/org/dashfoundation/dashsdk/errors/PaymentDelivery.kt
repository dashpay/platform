package org.dashfoundation.dashsdk.errors

/**
 * Whether a failed payment may have left the wallet, as the native layer
 * classified it. Decide "retry" or "wait" from this, never from lists of
 * error types or codes: the Rust wallet owns the classification.
 *
 * Read it from [DashSdkError.delivery]. Values mirror
 * `PlatformWalletFFIPaymentDelivery` in `rs-platform-wallet-ffi/src/error.rs`.
 */
enum class PaymentDelivery {
    /** The payment never left the wallet and can never execute: a retry may be offered. */
    NOT_SENT,

    /**
     * The payment may have left the wallet, and may execute or already has:
     * wait for sync to settle it and do not offer a second payment.
     */
    MAYBE_SENT,
    ;

    internal companion object {
        /**
         * `null` for the native `None` (0), which only a success carries. A
         * value this SDK build does not know reads as [MAYBE_SENT], the class
         * that never invites a second payment.
         */
        fun fromNative(value: Int): PaymentDelivery? = when (value) {
            0 -> null
            1 -> NOT_SENT
            else -> MAYBE_SENT
        }
    }
}
