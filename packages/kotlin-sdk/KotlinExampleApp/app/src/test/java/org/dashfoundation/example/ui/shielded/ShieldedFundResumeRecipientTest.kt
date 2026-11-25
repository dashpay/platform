package org.dashfoundation.example.ui.shielded

import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Which recipient the Core to shielded screen pays when it resumes a lock.
 * A lock restored after a restart with a recorded recipient pays that
 * recipient, even when it differs from the wallet's default and when an
 * address was pasted: Rust refuses any other. Port of the Swift view's
 * `lockBoundRecipient` in `ShieldedFundFromAssetLockView.swift`.
 */
class ShieldedFundResumeRecipientTest {

    private val recorded = ByteArray(43) { 0x21 }
    private val walletDefault = ByteArray(43) { 0x42 }
    private val pasted = ByteArray(43) { 0x63 }

    private fun lock(recipient: ByteArray?) = AssetLockEntity(
        outPointHex = "aa:0",
        walletId = ByteArray(32) { 1 },
        transactionBytes = ByteArray(20) { 2 },
        fundingTypeRaw = 5,
        identityIndexRaw = 0,
        amountDuffs = 100_000,
        statusRaw = 1,
        shieldedRecipientRaw43 = recipient,
    )

    @Test
    fun `should pay the recorded recipient of a restored lock, not the wallet default`() {
        val bound = lockBoundRecipient(lock(recorded))
        assertArrayEquals(recorded, bound)
        assertArrayEquals(recorded, effectiveShieldRecipient(bound, null, walletDefault))
    }

    @Test
    fun `should ignore a pasted address for a lock with a recorded recipient`() {
        val bound = lockBoundRecipient(lock(recorded))
        assertArrayEquals(recorded, effectiveShieldRecipient(bound, pasted, walletDefault))
    }

    @Test
    fun `should keep the picker for a lock tracked before recipients were recorded`() {
        val bound = lockBoundRecipient(lock(null))
        assertNull(bound)
        assertArrayEquals(walletDefault, effectiveShieldRecipient(bound, null, walletDefault))
        assertArrayEquals(pasted, effectiveShieldRecipient(bound, pasted, walletDefault))
    }

    @Test
    fun `should not bind a lock to a stored value that is not an address`() {
        assertNull(lockBoundRecipient(lock(ByteArray(42))))
        assertNull(lockBoundRecipient(null))
    }
}
