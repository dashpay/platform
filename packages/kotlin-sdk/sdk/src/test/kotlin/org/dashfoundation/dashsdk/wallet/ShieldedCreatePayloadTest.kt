package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentDelivery
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Regression coverage for the shielded-create codec boundary: Rust encodes
 * `[tag || identity_id[32] || delivery || retryable || diagnostic_utf8...]` in
 * `rs-unified-sdk-jni/src/funding.rs` and Kotlin decodes it in
 * [decodeShieldedCreatePayload]. A silent drift here would lose the
 * identity id, delivery class, retry or diagnostic on an ambiguous broadcast — the exact state a
 * retry must never be built from.
 */
class ShieldedCreatePayloadTest {

    private val id = ByteArray(32) { (it + 1).toByte() }

    private companion object {
        // `PlatformWalletFFIPaymentDelivery` discriminants.
        const val NONE: Byte = 0
        const val MAYBE_SENT: Byte = 2
        const val NOT_RETRYABLE: Byte = 0
        const val RETRYABLE: Byte = 1
    }

    @Test
    fun successReturnsTheIdentityId() {
        val packed = byteArrayOf(0) + id + byteArrayOf(NONE, NOT_RETRYABLE)
        assertArrayEquals(id, decodeShieldedCreatePayload(packed))
    }

    @Test
    fun unconfirmedThrowsTypedErrorWithIdAndDiagnostic() {
        val diagnostic = "proof wait timed out after 3 retries"
        val packed = byteArrayOf(1) + id + byteArrayOf(MAYBE_SENT, NOT_RETRYABLE) + diagnostic.encodeToByteArray()
        val e = runCatching { decodeShieldedCreatePayload(packed) }.exceptionOrNull()
        val unconfirmed = e as? DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed
        assertTrue("expected ShieldedCreateUnconfirmed, got $e", unconfirmed != null)
        assertArrayEquals(id, unconfirmed!!.identityId)
        // Rust's class, carried through the payload, not decided here.
        assertEquals(PaymentDelivery.MAYBE_SENT, unconfirmed.delivery)
        assertFalse(unconfirmed.isRetryable)
        assertTrue(
            "diagnostic lost: ${unconfirmed.message}",
            unconfirmed.message.orEmpty().contains(diagnostic),
        )
    }

    /// The retry is the native result's: Kotlin reads the byte, whatever the class.
    @Test
    fun unconfirmedReadsTheRetryTheNativeResultCarried() {
        val packed = byteArrayOf(1) + id + byteArrayOf(MAYBE_SENT, RETRYABLE)
        val e = runCatching { decodeShieldedCreatePayload(packed) }.exceptionOrNull()
            as DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed
        assertTrue(e.isRetryable)
    }

    @Test
    fun unconfirmedWithMultibyteUtf8DiagnosticRoundTrips() {
        val diagnostic = "确认失败 — proof ✗ (𝟛 retries)"
        val packed = byteArrayOf(1) + id + byteArrayOf(MAYBE_SENT, NOT_RETRYABLE) + diagnostic.encodeToByteArray()
        val e = runCatching { decodeShieldedCreatePayload(packed) }.exceptionOrNull()
            as DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed
        assertArrayEquals(id, e.identityId)
        assertTrue(e.message.orEmpty().contains(diagnostic))
    }

    @Test
    fun unconfirmedWithEmptyDiagnosticFallsBackToGenericMessage() {
        val packed = byteArrayOf(1) + id + byteArrayOf(MAYBE_SENT, NOT_RETRYABLE)
        val e = runCatching { decodeShieldedCreatePayload(packed) }.exceptionOrNull()
            as DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed
        assertArrayEquals(id, e.identityId)
        assertTrue(
            e.message.orEmpty().contains("shielded identity create broadcast unconfirmed"),
        )
    }

    @Test
    fun shortPayloadIsRejected() {
        val e = runCatching { decodeShieldedCreatePayload(ByteArray(34)) }.exceptionOrNull()
        assertEquals(IllegalStateException::class, e!!::class)
    }
}
