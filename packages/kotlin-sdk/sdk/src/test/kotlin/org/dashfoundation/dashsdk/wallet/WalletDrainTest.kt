package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.PaymentDelivery
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.UUID

/**
 * Decoding of what `WalletDrainNative` returns, with no native library: the
 * layouts are pinned on the Rust side by
 * `rs-unified-sdk-jni/src/wallet_drain.rs`'s `pack_outcome` / `pack_preview`
 * tests, and here against the same bytes. Mirrors the decoding cases of
 * `packages/swift-sdk/SwiftTests/SwiftDashSDKTests/WalletDrainTests.swift`.
 */
class WalletDrainTest {

    /** A packed outcome as `pack_outcome` lays it out. */
    private fun packedOutcome(
        state: Int,
        hasOutpoint: Boolean,
        txid: ByteArray = ByteArray(32),
        vout: Int = 0,
        errorCode: Int = 0,
        consensusCode: Int = 0,
        consensusKind: Int = 0,
        paymentDelivery: Int = 0,
        retryable: Boolean = false,
        heldAssetLock: ByteArray? = null,
        message: String = "",
    ): ByteArray {
        val text = message.toByteArray(Charsets.UTF_8)
        return ByteBuffer.allocate(WalletDrainOutcome.HEADER_SIZE + text.size)
            .order(ByteOrder.LITTLE_ENDIAN)
            .put(state.toByte())
            .put(if (hasOutpoint) 1 else 0)
            .put(txid)
            .putInt(vout)
            .putInt(errorCode)
            .putInt(consensusCode)
            .putInt(consensusKind)
            .putInt(paymentDelivery)
            .put(if (retryable) 1 else 0)
            .put(if (heldAssetLock != null) 1 else 0)
            .put(heldAssetLock ?: ByteArray(36))
            .put(text)
            .array()
    }

    @Test
    fun shouldDecodeAPendingOutcomeWithItsOutpointAndTheErrorBesideIt() {
        val txid = ByteArray(32) { (it + 1).toByte() }
        // 1020: ErrorTransactionBroadcastUnconfirmed (20) in the wallet range.
        val outcome = WalletDrainOutcome.decode(
            packedOutcome(
                state = 1,
                hasOutpoint = true,
                txid = txid,
                vout = 3,
                errorCode = 1020,
                message = "résumé: unresolved",
            ),
        )

        assertEquals(WalletDrainOutcome.State.Pending, outcome.state)
        assertEquals(WalletDrainOutpoint(txid, 3), outcome.outpoint)
        assertArrayEquals(txid, outcome.outpoint!!.txidWire)
        assertTrue(
            "the error keeps its type",
            outcome.error is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed,
        )
        // The typed error appends its own do-not-retry notice to Rust's text.
        assertTrue(
            "the message is read as UTF-8: ${outcome.error!!.message}",
            outcome.error!!.message!!.startsWith("résumé: unresolved"),
        )
    }

    /**
     * A failed drain's error keeps Rust's payment class, as a thrown
     * exception would: its delivery, the wallet's retry and the held lock.
     */
    @Test
    fun shouldKeepThePaymentClassOfTheErrorBesideTheOutcome() {
        val held = ByteArray(36) { if (it < 32) 5 else 0 }.also { it[32] = 2 }
        // 1006: ErrorWalletOperation (6); delivery 3: LockHeld.
        val lockHeld = WalletDrainOutcome.decode(
            packedOutcome(
                state = 1,
                hasOutpoint = true,
                errorCode = 1006,
                paymentDelivery = 3,
                retryable = true,
                heldAssetLock = held,
                message = "proof wait timed out",
            ),
        ).error!!
        assertEquals(PaymentDelivery.LOCK_HELD, lockHeld.delivery)
        assertTrue(lockHeld.isRetryable)
        assertEquals(AssetLockBuiltOutPoint.fromRawBytes(held), lockHeld.heldAssetLock)

        // delivery 2: MaybeSent; no held lock, no retry.
        val maybeSent = WalletDrainOutcome.decode(
            packedOutcome(state = 1, hasOutpoint = true, errorCode = 1006, paymentDelivery = 2),
        ).error!!
        assertEquals(PaymentDelivery.MAYBE_SENT, maybeSent.delivery)
        assertFalse(maybeSent.isRetryable)
        assertNull(maybeSent.heldAssetLock)
    }

    @Test
    fun shouldDecodeACompletedOutcomeThatAlsoCarriesAnError() {
        // A repeated operation id reports "already consumed" (24) beside Completed.
        val outcome = WalletDrainOutcome.decode(
            packedOutcome(state = 2, hasOutpoint = true, vout = 0, errorCode = 1024, message = "used"),
        )

        assertEquals(WalletDrainOutcome.State.Completed, outcome.state)
        assertNotNull(outcome.outpoint)
        assertTrue(outcome.error is DashSdkError.PlatformWallet.AssetLockAlreadyConsumed)
    }

    @Test
    fun shouldDecodeASuccessfulOutcomeWithNoOutpointAndNoError() {
        val outcome = WalletDrainOutcome.decode(packedOutcome(state = 0, hasOutpoint = false))

        assertEquals(WalletDrainOutcome.State.NotSent, outcome.state)
        assertNull(outcome.outpoint)
        assertNull(outcome.error)
    }

