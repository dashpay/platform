package org.dashfoundation.example.ui.dashpay

import java.util.concurrent.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ShieldedTipFailureTest {
    @Test
    fun permitsFreshReviewAfterDefinitiveFailures() {
        val errors = listOf(
            IllegalArgumentException("amount must be positive"),
            DashSdkError.InvalidParameter("invalid account"),
            DashSdkError.PlatformWallet.Generic(2, "invalid memo"),
            DashSdkError.PlatformWallet.InvalidHandle("closed"),
            DashSdkError.PlatformWallet.NotFound("wallet removed"),
            DashSdkError.PlatformWallet.SigningKeyUnavailable("unlock wallet"),
            DashSdkError.PlatformWallet.WalletOperation("The tip recipient changed; review and confirm again"),
            DashSdkError.PlatformWallet.WalletOperation("insufficient notes"),
            DashSdkError.PlatformWallet.ShieldedNoRecordedAnchor("sync first"),
            DashSdkError.PlatformWallet.ShieldedBroadcastFailed("consensus rejected"),
        )
        errors.forEach { assertTrue(it.toString(), canReviewShieldedTipAfterFailure(it)) }
    }

    @Test
    fun retainsLockForAmbiguousAndUnclassifiedOutcomes() {
        val errors = listOf(
            DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed("no result proof"),
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("no relay verdict"),
            DashSdkError.Timeout("timed out"),
            DashSdkError.NetworkError("connection lost"),
            DashSdkError.PlatformWallet.Generic(999, "unknown native error"),
            RuntimeException("JNI failure"),
            CancellationException("cancelled"),
            IllegalStateException("unexpected state"),
        )
        errors.forEach { assertFalse(it.toString(), canReviewShieldedTipAfterFailure(it)) }
    }
}
