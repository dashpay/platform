package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.ShieldedIdentityDebitRecoveryData
import org.junit.Assert.*
import org.junit.Test

class ShieldedIdentityDebitRecoveryRecordTest {
    @Test
    fun shouldKeepUnreadableFieldsAbsentAndCopyScope() {
        val activity = ByteArray(32) { 7 }
        val record = ShieldedIdentityDebitRecoveryRecord.fromNative(
            ShieldedIdentityDebitRecoveryData(9, activity, null, false, -1L, false, -1L, 1)
        )
        activity[0] = 0
        assertEquals(9u, record.accountIndex)
        assertArrayEquals(ByteArray(32) { 7 }, record.activityId)
        assertNull(record.identityId)
        assertNull(record.nonce)
        assertNull(record.amount)
        assertEquals(ShieldedIdentityDebitRecoveryStatus.PARKED, record.status)
    }

    @Test
    fun shouldPreserveUnsignedValuesAndUnknownOutcome() {
        val record = ShieldedIdentityDebitRecoveryRecord.fromNative(
            ShieldedIdentityDebitRecoveryData(-1, ByteArray(32), ByteArray(32) { 8 }, true, -1L, true, -1L, 2)
        )
        assertEquals(UInt.MAX_VALUE, record.accountIndex)
        assertArrayEquals(ByteArray(32) { 8 }, record.identityId)
        assertEquals(ULong.MAX_VALUE, record.nonce)
        assertEquals(ULong.MAX_VALUE, record.amount)
        assertEquals(ShieldedIdentityDebitRecoveryStatus.UNKNOWN, record.status)
    }

    @Test(expected = IllegalStateException::class)
    fun shouldRejectUnrecognizedRecoveryStatus() {
        ShieldedIdentityDebitRecoveryRecord.fromNative(
            ShieldedIdentityDebitRecoveryData(0, ByteArray(32), null, false, 0, false, 0, 255)
        )
    }
}
