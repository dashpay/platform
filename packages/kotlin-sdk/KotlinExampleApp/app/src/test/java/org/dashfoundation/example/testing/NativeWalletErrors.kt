package org.dashfoundation.example.testing

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentDelivery
import org.dashfoundation.dashsdk.ffi.DashSDKException

/**
 * A platform-wallet error as the native layer reports it: Rust's [code] with
 * its payment class. The app reads "may have gone through", retry and a held
 * lock from that class, never from the error's type, so a fixture built
 * without it reads as unclassified.
 */
internal fun nativeWalletError(
    code: Int,
    message: String,
    delivery: PaymentDelivery,
    retryable: Boolean = false,
): DashSdkError = DashSdkError.fromNative(
    DashSDKException(
        code = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + code,
        message = message,
        consensusError = null,
        paymentDelivery = delivery,
        retryable = retryable,
    ),
)

/** `ErrorShieldedSpendUnconfirmed` (18): a shielded spend that may have left. */
internal fun spendUnconfirmed(message: String): DashSdkError =
    nativeWalletError(18, message, PaymentDelivery.MAYBE_SENT)

/** `ErrorTransactionBroadcastUnconfirmed` (20): a transition that may have left. */
internal fun broadcastUnconfirmed(message: String): DashSdkError =
    nativeWalletError(20, message, PaymentDelivery.MAYBE_SENT)
