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
        assertEquals(0, parsed.userFeeIncrease)
        assertTrue(parsed.complete)
        assertNull("described kinds carry no common details", parsed.details)
        assertEquals("IdentityUpdate variant tag", 6, parsed.serialized.first().toInt())

        val update = parsed.kind as ParsedStateTransitionKind.IdentityUpdate
        assertArrayEquals(ByteArray(32) { 0x11 }, update.identityId)
        assertEquals(listOf(4), update.disablePublicKeyIds)
        assertEquals(2, update.addPublicKeys.size)

        // The document-type-bound key: its bounds carry a variable-length
        // name before the limits flags, so the fields after it are pinned.
        val bound = update.addPublicKeys[1]
        assertEquals(19, bound.keyId)
        assertEquals(KeyPurpose.ENCRYPTION, bound.purpose)
        assertEquals(SecurityLevel.MEDIUM, bound.securityLevel)
        assertTrue(bound.readOnly)
        assertEquals(
            ContractBounds.SingleContractDocumentType(ByteArray(32) { 0x44 }, "profile"),
            bound.contractBounds,
        )
        assertNull(bound.totalBudget)
        assertNull(bound.expiresAt)

        val session = update.addPublicKeys[0]
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
                    tokenCount = null,
                    complete = true,
                    details = null,
                ),
            ),
            batch.transitions,
        )
    }

    @Test
    fun `decodes a mixed document and token batch with completeness and details`() {
        val parsed = StateTransitionParser.parseBlob(golden("parsed_mixed_batch_v1.bin"))

        assertEquals("DocumentsBatch([Create, Transfer, TokenDirectPurchase])", parsed.kindName)
        assertEquals(7, parsed.userFeeIncrease)
        assertFalse(parsed.complete)
        assertNull(parsed.details)

        val batch = parsed.kind as ParsedStateTransitionKind.Batch
        assertEquals(3, batch.transitions.size)

        // Document create: incomplete, its data rendered in details; the
        // fields after the two variable-length strings still line up.
        val create = batch.transitions[0] as ParsedBatchedTransition.Document
        assertEquals("Create", create.action)
        assertEquals("post", create.documentType)
        assertArrayEquals(ByteArray(32) { 0x0D }, create.documentId)
        assertNull(create.amount)
        assertNull(create.recipientId)
        assertFalse(create.complete)
        assertTrue(create.details!!.contains("\"message\""))

        assertEquals(
            ParsedBatchedTransition.Document(
                dataContractId = ByteArray(32) { 0x42 },
                documentType = "profile",
                documentId = ByteArray(32) { 0x0D },
                action = "Transfer",
                amount = null,
                recipientId = ByteArray(32) { 0x22 },
                complete = true,
                details = null,
            ),
            batch.transitions[1],
        )
        assertEquals(
            ParsedBatchedTransition.Token(
                dataContractId = ByteArray(32) { 0x42 },
                tokenId = ByteArray(32) { 0x77 },
                tokenContractPosition = 3,
                action = "DirectPurchase",
                amount = 100_000_000L,
                recipientId = null,
                tokenCount = 100L,
                complete = true,
                details = null,
            ),
            batch.transitions[2],
        )
    }

    /**
     * [StateTransitionParser.parse] is a handle-free utility and may be the
     * first SDK call in a process. Without the native library it must fail
     * inside [org.dashfoundation.dashsdk.Sdk.initialize] (the load step) and
     * not with an [UnsatisfiedLinkError] from the external fun itself, which
     * is what a forgotten initialize looks like. This host JVM has no
     * `libdash_sdk_jni.so`, so the load is what fails here; on a device the
     * same call path loads the library and parses.
     */
    @Test
    fun `parse initializes the native library before entering JNI`() {
        val error = assertThrows(Throwable::class.java) {
            StateTransitionParser.parse(byteArrayOf(0x07, 0x00))
        }
        // System.loadLibrary failing is an UnsatisfiedLinkError whose message
        // names the library; a missing native method names the method.
        assertTrue(
            "expected the library load to fail, got ${error::class.simpleName}: ${error.message}",
            error is UnsatisfiedLinkError && error.message?.contains("dash_sdk_jni") == true,
        )
        assertFalse(
            "parse reached the external fun before loading the library",
            error.message?.contains("parseStateTransition") == true,
        )
    }

    @Test
    fun `decodes the other kind with the common fields and details`() {
        // kind 255, name "MasternodeVote", no owner, signed, fee increase 3,
        // incomplete, empty serialized, details "vote".
        val name = "MasternodeVote".toByteArray()
        val details = "vote".toByteArray()
        val blob = byteArrayOf(0xFF.toByte(), 0, 0, 0, name.size.toByte()) + name +
            byteArrayOf(0, 1, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, details.size.toByte()) + details
        val parsed = StateTransitionParser.parseBlob(blob)

        assertEquals("MasternodeVote", parsed.kindName)
        assertNull(parsed.ownerId)
        assertTrue(parsed.isSigned)
        assertEquals(3, parsed.userFeeIncrease)
        assertFalse(parsed.complete)
        assertEquals(0, parsed.serialized.size)
        assertEquals("vote", parsed.details)
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
        // kind 77, u32 name, no owner, unsigned, fee 0, incomplete, empty
        // serialized, empty details.
        val blob = byteArrayOf(77, 0, 0, 0, name.size.toByte()) + name +
            byteArrayOf(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        val error = assertThrows(IllegalArgumentException::class.java) {
            StateTransitionParser.parseBlob(blob)
        }
        assertTrue(error.message!!.contains("unknown kind"))
    }
}
