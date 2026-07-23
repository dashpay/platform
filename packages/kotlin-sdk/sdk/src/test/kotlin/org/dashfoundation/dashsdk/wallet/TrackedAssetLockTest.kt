package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.TrackedAssetLockNativeData
import org.dashfoundation.dashsdk.ffi.TrackedAssetLocksNativeResult
import org.junit.Assert.assertEquals
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
        )
        val native = TrackedAssetLocksNativeResult(
            arrayOf(
                row(0, 0), row(1, 1), row(2, 3),
                row(3, 2), // invitation: never generic
                row(4, 2), // address top-up
                row(0, 4), // consumed
                row(0, 2, txidSize = 31),
            ),
        )

        val eligible = TrackedAssetLock.eligibleFromNative(native)

        assertEquals(listOf(0, 1, 2), eligible.map { it.fundingType.raw })
        assertEquals(listOf(0, 1, 3), eligible.map { it.status.raw })
        assertTrue(eligible.all { it.outpointTxid.size == 32 })
        // Mapper owns its Kotlin copy; callers can't mutate the JNI row.
        native.entries[0].outpointTxid[0] = 99
        assertEquals(0, eligible[0].outpointTxid[0].toInt())
    }
}
