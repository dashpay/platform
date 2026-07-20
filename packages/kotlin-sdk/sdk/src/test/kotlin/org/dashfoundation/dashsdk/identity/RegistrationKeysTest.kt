package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
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
        rows.forEach { assertEquals(KeyType.ECDSA_SECP256K1, it.keyType) }
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
            assertEquals(SecurityLevel.MEDIUM, row.securityLevel)
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
        val golden = javaClass.getResourceAsStream("/golden/registration_pubkeys_v1.bin")
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
}
