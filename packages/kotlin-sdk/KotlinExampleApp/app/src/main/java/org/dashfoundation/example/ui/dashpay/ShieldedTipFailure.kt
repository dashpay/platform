package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError

/**
 * Permit a fresh user review only for failures known not to have executed a tip.
 * On the tip path, Rust maps selection/build/recipient-check failures to
 * WalletOperation. Broadcast ambiguity maps to ShieldedSpendUnconfirmed, and
 * successful post-broadcast bookkeeping is best-effort (never WalletOperation).
 * Unknown exceptions, including JNI failures and cancellation, remain locked.
 */
internal fun canReviewShieldedTipAfterFailure(error: Exception): Boolean = when (error) {
    is IllegalArgumentException,
    is DashSdkError.InvalidParameter,
    is DashSdkError.PlatformWallet.InvalidHandle,
    is DashSdkError.PlatformWallet.NotFound,
    is DashSdkError.PlatformWallet.SigningKeyUnavailable,
    is DashSdkError.PlatformWallet.WalletOperation,
    is DashSdkError.PlatformWallet.ShieldedNoRecordedAnchor,
    is DashSdkError.PlatformWallet.ShieldedBroadcastFailed -> true
    // ErrorInvalidParameter is a preflight-only FFI failure on this call path.
    is DashSdkError.PlatformWallet.Generic -> error.nativeCode == 2
    else -> false
}
