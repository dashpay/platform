package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decodes the JSON `rs-unified-sdk-jni/src/private_documents.rs` writes for one private
 * document read (its Rust tests pin the same shape) into [PrivateDocumentEntriesRead]. The
 * Kotlin side of `PrivateDocumentReadCursorTests.swift`.
 */
class PrivateDocumentEntriesReadTest {

    private fun hex(byte: Int) = "%02x".format(byte).repeat(32)

    private fun bytes(byte: Int) = ByteArray(32) { byte.toByte() }

    private fun entry(
        direction: Int = 0,
        authMode: Int = 0,
        properties: String? = "\"{\\\"note\\\":\\\"hi\\\"}\"",
        refusal: String? = null,
        sender: String? = null,
        senderKey: Int? = null,
    ) = """{"position":7,"block_height":120,"block_time_ms":1700000000000,""" +
        """"properties_json":${properties ?: "null"},"properties_refusal":${refusal ?: "null"},""" +
        """"wallet_identity_id":"${hex(0xa1)}","wallet_key_id":3,""" +
        """"recipient_identity_id":"${hex(0xb2)}",""" +
        """"claimed_sender_identity_id":${sender?.let { "\"$it\"" } ?: "null"},""" +
        """"claimed_sender_key_id":${senderKey ?: "null"},""" +
        """"commitment":"${hex(0xc3)}","direction":$direction,"auth_mode":$authMode}"""

    private fun read(
        entries: String,
        provedTotalCount: String = "null",
        complete: Boolean = false,
        restarted: Boolean = false,
    ) = """{"entries":[$entries],"cursor":{"position":40,"proved_total_count":$provedTotalCount,""" +
        """"key_set":"${hex(0x01)}","store":"${hex(0x02)}"},"pages_read":2,"rejected_count":1,""" +
        """"identity_count":3,"unscanned_key_count":4,"complete":$complete,"restarted":$restarted}"""

    @Test
    fun shouldDecodeAReceivedAnonymousEntryAndTheCounts() {
        val result = PrivateDocumentEntriesRead.fromJson(read(entry()))
        val entry = result.entries.single()
        assertEquals(7L, entry.position)
        assertEquals(120L, entry.blockHeight)
        assertEquals(1_700_000_000_000L, entry.blockTimeMs)
        assertEquals(PrivateDocumentDirection.RECEIVED, entry.direction)
        assertEquals(PrivateDocumentAuthMode.ANONYMOUS, entry.authMode)
        assertArrayEquals(bytes(0xa1), entry.walletIdentityId)
        assertEquals(3L, entry.walletKeyId)
        assertArrayEquals(bytes(0xb2), entry.recipientIdentityId)
        assertArrayEquals(bytes(0xc3), entry.commitment)
        assertNull(entry.claimedSenderIdentityId)
        assertNull(entry.claimedSenderKeyId)
        assertEquals(PrivateDocumentPayload.Properties("{\"note\":\"hi\"}"), entry.payload)

        assertEquals(2L, result.pagesRead)
        assertEquals(1L, result.rejectedCount)
        assertEquals(3L, result.identityCount)
        assertEquals(4L, result.unscannedKeyCount)
        assertFalse(result.complete)
        assertFalse(result.restarted)
    }

    @Test
    fun shouldDecodeASentSignedEntryWithAClaimedSenderAndARefusal() {
        val result = PrivateDocumentEntriesRead.fromJson(
            read(
                entry(
                    direction = 1,
                    authMode = 2,
                    properties = null,
                    refusal = "\"missing note\"",
                    sender = hex(0xd4),
                    senderKey = 9,
                ),
                complete = true,
                restarted = true,
            ),
        )
        val entry = result.entries.single()
        assertEquals(PrivateDocumentDirection.SENT, entry.direction)
        assertEquals(PrivateDocumentAuthMode.SIGNATURE, entry.authMode)
        assertArrayEquals(bytes(0xd4), entry.claimedSenderIdentityId)
        assertEquals(9L, entry.claimedSenderKeyId)
        assertEquals(PrivateDocumentPayload.Refused("missing note"), entry.payload)
        assertTrue(result.complete)
        assertTrue(result.restarted)
    }

    /** The cursor crosses back as it came; an unknown store size stays unknown, not 0. */
    @Test
    fun shouldKeepTheCursorAsItCameWithAnUnknownOrProvedStoreSize() {
        val unknown = PrivateDocumentEntriesRead.fromJson(read("")).cursor
        assertEquals(40L, unknown.position)
        assertNull(unknown.provedTotalCount)
        assertArrayEquals(bytes(0x01), unknown.keySet)
        assertArrayEquals(bytes(0x02), unknown.store)

        val proved = PrivateDocumentEntriesRead.fromJson(read("", provedTotalCount = "55")).cursor
        assertEquals(55L, proved.provedTotalCount)
    }

    @Test
    fun shouldDecodeTheDeniableMacMode() {
        val entry = PrivateDocumentEntriesRead.fromJson(read(entry(authMode = 1))).entries.single()
        assertEquals(PrivateDocumentAuthMode.DENIABLE_MAC, entry.authMode)
    }

    /** As Swift does, the whole result is refused rather than shown in part. */
    @Test
    fun shouldRefuseAnEntryItCannotRead() {
        for (json in listOf(
            read(entry(direction = 2)),
            read(entry(authMode = 3)),
            read(entry(properties = null, refusal = null)),
            read(entry(sender = "abcd")),
            read(""" ${entry()}""").replace(hex(0xc3), "zz".repeat(32)),
            "not json",
        )) {
            assertThrows(json, DashSdkError.SerializationError::class.java) {
                PrivateDocumentEntriesRead.fromJson(json)
            }
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            PrivateDocumentEntriesRead.fromJson(null)
        }
    }
}
