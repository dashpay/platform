package org.dashfoundation.dashsdk.dpns

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Decoding of the JSON `rs-unified-sdk-jni/src/dpns_give_up.rs` copies the
 * give-up results into. Mirrors Swift's
 * `DpnsMarketplaceDecodingTests.testUnknownOrInconsistentGivenUpResultsFailClosed`.
 */
@RunWith(RobolectricTestRunner::class)
class DpnsNameGiveUpTest {
    private val documentHex = "0a".repeat(32)
    private val tombstoneHex = "0b".repeat(32)

    @Test
    fun shouldDecodeTheChoicesTheNetworkAccepts() {
        val choices = DpnsNameGiveUpCodec.decodeChoices(
            """{"documentId":"$documentHex","canDelete":false,"canTombstone":true}""",
        )
        assertArrayEquals(ByteArray(32) { 0x0a }, choices.documentId)
        assertFalse(choices.canDelete)
        assertTrue(choices.canTombstone)
    }

    @Test
    fun shouldDecodeADeleteAndATombstoneWithItsId() {
        val deleted = DpnsNameGiveUpCodec.decodeGivenUp(
            """{"documentId":"$documentHex","method":0,"tombstoneId":null}""",
        )
        assertEquals(DpnsNameGiveUpMethod.DELETE, deleted.method)
        assertNull(deleted.tombstoneId)

        val tombstoned = DpnsNameGiveUpCodec.decodeGivenUp(
            """{"documentId":"$documentHex","method":1,"tombstoneId":"$tombstoneHex"}""",
        )
        assertEquals(DpnsNameGiveUpMethod.TOMBSTONE, tombstoned.method)
        assertArrayEquals(ByteArray(32) { 0x0b }, tombstoned.tombstoneId)
        assertArrayEquals(ByteArray(32) { 0x0a }, tombstoned.documentId)
    }

    @Test
    fun shouldFailClosedOnAnUnknownOrInconsistentGivenUpResult() {
        listOf(
            """{"documentId":"$documentHex","method":7,"tombstoneId":null}""",
            """{"documentId":"$documentHex","method":1,"tombstoneId":null}""",
            """{"documentId":"$documentHex","method":0,"tombstoneId":"$tombstoneHex"}""",
        ).forEach { json ->
            assertThrows(DashSdkError.PlatformWallet.WalletOperation::class.java) {
                DpnsNameGiveUpCodec.decodeGivenUp(json)
            }
        }
    }

    @Test
    fun shouldCrossTheMethodAsTheFfiByte() {
        assertEquals(0, DpnsNameGiveUpMethod.DELETE.ffiValue)
        assertEquals(1, DpnsNameGiveUpMethod.TOMBSTONE.ffiValue)
        assertNull(DpnsNameGiveUpMethod.fromFfi(2))
    }
}
