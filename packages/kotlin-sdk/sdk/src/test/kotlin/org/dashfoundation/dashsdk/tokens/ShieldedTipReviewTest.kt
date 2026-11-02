package org.dashfoundation.dashsdk.tokens

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Marshalling of the native tip review and retry decision; the decisions themselves are Rust's. */
class ShieldedTipReviewTest {
    private fun review(change: Int, unresolved: Boolean): ByteArray =
        ByteArray(32) { 1 } + ByteArray(43) { 2 } + byteArrayOf(change.toByte()) +
            ByteArray(32) { 3 } + ByteArray(43) { 4 } + byteArrayOf(if (unresolved) 1 else 0)

    @Test
    fun shouldDecodeEachRecipientChange() {
        val new = ShieldedTipReview.fromNative(review(0, unresolved = false))
        assertEquals(ShieldedTipRecipientChange.New, new.change)
        assertArrayEquals(ByteArray(32) { 1 }, new.recipient.identityId)
        assertArrayEquals(ByteArray(43) { 2 }, new.recipient.address)
        assertFalse(new.unresolved)

        assertEquals(
            ShieldedTipRecipientChange.Unchanged,
            ShieldedTipReview.fromNative(review(1, unresolved = true)).change,
        )

        val changed = ShieldedTipReview.fromNative(review(2, unresolved = true))
        val previous = (changed.change as ShieldedTipRecipientChange.Changed).previous
        assertArrayEquals(ByteArray(32) { 3 }, previous.identityId)
        assertArrayEquals(ByteArray(43) { 4 }, previous.address)
        assertTrue(changed.unresolved)
    }

    @Test
    fun shouldTreatAnyDispositionButNotSentAsUnresolved() {
        assertEquals(ShieldedTipDisposition.NOT_SENT, ShieldedTipDisposition.fromNative(1))
        for (value in listOf(0, 2, -1)) {
            assertEquals(ShieldedTipDisposition.UNRESOLVED, ShieldedTipDisposition.fromNative(value))
        }
    }
}