    @Test
    fun shouldKeepAnOutcomeItCannotReadPendingAndNeverThrow() {
        val unknownState = WalletDrainOutcome.decode(
            packedOutcome(state = 9, hasOutpoint = true, vout = 1),
        )
        assertEquals(WalletDrainOutcome.State.Pending, unknownState.state)
        assertNotNull("a readable outpoint survives", unknownState.outpoint)
        assertTrue(unknownState.error is DashSdkError.SerializationError)

        val truncated = WalletDrainOutcome.decode(ByteArray(WalletDrainOutcome.HEADER_SIZE - 1))
        assertEquals(WalletDrainOutcome.State.Pending, truncated.state)
        assertNull(truncated.outpoint)
        assertTrue(truncated.error is DashSdkError.SerializationError)

        // A "completed" whose outpoint cannot be used is not reported as completed.
        val badVout = WalletDrainOutcome.decode(
            packedOutcome(state = 2, hasOutpoint = true, vout = -1),
        )
        assertEquals(WalletDrainOutcome.State.Pending, badVout.state)
        assertNull(badVout.outpoint)
        assertTrue(badVout.error is DashSdkError.SerializationError)
    }

    @Test
    fun shouldReportAnUnresolvedCallAsPendingWithoutAnOutpoint() {
        val error = DashSdkError.InvalidState("closed")
        val outcome = WalletDrainOutcome.unresolved(error)

        assertEquals(WalletDrainOutcome.State.Pending, outcome.state)
        assertNull(outcome.outpoint)
        assertSame(error, outcome.error)
    }

    @Test
    fun shouldDecodeAPreviewAndItsAbsentEstimates() {
        val ready = WalletDrainPreview.decode(
            longArrayOf(0, 4, 31_200_000, 100_000_000, 1, 985, 1, 31_099_015_000),
        )
        assertEquals(
            WalletDrainPreview(
                candidateInputCount = 4,
                candidateBalanceDuffs = 31_200_000,
                poolFeeCredits = 100_000_000,
                estimatedL1FeeDuffs = 985,
                estimatedRecipientCredits = 31_099_015_000,
                status = WalletDrainPreview.Status.Ready,
            ),
            ready,
        )

        // The flags decide, not the value slots beside them.
        val limited = WalletDrainPreview.decode(longArrayOf(2, 101, 1_000_000, 100_000, 0, 77, 0, 88))
        assertEquals(WalletDrainPreview.Status.InputLimitMayBeExceeded, limited.status)
        assertNull(limited.estimatedL1FeeDuffs)
        assertNull(limited.estimatedRecipientCredits)
    }

    @Test
    fun shouldRefuseAMalformedPreview() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            WalletDrainPreview.decode(longArrayOf(0, 1, 2))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            WalletDrainPreview.decode(longArrayOf(3, 0, 0, 0, 0, 0, 0, 0))
        }
        // An unsigned amount past Long.MAX_VALUE arrives negative.
        assertThrows(DashSdkError.SerializationError::class.java) {
            WalletDrainPreview.decode(longArrayOf(0, 1, -1, 0, 0, 0, 0, 0))
        }
    }

    @Test
    fun shouldEncodeAnOperationIdAsTheUuidsBytesMostSignificantFirst() {
        val uuid = UUID.fromString("00112233-4455-6677-8899-aabbccddeeff")
        val bytes = uuid.toWalletDrainOperationIdBytes()

        // The same 16 bytes Swift's `UUID.uuid` holds for this string.
        assertArrayEquals(
            byteArrayOf(
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                0x88.toByte(), 0x99.toByte(), 0xaa.toByte(), 0xbb.toByte(),
                0xcc.toByte(), 0xdd.toByte(), 0xee.toByte(), 0xff.toByte(),
            ),
            bytes,
        )
        assertEquals(uuid, walletDrainOperationIdFromBytes(bytes))
        assertNull(walletDrainOperationIdFromBytes(ByteArray(15)))
        assertNull(walletDrainOperationIdFromBytes(ByteArray(17)))
    }

    /**
     * The host's own fact: whether one of its drain calls is active. What a
     * status read reports while one is, is Rust's rule (instrumented
     * `WalletDrainFfiTest`).
     */
    @Test
    fun shouldCountActiveCallsPerManagerWalletAndOperationId() {
        val walletId = ByteArray(32) { 3 }
        val id = UUID.randomUUID()
        assertTrue(!WalletDrainActiveCalls.isActive(7L, walletId, id))
        WalletDrainActiveCalls.enter(7L, walletId, id)
        WalletDrainActiveCalls.enter(7L, walletId, id)
        WalletDrainActiveCalls.leave(7L, walletId, id)
        assertTrue("one returning call does not clear another's", WalletDrainActiveCalls.isActive(7L, walletId, id))
        assertTrue("another manager's call is not this one's", !WalletDrainActiveCalls.isActive(8L, walletId, id))
        WalletDrainActiveCalls.leave(7L, walletId, id)
        assertTrue(!WalletDrainActiveCalls.isActive(7L, walletId, id))
    }

    @Test
    fun shouldCompareOutpointsByValueAndPrintTheDisplayTxid() {
        val txid = ByteArray(32) { it.toByte() }
        val outpoint = WalletDrainOutpoint(txid, 2)
        txid[0] = 0x7f
        assertEquals("the outpoint copied its bytes", 0, outpoint.txidWire[0].toInt())
        assertEquals(WalletDrainOutpoint(ByteArray(32) { it.toByte() }, 2), outpoint)
        assertTrue(outpoint.toString().startsWith("1f1e1d"))
        assertTrue(outpoint.toString().endsWith(":2"))
        assertThrows(IllegalArgumentException::class.java) { WalletDrainOutpoint(ByteArray(31), 0) }
    }
}
