package org.dashfoundation.dashsdk.dpns

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class DpnsMarketplaceTest {
    @Test
    fun decodesMarketplaceNameWithOptionalFields() {
        val name = DpnsMarketplace.decodeName(
            """{"documentId":"${"01".repeat(32)}","ownerId":"${"02".repeat(32)}","recordsIdentityId":null,"label":"Alice","normalizedLabel":"a11ce","priceCredits":5000,"createdAtMs":1,"updatedAtMs":2,"transferredAtMs":0}""",
        )

        assertArrayEquals(ByteArray(32) { 1 }, name.documentId)
        assertArrayEquals(ByteArray(32) { 2 }, name.ownerId)
        assertNull(name.recordsIdentityId)
        assertEquals("Alice", name.label)
        assertEquals(5_000uL, name.priceCredits)
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsUnknownSaleStatus() {
        DpnsMarketplace.decodeStates(
            """[{"documentId":"${"01".repeat(32)}","walletIdentityId":"${"02".repeat(32)}","label":"Alice","normalizedLabel":"a11ce","priceCredits":null,"status":99,"counterpartyId":null,"createdAtMs":0,"updatedAtMs":0,"transferredAtMs":0,"lastSyncedAtMs":0}]""",
        )
    }

    @Test
    fun decodesFullUnsignedPriceRangeLosslessly() {
        val name = DpnsMarketplace.decodeName(
            """{"documentId":"${"01".repeat(32)}","ownerId":"${"02".repeat(32)}","recordsIdentityId":null,"label":"Alice","normalizedLabel":"a11ce","priceCredits":"18446744073709551615","createdAtMs":1,"updatedAtMs":2,"transferredAtMs":0}""",
        )
        assertEquals(ULong.MAX_VALUE, name.priceCredits)

        val summary = DpnsMarketplace.decodeSyncSummary(
            """{"tracked":1,"added":[],"departed":[],"pricesChanged":[{"documentId":"${"03".repeat(32)}","label":"Alice","previousCredits":null,"currentCredits":"18446744073709551615"}],"syncUnixMs":4}""",
        )
        assertNull(summary.pricesChanged.single().previousCredits)
        assertEquals(ULong.MAX_VALUE, summary.pricesChanged.single().currentCredits)
        assertEquals(4L, summary.syncUnixMs)
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsUnknownHistoryKind() {
        DpnsMarketplace.decodeHistory(
            """[{"kind":99,"atMs":0,"blockHeight":null,"priceCredits":null,"fromId":null,"toId":null}]""",
        )
    }
}
