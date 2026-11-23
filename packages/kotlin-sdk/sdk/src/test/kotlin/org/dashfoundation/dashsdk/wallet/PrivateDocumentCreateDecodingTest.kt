package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decoding of what `rs-unified-sdk-jni/src/private_document_create.rs` returns: the recipient
 * keys as flattened `[keyId, purpose]` pairs and a created entry as its commitment followed by
 * the sender key id (u32, little-endian). Mirrors that file's Rust tests on the Kotlin side.
 */
class PrivateDocumentCreateDecodingTest {

    @Test
    fun shouldDecodeTheRecipientKeyPairsInOrder() {
        val keys = PrivateDocumentRecipientKey.fromFlat(longArrayOf(1, 1, 4_294_967_295L, 2))
        assertEquals(
            listOf(
                PrivateDocumentRecipientKey(keyId = 1, purpose = 1),
                PrivateDocumentRecipientKey(keyId = 4_294_967_295L, purpose = 2),
            ),
            keys,
        )
        assertEquals("Encryption", keys[0].purposeName)
        assertEquals("Decryption", keys[1].purposeName)
        assertEquals("Purpose 7", PrivateDocumentRecipientKey(keyId = 0, purpose = 7).purposeName)
    }

    @Test
    fun shouldDecodeNoRecipientKeysAsAnEmptyList() {
        assertTrue(PrivateDocumentRecipientKey.fromFlat(LongArray(0)).isEmpty())
    }

    @Test
    fun shouldRefuseRecipientKeysThatAreNotPairs() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            PrivateDocumentRecipientKey.fromFlat(longArrayOf(1, 1, 2))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            PrivateDocumentRecipientKey.fromFlat(null)
        }
    }

    @Test
    fun shouldDecodeTheCommitmentAndALittleEndianSenderKeyId() {
        val bytes = ByteArray(32) { 0xC3.toByte() } + byteArrayOf(0x04, 0x03, 0x02, 0x01)
        val created = PrivateDocumentEntryCreated.fromBytes(bytes)
        assertArrayEquals(ByteArray(32) { 0xC3.toByte() }, created.commitment)
        assertEquals(0x0102_0304L, created.senderKeyId)
    }

    @Test
    fun shouldReadTheSenderKeyIdAsUnsigned() {
        val bytes = ByteArray(32) + byteArrayOf(-1, -1, -1, -1)
        assertEquals(4_294_967_295L, PrivateDocumentEntryCreated.fromBytes(bytes).senderKeyId)
    }

    @Test
    fun shouldRefuseACreatedEntryOfAnotherLength() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            PrivateDocumentEntryCreated.fromBytes(ByteArray(35))
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            PrivateDocumentEntryCreated.fromBytes(null)
        }
    }

    @Test
    fun shouldSendTheAuthModesAsTheFfiBytesAndOfferNoSignedOne() {
        assertEquals(0, PrivateDocumentSenderAuth.ANONYMOUS.ffiValue)
        assertEquals(1, PrivateDocumentSenderAuth.DENIABLE_MAC.ffiValue)
        assertEquals(2, PrivateDocumentSenderAuth.entries.size)
    }
}
