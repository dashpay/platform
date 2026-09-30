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

    @Test
    fun `base four rows carry the canonical auth and transfer roles`() {
        val rows = RegistrationKeys.buildRegistrationRows(fixturePubkeys(4), includeDashPayKeys = false)
        assertEquals(4, rows.size)
        assertEquals(
            listOf(0, 1, 2, 3),
            rows.map { it.keyId },
        )
        assertEquals(
            listOf(
                Triple(KeyPurpose.AUTHENTICATION, SecurityLevel.MASTER, null),
                Triple(KeyPurpose.AUTHENTICATION, SecurityLevel.CRITICAL, null),
                Triple(KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH, null),
                Triple(KeyPurpose.TRANSFER, SecurityLevel.CRITICAL, null),
            ),
            rows.map { Triple(it.purpose, it.securityLevel, it.contractBounds) },
        )
        rows.forEach {
            assertEquals(KeyType.ECDSA_SECP256K1, it.keyType)
            assertEquals(false, it.readOnly)
        }
    }

    @Test
    fun `six rows append the DashPay encryption and decryption pair`() {
        val rows = RegistrationKeys.buildRegistrationRows(fixturePubkeys(6), includeDashPayKeys = true)
        assertEquals(6, rows.size)

        val enc = rows[4]
        val dec = rows[5]
        assertEquals(KeyPurpose.ENCRYPTION, enc.purpose)
        assertEquals(KeyPurpose.DECRYPTION, dec.purpose)
        for (row in listOf(enc, dec)) {
            assertEquals(KeyType.ECDSA_SECP256K1, row.keyType)
            assertEquals(SecurityLevel.MEDIUM, row.securityLevel)
            assertEquals(false, row.readOnly)
            val bounds = row.contractBounds
            assertTrue(bounds is ContractBounds.SingleContractDocumentType)
            bounds as ContractBounds.SingleContractDocumentType
            assertArrayEquals(RegistrationKeys.DASHPAY_CONTRACT_ID, bounds.contractId)
            assertEquals(
                RegistrationKeys.DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE,
                bounds.documentTypeName,
            )
        }
        // The base four keep their auth/transfer roles unbounded.
        (0..3).forEach { assertNull(rows[it].contractBounds) }
    }

    @Test
    fun `wrong key count is rejected`() {
        val failure = runCatching {
            RegistrationKeys.buildRegistrationRows(fixturePubkeys(5), includeDashPayKeys = true)
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
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

        val rows = RegistrationKeys.buildRegistrationRows(fixturePubkeys(6), includeDashPayKeys = true)
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
