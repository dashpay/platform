package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.TrackedAssetLockNativeData
import org.dashfoundation.dashsdk.ffi.TrackedAssetLocksNativeResult
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TrackedAssetLockTest {

    @Test
    fun `native snapshot filters to generic funding types and resumable statuses`() {
        fun row(type: Int, status: Int, txidSize: Int = 32) = TrackedAssetLockNativeData(
            outpointTxid = ByteArray(txidSize) { type.toByte() },
            outpointVout = type,
            fundingType = type,
            status = status.toByte(),
            registrationIndex = 7,
            instantLockPresent = status >= 2,
            chainLockHeight = if (status == 3) 500 else 0,
            identityRecipient = null,
            identityRecipientUnassigned = false,
        )
        val native = TrackedAssetLocksNativeResult(
            arrayOf(
                row(0, 0), row(1, 1), row(2, 3),
                row(3, 2), // invitation: never generic
                row(4, 2), // address top-up
                row(0, 4), // consumed: terminal tombstone, never recoverable
                row(1, 5), // recovered from chain: user-driven recovery allowed
                row(0, 2, txidSize = 31),
            ),
        )

        val eligible = TrackedAssetLock.eligibleFromNative(native)

        assertEquals(listOf(0, 1, 2, 1), eligible.map { it.fundingType.raw })
        assertEquals(listOf(0, 1, 3, 5), eligible.map { it.status.raw })
        assertEquals(
            TrackedAssetLock.Status.RECOVERED_FROM_CHAIN,
            eligible.last().status,
        )
        assertTrue(eligible.all { it.outpointTxid.size == 32 })
        // Mapper owns its Kotlin copy; callers can't mutate the JNI row.
        native.entries[0].outpointTxid[0] = 99
        assertEquals(0, eligible[0].outpointTxid[0].toInt())
    }

    @Test
    fun shouldCarryTheRecordedIdentityRecipientAndTheUnassignedFlag() {
        fun row(vout: Int, recipient: ByteArray?, unassigned: Boolean) = TrackedAssetLockNativeData(
            outpointTxid = ByteArray(32) { 1 },
            outpointVout = vout,
            fundingType = TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND.raw,
            status = 3,
            registrationIndex = 0,
            instantLockPresent = true,
            chainLockHeight = 500,
            identityRecipient = recipient,
            identityRecipientUnassigned = unassigned,
        )
        val recipient = ByteArray(32) { 0x2A }
        val native = TrackedAssetLocksNativeResult(
            arrayOf(
                row(0, recipient, unassigned = false),
                row(1, null, unassigned = true),
                row(2, ByteArray(31), unassigned = false), // malformed: dropped
            ),
        )

        val eligible = TrackedAssetLock.eligibleFromNative(native)

        assertEquals(listOf(0, 1), eligible.map { it.outpointVout })
        assertArrayEquals(recipient, eligible[0].identityRecipient)
        assertFalse(eligible[0].identityRecipientUnassigned)
        assertNull(eligible[1].identityRecipient)
        assertTrue(eligible[1].identityRecipientUnassigned)
        // The model owns its copy of the recipient too.
        recipient[0] = 0
        assertEquals(0x2A, eligible[0].identityRecipient?.get(0)?.toInt())
    }
}
