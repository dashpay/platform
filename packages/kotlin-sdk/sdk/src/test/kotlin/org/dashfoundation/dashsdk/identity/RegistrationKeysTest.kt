package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RegistrationKeysTest {

    /** Deterministic fixture pubkey for slot [keyId]: 33 bytes of `keyId + 2`. */
    private fun fixturePubkey(keyId: Int): ByteArray = ByteArray(33) { (keyId + 2).toByte() }

    private fun fixturePubkeys(count: Int): List<ByteArray> = (0 until count).map(::fixturePubkey)

    private fun golden(name: String): ByteArray =
        javaClass.getResourceAsStream("/golden/identity_creation_key_layout_${name}_v1.bin")
            .use { requireNotNull(it) { "golden layout fixture $name missing" }.readBytes() }

    /**
     * The Rust layouts, decoded from the golden BLOBs the Rust JNI encoder is
     * pinned to (`identity::tests::should_encode_*`).
     */
    private val newIdentity get() = RegistrationKeys.decodeLayout(golden("new_identity"))
    private val withDashPayKeys get() = RegistrationKeys.decodeLayout(golden("new_identity_with_dashpay_keys"))

    private fun roles(slots: List<RegistrationKeySlot>) =
        slots.map { Triple(it.keyId, it.purpose, it.securityLevel) }

    @Test
    fun `new identity layout registers the four base keys`() {
        assertEquals(
            listOf(
                Triple(0, KeyPurpose.AUTHENTICATION, SecurityLevel.MASTER),
                Triple(1, KeyPurpose.AUTHENTICATION, SecurityLevel.CRITICAL),
                Triple(2, KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH),
                Triple(3, KeyPurpose.TRANSFER, SecurityLevel.CRITICAL),
            ),
            roles(newIdentity),
        )
        newIdentity.forEach {
            assertEquals(KeyType.ECDSA_SECP256K1, it.keyType)
            assertEquals(it.keyId, it.derivationKeyIndex)
            assertNull(it.contractBounds)
        }
    }

    @Test
    fun `DashPay layout adds the DashPay pair at ids four and five`() {
        val slots = withDashPayKeys
        assertEquals(
            listOf(
                Triple(0, KeyPurpose.AUTHENTICATION, SecurityLevel.MASTER),
                Triple(1, KeyPurpose.AUTHENTICATION, SecurityLevel.CRITICAL),
                Triple(2, KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH),
                Triple(3, KeyPurpose.TRANSFER, SecurityLevel.CRITICAL),
                Triple(4, KeyPurpose.ENCRYPTION, SecurityLevel.MEDIUM),
                Triple(5, KeyPurpose.DECRYPTION, SecurityLevel.MEDIUM),
            ),
            roles(slots),
        )
        for (slot in listOf(slots[4], slots[5])) {
            val bounds = slot.contractBounds
            assertTrue(bounds is ContractBounds.SingleContractDocumentType)
            bounds as ContractBounds.SingleContractDocumentType
            assertArrayEquals(RegistrationKeys.DASHPAY_CONTRACT_ID, bounds.contractId)
            assertEquals(RegistrationKeys.DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE, bounds.documentTypeName)
        }
        assertEquals(roles(newIdentity), roles(slots).take(4))
    }

    @Test
    fun `rows stamp each slot role onto its public key`() {
        val rows = RegistrationKeys.buildRegistrationRows(fixturePubkeys(6), withDashPayKeys)
        assertEquals(listOf(0, 1, 2, 3, 4, 5), rows.map { it.keyId })
        rows.forEachIndexed { i, row ->
            assertArrayEquals(fixturePubkey(i), row.pubkeyBytes)
            assertEquals(withDashPayKeys[i].purpose, row.purpose)
            assertEquals(withDashPayKeys[i].securityLevel, row.securityLevel)
            assertEquals(withDashPayKeys[i].contractBounds, row.contractBounds)
            assertEquals(false, row.readOnly)
        }
    }

    @Test
    fun `wrong key count is rejected`() {
        val failure = runCatching {
            RegistrationKeys.buildRegistrationRows(fixturePubkeys(5), withDashPayKeys)
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
    }

    @Test
    fun `malformed layout blobs are rejected`() {
        val blob = golden("new_identity")
        for (bad in listOf(
            blob.copyOf(blob.size - 1), // truncated
            blob + byteArrayOf(0), // trailing byte
            blob.copyOf().also { it[4 + 8 + 3] = 9 }, // unknown bounds kind on row 0
        )) {
            assertTrue(runCatching { RegistrationKeys.decodeLayout(bad) }.exceptionOrNull() is IllegalArgumentException)
        }
    }

    /**
     * Cross-language wire pin: the Kotlin encoder must produce exactly the
     * checked-in golden bytes. A byte-identical Rust test
     * (`pubkey_rows::tests::golden_fixture_decodes_to_the_dashpay_policy`)
     * decodes the SAME file and asserts the DashPay-bound rows carry
     * `dashpay_contract::ID_BYTES`, so this catches byte-order / field-order
     * skew that two independently-passing tests would miss — and pins the
     * mirrored DashPay contract id to the real Rust constant transitively.
     */
    @Test
    fun `encoder output matches the cross-language golden fixture`() {
        val golden = javaClass.getResourceAsStream("/golden/registration_pubkeys_v2.bin")
            .use { requireNotNull(it) { "golden fixture resource missing" }.readBytes() }

        // The golden fixture holds the six rows of the DashPay layout.
        val rows = RegistrationKeys.buildRegistrationRows(fixturePubkeys(6), withDashPayKeys)
        val encoded = IdentityPubkeyCodec.encode(rows)

        assertArrayEquals(
            "Kotlin encoder drifted from the checked-in golden fixture — the Rust " +
                "parser will misread the registration blob (wire-format skew)",
            golden,
            encoded,
        )
    }

    /**
     * Kind 3 (ContractGroup) rides the wire as the kind byte plus the 32-byte
     * contract group id, with no document type before the limits flags: the layout
     * `pubkey_rows::parse_pubkey_rows` reads
     * (`pubkey_rows::tests::round_trips_contract_group_bounds_kind`).
     */
    @Test
    fun `should encode contract group bounds as kind 3 with the id and no document type`() {
        val contractGroupId = ByteArray(32) { (it + 0x40).toByte() }
        val pubkey = fixturePubkey(7)
        val encoded = IdentityPubkeyCodec.encode(
            listOf(
                IdentityPubkey(
                    keyId = 7,
                    keyType = KeyType.ECDSA_SECP256K1,
                    purpose = KeyPurpose.AUTHENTICATION,
                    securityLevel = SecurityLevel.HIGH,
                    pubkeyBytes = pubkey,
                    contractBounds = ContractBounds.ContractGroup(contractGroupId),
                ),
            ),
        )

        val expected = byteArrayOf(
            0, 0, 0, 1, // rowCount
            0, 0, 0, 7, // keyId
            0, // keyType ECDSA_SECP256K1
            0, // purpose AUTHENTICATION
            2, // securityLevel HIGH
            0, // readOnly
            3, // contractBoundsKind ContractGroup
            0, 33, // pubkeyLen
        ) + pubkey + contractGroupId + byteArrayOf(
            0, // limitsFlags: no budget, no expiry
        )
        assertArrayEquals(expected, encoded)
        // Only the limits flags follow the id: no docTypeLen, no docType.
        assertEquals(4 + 4 + 5 + 2 + 33 + 32 + 1, encoded.size)
    }

    @Test
    fun `should keep the contract bounds kind bytes of the other variants`() {
        val id = ByteArray(32) { 9 }
        assertEquals(0, IdentityPubkeyCodec.contractBoundsKind(null))
        assertEquals(1, IdentityPubkeyCodec.contractBoundsKind(ContractBounds.SingleContract(id)))
        assertEquals(
            2,
            IdentityPubkeyCodec.contractBoundsKind(
                ContractBounds.SingleContractDocumentType(id, "contactRequest"),
            ),
        )
        assertEquals(3, IdentityPubkeyCodec.contractBoundsKind(ContractBounds.ContractGroup(id)))
    }

    @Test
    fun `should require a 32 byte contract group id and compare by content`() {
        val failure = runCatching { ContractBounds.ContractGroup(ByteArray(31)) }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)

        val a: ContractBounds = ContractBounds.ContractGroup(ByteArray(32) { 5 })
        val b: ContractBounds = ContractBounds.ContractGroup(ByteArray(32) { 5 })
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
        // Same id, different variant: a group bound is not a contract bound.
        val single: ContractBounds = ContractBounds.SingleContract(ByteArray(32) { 5 })
        assertNotEquals(single, a)
    }
}
