package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.identity.IdentityCreationKeyLayout
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CreateIdentityFundingPolicyTest {

    /** Rust-computed minimums as the screen receives them (values arbitrary). */
    private val minimums = CreateIdentityFundingMinimums(
        coreFundingDuffs = 247_500L,
        addressFundingFeeCredits = listOf(41_500_000L, 42_000_000L),
    )

    @Test
    fun `fresh Core funding rejects amounts below the Rust minimum`() {
        assertFalse(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                247_499L,
                minimums,
            ),
        )
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.CoreBalance,
                247_500L,
                minimums,
            ),
        )
    }

    @Test
    fun `fundings that need the Rust minimum are invalid until it loads`() {
        for (source in listOf(
            CreateIdentityFundingSource.CoreBalance,
            CreateIdentityFundingSource.PlatformAddress,
        )) {
            assertTrue(source.needsFundingMinimums)
            assertFalse(isCreateIdentityFundingAmountValid(source, 10_000_000L, null))
        }
        assertFalse(CreateIdentityFundingSource.ShieldedBalance.needsFundingMinimums)
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.ShieldedBalance,
                10_000_000L,
                null,
            ),
        )
    }

    @Test
    fun `resume has no new funding amount to validate`() {
        assertTrue(
            isCreateIdentityFundingAmountValid(
                CreateIdentityFundingSource.AssetLockResume,
                null,
                null,
            ),
        )
    }

    @Test
    fun `resume re-derives the base layout while fresh funding gets the DashPay layout`() {
        assertEquals(
            IdentityCreationKeyLayout.NEW_IDENTITY,
            CreateIdentityFundingSource.AssetLockResume.keyLayout,
        )
        assertEquals(null, CreateIdentityFundingSource.AssetLockResume.freshKeyLayout)
        for (source in listOf(
            CreateIdentityFundingSource.CoreBalance,
            CreateIdentityFundingSource.PlatformAddress,
            CreateIdentityFundingSource.ShieldedBalance,
        )) {
            assertEquals(IdentityCreationKeyLayout.NEW_IDENTITY_WITH_DASHPAY_KEYS, source.keyLayout)
            assertEquals(source.keyLayout, source.freshKeyLayout)
        }
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
            addressFundingFeeCredits = minimums.addressFundingFeeCredits,
        )

        // One input needs a 41.5M-credit fee left after the requested
        // 30M spend. This address would leave only 40M, so submitting it
        // would persist all keys and then fail on-chain.
        assertTrue(inputs.isEmpty())
    }

    @Test
    fun `Platform address packing never exceeds the Rust maximum input count`() {
        val inputs = packFundingInputs(
            candidates = listOf(
                fundingInput(hashByte = 1, credits = 41_500_010L),
                fundingInput(hashByte = 2, credits = 10L),
                fundingInput(hashByte = 3, credits = 10L),
            ),
            target = 30L,
            addressFundingFeeCredits = listOf(41_500_000L),
        )

        // Thirty credits need three inputs, but Rust allows only one.
        assertTrue(inputs.isEmpty())
    }

    @Test
    fun `Platform address packing accepts the exact one-input fee boundary`() {
        val inputs = packFundingInputs(
            candidates = listOf(fundingInput(hashByte = 1, credits = 71_500_000L)),
            target = 30_000_000L,
            addressFundingFeeCredits = minimums.addressFundingFeeCredits,
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
            addressFundingFeeCredits = minimums.addressFundingFeeCredits,
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
        assertEquals(null, snapshot.freshKeyLayout)
        assertEquals(IdentityCreationKeyLayout.NEW_IDENTITY, snapshot.keyLayout)
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
