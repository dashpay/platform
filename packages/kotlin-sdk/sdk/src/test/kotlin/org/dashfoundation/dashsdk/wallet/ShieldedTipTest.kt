package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.ByteBuffer

/**
 * Reading the BLOBs the shielded tip exports return (layouts in
 * `rs-unified-sdk-jni/src/funding.rs`) into the public types.
 */
class ShieldedTipTest {

    private val recipientId = ByteArray(32) { 5 }
    private val recipientAddress = ByteArray(43) { 6 }
    private val previousId = ByteArray(32) { 7 }
    private val previousAddress = ByteArray(43) { 8 }

    /** The review BLOB as the JNI export writes it, the C struct's fields in order. */
    private fun reviewBlob(
        change: Int,
        unresolved: Int,
        previousIdentityId: ByteArray = ByteArray(32),
        previousRawAddress: ByteArray = ByteArray(43),
    ): ByteArray =
        recipientId + recipientAddress + byteArrayOf(change.toByte()) +
            previousIdentityId + previousRawAddress + byteArrayOf(unresolved.toByte())

    /** The balance BLOB as the JNI export writes it: `u8 state, u32 account, u64 credits`. */
    private fun balanceBlob(state: Int, account: Int, credits: Long): ByteArray =
        ByteBuffer.allocate(13).put(state.toByte()).putInt(account).putLong(credits).array()

    @Test
    fun shouldReadANewRecipient() {
        val review = decodeShieldedTipReview(reviewBlob(change = 0, unresolved = 0))
        assertEquals(ShieldedTipRecipient(recipientId, recipientAddress), review.recipient)
        assertEquals(ShieldedTipRecipientChange.New, review.change)
        assertFalse(review.unresolved)
    }

    @Test
    fun shouldReadAnUnchangedRecipient() {
        val review = decodeShieldedTipReview(reviewBlob(change = 1, unresolved = 0))
        assertEquals(ShieldedTipRecipient(recipientId, recipientAddress), review.recipient)
        assertEquals(ShieldedTipRecipientChange.Unchanged, review.change)
        assertFalse(review.unresolved)
    }

    @Test
    fun shouldReadAChangedRecipientWithThePreviousOne() {
        val review = decodeShieldedTipReview(
            reviewBlob(change = 2, unresolved = 0, previousId, previousAddress),
        )
        assertEquals(ShieldedTipRecipient(recipientId, recipientAddress), review.recipient)
        assertEquals(
            ShieldedTipRecipientChange.Changed(ShieldedTipRecipient(previousId, previousAddress)),
            review.change,
        )
        assertFalse(review.unresolved)
    }

    @Test
    fun shouldCarryAPreviousRecipientOnlyWhenTheRecipientChanged() {
        // The C struct zeroes the previous recipient unless the recipient
        // changed; even bytes in that place are not read as one otherwise.
        for (change in 0..1) {
            val review = decodeShieldedTipReview(
                reviewBlob(change, unresolved = 0, previousId, previousAddress),
            )
            assertFalse(review.change is ShieldedTipRecipientChange.Changed)
        }
    }

    @Test
    fun shouldReadTheUnresolvedFlagForEveryChange() {
        for (change in 0..2) {
            assertTrue(decodeShieldedTipReview(reviewBlob(change, unresolved = 1)).unresolved)
            assertFalse(decodeShieldedTipReview(reviewBlob(change, unresolved = 0)).unresolved)
        }
    }

    @Test
    fun shouldRefuseAReviewItCannotReadExactly() {
        val unknownChange = reviewBlob(change = 3, unresolved = 0)
        val unknownFlag = reviewBlob(change = 0, unresolved = 2)
        val whole = reviewBlob(change = 0, unresolved = 0)
        val malformed = listOf(
            unknownChange,
            unknownFlag,
            whole.copyOf(whole.size - 1),
            whole + byteArrayOf(0),
            ByteArray(0),
        )
        for (blob in malformed) {
            assertThrows(DashSdkError.SerializationError::class.java) {
                decodeShieldedTipReview(blob)
            }
        }
    }

    @Test
    fun shouldCompareRecipientsByTheirBytes() {
        val one = ShieldedTipRecipient(recipientId.copyOf(), recipientAddress.copyOf())
        val same = ShieldedTipRecipient(recipientId.copyOf(), recipientAddress.copyOf())
        assertEquals(one, same)
        assertEquals(one.hashCode(), same.hashCode())
        assertNotEquals(one, ShieldedTipRecipient(previousId, recipientAddress))
        assertNotEquals(one, ShieldedTipRecipient(recipientId, previousAddress))
        assertEquals(
            ShieldedTipSource.Identity(recipientId.copyOf()),
            ShieldedTipSource.Identity(recipientId.copyOf()),
        )
        assertEquals(
            ShieldedTipSource.Identity(recipientId.copyOf()).hashCode(),
            ShieldedTipSource.Identity(recipientId.copyOf()).hashCode(),
        )
        assertNotEquals(
            ShieldedTipSource.Identity(recipientId),
            ShieldedTipSource.Identity(previousId),
        )
    }

    @Test
    fun shouldReadAnUnverifiedIdentityBalance() {
        assertEquals(
            ShieldedIdentityBalance.Unverified,
            decodeShieldedIdentityBalance(balanceBlob(state = 0, account = 0, credits = 0)),
        )
    }

    @Test
    fun shouldReadAnUnavailableIdentityBalanceWithItsAccount() {
        val identityAccount = 0x4000_0003
        assertEquals(
            ShieldedIdentityBalance.Unavailable(identityAccount),
            decodeShieldedIdentityBalance(balanceBlob(state = 1, account = identityAccount, credits = 0)),
        )
    }

    @Test
    fun shouldReadAnAvailableIdentityBalanceWithItsAccountAndCredits() {
        val identityAccount = 0x4000_0003
        assertEquals(
            ShieldedIdentityBalance.Available(identityAccount, 150_000_000_000),
            decodeShieldedIdentityBalance(
                balanceBlob(state = 2, account = identityAccount, credits = 150_000_000_000),
            ),
        )
        // An established account that holds nothing is a known zero.
        assertEquals(
            ShieldedIdentityBalance.Available(identityAccount, 0),
            decodeShieldedIdentityBalance(balanceBlob(state = 2, account = identityAccount, credits = 0)),
        )
    }

    @Test
    fun shouldRefuseAnIdentityBalanceItCannotReadExactly() {
        val whole = balanceBlob(state = 2, account = 1, credits = 1)
        val malformed = listOf(
            // A state this build does not know.
            balanceBlob(state = 3, account = 1, credits = 1),
            // An account past what an Int carries (0x8000_0000).
            balanceBlob(state = 1, account = Int.MIN_VALUE, credits = 0),
            // Credits past what a Long carries.
            balanceBlob(state = 2, account = 1, credits = -1),
            whole.copyOf(whole.size - 1),
            whole + byteArrayOf(0),
            ByteArray(0),
        )
        for (blob in malformed) {
            assertThrows(DashSdkError.SerializationError::class.java) {
                decodeShieldedIdentityBalance(blob)
            }
        }
    }
}
