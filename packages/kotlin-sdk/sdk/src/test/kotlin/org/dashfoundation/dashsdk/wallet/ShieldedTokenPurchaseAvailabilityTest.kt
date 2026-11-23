package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * The values the shielded token purchase bridge shares with Rust: the
 * availability byte `platform_wallet_manager_shielded_token_purchase_availability`
 * writes and the fee kind `platform_wallet_shielded_estimate_fee` takes.
 */
class ShieldedTokenPurchaseAvailabilityTest {

    @Test
    fun shouldReadEachAvailabilityByteRustWrites() {
        assertEquals(
            ShieldedTokenPurchaseAvailability.Available,
            ShieldedTokenPurchaseAvailability.fromNative(0),
        )
        assertEquals(
            ShieldedTokenPurchaseAvailability.NoShieldedPool,
            ShieldedTokenPurchaseAvailability.fromNative(1),
        )
        assertEquals(
            ShieldedTokenPurchaseAvailability.ConfigurationUnreadable,
            ShieldedTokenPurchaseAvailability.fromNative(2),
        )
    }

    @Test
    fun shouldNeverReadAnUnknownByteAsAvailable() {
        // Includes -1, what the JNI export returns beside a pending exception.
        for (raw in listOf(-1, 3, 255)) {
            assertEquals(
                "raw $raw",
                ShieldedTokenPurchaseAvailability.ConfigurationUnreadable,
                ShieldedTokenPurchaseAvailability.fromNative(raw),
            )
        }
    }

    @Test
    fun shouldEstimateTheTokenPurchaseFeeWithKindFive() {
        assertEquals(5, ShieldedProver.FeeKind.TokenPurchaseFromPool.raw)
    }
}
