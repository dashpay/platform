package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CreateIdentityFundingPolicyTest {

    @Test
    fun `Core minimum scales with the number of registration keys`() {
        assertEquals(228_000L, minimumCoreFundingDuffsForKeyCount(4))
        assertEquals(241_000L, minimumCoreFundingDuffsForKeyCount(6))
    }

    @Test
    fun `fresh six-key Core funding rejects amounts below the protocol floor`() {
        assertFalse(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                240_999L,
            ),
        )
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                241_000L,
            ),
        )
    }

    @Test
    fun `resume has no new funding amount to validate`() {
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.AssetLockResume,
                null,
            ),
        )
    }

    @Test
    fun `Platform address packing reserves the six-key fee on input zero`() {
        val inputs = packFundingInputs(
            candidates = listOf(
                FundingInput(
                    addressType = 0,
                    hash = ByteArray(20) { 1 },
                    credits = 70_000_000L,
                ),
            ),
            target = 30_000_000L,
        )

        // One six-key input needs 41.5M credits left after the requested
        // 30M spend. This address would leave only 40M, so submitting it
        // would persist all keys and then fail on-chain.
        assertTrue(inputs.isEmpty())
    }

    @Test
    fun `Platform address fee scales with key and input counts`() {
        assertEquals(41_500_000L, minimumPlatformAddressFundingFeeCredits(6, 1))
        assertEquals(42_000_000L, minimumPlatformAddressFundingFeeCredits(6, 2))
    }

    @Test
    fun `Platform address packing accepts the exact one-input fee boundary`() {
        val inputs = packFundingInputs(
            candidates = listOf(fundingInput(hashByte = 1, credits = 71_500_000L)),
            target = 30_000_000L,
            keyCount = 6,
        )

        assertEquals(1, inputs.size)
        assertEquals(30_000_000L, inputs.single().credits)
    }

    @Test
    fun `Platform address packing leaves the two-input fee on BTreeMap input zero`() {
        val lowerAddress = fundingInput(hashByte = 1, credits = 42_000_001L)
        val higherAddress = fundingInput(hashByte = 2, credits = 29_999_999L)
        val inputs = packFundingInputs(
            // Deliberately reverse native BTreeMap order.
            candidates = listOf(higherAddress, lowerAddress),
            target = 30_000_000L,
            keyCount = 6,
        )

        assertEquals(2, inputs.size)
        assertEquals(30_000_000L, inputs.sumOf { it.credits })
        val inputZero = inputs.minWith(::compareFundingAddressesForTest)
        val originalBalance = if (inputZero.hash[0] == 1.toByte()) {
            lowerAddress.credits
        } else {
            higherAddress.credits
        }
        assertTrue(originalBalance - inputZero.credits >= 42_000_000L)
    }

    @Test
    fun `submission snapshot is not redirected by later form changes`() {
        val originalTxid = ByteArray(32) { 7 }
        var source = CreateIdentityFundingSource.AssetLockResume
        var selectedLock = registrationLock(registrationIndex = 5, txid = originalTxid)

        val snapshot = createIdentitySubmissionSnapshot(
            fundingSource = source,
            amountText = "999",
            identityIndexText = "42",
            selectedRecoveryLock = selectedLock,
        )!!

        source = CreateIdentityFundingSource.CoreBalance
        selectedLock = registrationLock(registrationIndex = 9, txid = ByteArray(32) { 9 })
        originalTxid.fill(3)

        assertEquals(CreateIdentityFundingSource.AssetLockResume, snapshot.fundingSource)
        assertFalse(snapshot.includesDashPayKeys)
        assertEquals(5, snapshot.identityIndex)
        assertEquals(null, snapshot.amount)
        assertEquals(5, snapshot.recoveryLock!!.registrationIndex)
        assertArrayEquals(ByteArray(32) { 7 }, snapshot.recoveryLock.outpointTxid)
        // Keep assignments observable so the test documents the controls did change.
        assertEquals(CreateIdentityFundingSource.CoreBalance, source)
        assertEquals(9, selectedLock.registrationIndex)
    }

    private fun fundingInput(hashByte: Int, credits: Long) = FundingInput(
        addressType = 0,
        hash = ByteArray(20).also { it[0] = hashByte.toByte() },
        credits = credits,
    )

    private fun compareFundingAddressesForTest(left: FundingInput, right: FundingInput): Int {
        val typeOrder = left.addressType.compareTo(right.addressType)
        if (typeOrder != 0) return typeOrder
        for (index in left.hash.indices) {
            val order = left.hash[index].toUByte().compareTo(right.hash[index].toUByte())
            if (order != 0) return order
        }
        return 0
    }

    private fun registrationLock(registrationIndex: Int, txid: ByteArray) = TrackedAssetLock(
        outpointTxid = txid,
        outpointVout = 0,
        fundingType = TrackedAssetLock.FundingType.IDENTITY_REGISTRATION,
        status = TrackedAssetLock.Status.BUILT,
        registrationIndex = registrationIndex,
        instantLockPresent = false,
        chainLockHeight = 0,
    )
}
