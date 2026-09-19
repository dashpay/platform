package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Host-JVM tests of [StateTransitionParser.parseBlob] — the Kotlin half of
 * the JNI decode contract. The golden blobs under `resources/golden/` are
 * the EXACT bytes the Rust side produces and pins in
 * `rs-unified-sdk-jni/src/parse_state_transition.rs`
 * (`*_blob_round_trips_and_is_pinned_for_kotlin`, which `include_bytes!`
 * these same files), so the layout is verified from both ends without
 * loading the native library.
 */
class StateTransitionParserTest {

    private fun golden(name: String): ByteArray =
        javaClass.getResourceAsStream("/golden/$name")
            .use { requireNotNull(it) { "golden fixture $name missing" }.readBytes() }

    @Test
    fun `decodes an identity update registering a limited group-bound session key`() {
        val parsed = StateTransitionParser.parseBlob(golden("parsed_identity_update_v1.bin"))

        assertEquals("IdentityUpdate", parsed.kindName)
        assertArrayEquals(ByteArray(32) { 0x11 }, parsed.ownerId)
        assertFalse(parsed.isSigned)
        assertEquals("IdentityUpdate variant tag", 6, parsed.serialized.first().toInt())

        val update = parsed.kind as ParsedStateTransitionKind.IdentityUpdate
        assertArrayEquals(ByteArray(32) { 0x11 }, update.identityId)
        assertEquals(listOf(4), update.disablePublicKeyIds)
        assertEquals(1, update.addPublicKeys.size)

        val session = update.addPublicKeys.single()
        assertEquals(18, session.keyId)
        assertEquals(KeyType.ECDSA_SECP256K1, session.keyType)
        assertEquals(KeyPurpose.AUTHENTICATION, session.purpose)
        assertEquals(SecurityLevel.HIGH, session.securityLevel)
        assertFalse(session.readOnly)
        assertArrayEquals(ByteArray(33) { 0x03 }, session.pubkeyBytes)
        assertEquals(ContractBounds.ContractGroup(ByteArray(32) { 0x66 }), session.contractBounds)
        assertEquals(10_000_000_000L, session.totalBudget)
        assertEquals(1_800_000_000_000L, session.expiresAt)
        assertTrue(session.hasLimits)
    }

    @Test
    fun `decodes a batch carrying a token transfer`() {
        val parsed = StateTransitionParser.parseBlob(golden("parsed_token_transfer_batch_v1.bin"))

        assertEquals("DocumentsBatch([TokenTransfer])", parsed.kindName)
        assertArrayEquals(ByteArray(32) { 0x21 }, parsed.ownerId)
        assertFalse(parsed.isSigned)
        assertEquals("Batch variant tag", 2, parsed.serialized.first().toInt())

        val batch = parsed.kind as ParsedStateTransitionKind.Batch
        assertArrayEquals(ByteArray(32) { 0x21 }, batch.ownerId)
        assertEquals(
            listOf<ParsedBatchedTransition>(
                ParsedBatchedTransition.Token(
                    dataContractId = ByteArray(32) { 0x42 },
                    tokenId = ByteArray(32) { 0x77 },
                    tokenContractPosition = 3,
                    action = "Transfer",
                    amount = 250L,
                    recipientId = ByteArray(32) { 0x22 },
                ),
            ),
            batch.transitions,
        )
    }

    @Test
    fun `decodes the other kind with only the common fields`() {
        // kind 255, name "MasternodeVote", no owner, signed, empty serialized.
        val name = "MasternodeVote".toByteArray()
        val blob = byteArrayOf(0xFF.toByte(), 0, name.size.toByte()) + name +
            byteArrayOf(0, 1, 0, 0, 0, 0)
        val parsed = StateTransitionParser.parseBlob(blob)

        assertEquals("MasternodeVote", parsed.kindName)
        assertNull(parsed.ownerId)
        assertTrue(parsed.isSigned)
        assertEquals(0, parsed.serialized.size)
        assertTrue(parsed.kind is ParsedStateTransitionKind.Other)
    }

    @Test
    fun `truncated and trailing-garbage blobs throw`() {
        val golden = golden("parsed_token_transfer_batch_v1.bin")
        assertThrows(RuntimeException::class.java) {
            StateTransitionParser.parseBlob(golden.copyOf(golden.size - 5))
        }
        val error = assertThrows(IllegalArgumentException::class.java) {
            StateTransitionParser.parseBlob(golden + byteArrayOf(0))
        }
        assertTrue(error.message!!.contains("trailing"))
    }

    @Test
    fun `unknown kind tag throws rather than decoding as other`() {
        val name = "Whatever".toByteArray()
        val blob = byteArrayOf(77, 0, name.size.toByte()) + name + byteArrayOf(0, 0, 0, 0, 0, 0)
        val error = assertThrows(IllegalArgumentException::class.java) {
            StateTransitionParser.parseBlob(blob)
        }
        assertTrue(error.message!!.contains("unknown kind"))
    }
}
