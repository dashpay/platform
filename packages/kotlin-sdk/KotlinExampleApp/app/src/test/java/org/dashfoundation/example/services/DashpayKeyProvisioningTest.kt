package org.dashfoundation.example.services

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.identity.ContractBounds
import org.dashfoundation.dashsdk.identity.IdentityKeyPreview
import org.dashfoundation.dashsdk.identity.KeyPurpose
import org.dashfoundation.dashsdk.identity.KeyType
import org.dashfoundation.dashsdk.identity.RegistrationKeySlot
import org.dashfoundation.dashsdk.identity.RegistrationKeys
import org.dashfoundation.dashsdk.identity.SecurityLevel
import org.dashfoundation.example.ui.identity.CreateIdentityFundingSource
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Lifecycle + policy coverage for [DashpayKeyProvisioning] and the
 * funding-source DashPay-provisioning gate. Verifies the rows follow the
 * supplied Rust layout slots (7-key fresh / 4-key resume fixtures mirroring
 * `platform_wallet::IdentityCreationKeyLayout`), and that every derived
 * private scalar is scrubbed on both the success and the failure path.
 */
class DashpayKeyProvisioningTest {

    private fun slot(
        keyId: Int,
        purpose: KeyPurpose,
        level: SecurityLevel,
        bounds: ContractBounds? = null,
    ) = RegistrationKeySlot(keyId, keyId, KeyType.ECDSA_SECP256K1, purpose, level, bounds)

    private fun resumeSlots(): List<RegistrationKeySlot> = listOf(
        slot(0, KeyPurpose.AUTHENTICATION, SecurityLevel.MASTER),
        slot(1, KeyPurpose.AUTHENTICATION, SecurityLevel.CRITICAL),
        slot(2, KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH),
        slot(3, KeyPurpose.TRANSFER, SecurityLevel.CRITICAL),
    )

    private fun dashPaySlots(): List<RegistrationKeySlot> {
        val bounds = ContractBounds.SingleContractDocumentType(
            RegistrationKeys.DASHPAY_CONTRACT_ID,
            RegistrationKeys.DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE,
        )
        return resumeSlots() + listOf(
            slot(4, KeyPurpose.ENCRYPTION, SecurityLevel.MEDIUM, bounds),
            slot(5, KeyPurpose.DECRYPTION, SecurityLevel.MEDIUM, bounds),
            slot(6, KeyPurpose.AUTHENTICATION, SecurityLevel.MEDIUM),
        )
    }

    /** A derived preview at [keyId] with a non-zero private scalar. */
    private fun preview(keyId: Int): IdentityKeyPreview = IdentityKeyPreview(
        identityIndex = 5,
        derivationPath = "m/9'/5'/$keyId'",
        publicKey = ByteArray(33) { (keyId + 2).toByte() },
        privateKey = ByteArray(32) { (keyId + 1).toByte() },
    )

    @Test
    fun `fresh funding provisions every layout key with DashPay bounds`() = runTest {
        val previews = (0 until 7).map(::preview)
        val walletId = ByteArray(32) { 9 }
        data class StoredKey(val hex: String, val scalar: ByteArray, val owner: ByteArray)
        val stored = mutableListOf<StoredKey>()
        val keySet = DashpayKeyProvisioning.provision(
            previews = previews,
            slots = dashPaySlots(),
            walletId = walletId,
            persister = { hex, privateKey, owner ->
                stored += StoredKey(hex, privateKey.copyOf(), owner.copyOf())
            },
        )
        val rows = keySet.rows

        assertEquals(5, keySet.identityIndex)
        assertEquals(7, rows.size)
        assertEquals(listOf(0, 1, 2, 3, 4, 5, 6), rows.map { it.keyId })
        assertEquals(KeyPurpose.AUTHENTICATION, rows[6].purpose)
        assertEquals(SecurityLevel.MEDIUM, rows[6].securityLevel)
        assertEquals(KeyPurpose.ENCRYPTION, rows[4].purpose)
        assertEquals(KeyPurpose.DECRYPTION, rows[5].purpose)
        assertEquals(SecurityLevel.MEDIUM, rows[4].securityLevel)
        assertEquals(SecurityLevel.MEDIUM, rows[5].securityLevel)
        assertEquals(KeyType.ECDSA_SECP256K1, rows[4].keyType)
        assertEquals(KeyType.ECDSA_SECP256K1, rows[5].keyType)
        assertFalse(rows[4].readOnly)
        assertFalse(rows[5].readOnly)
        val bounds = rows[4].contractBounds
        assertTrue(bounds is ContractBounds.SingleContractDocumentType)
        bounds as ContractBounds.SingleContractDocumentType
        assertArrayEquals(RegistrationKeys.DASHPAY_CONTRACT_ID, bounds.contractId)
        assertEquals(RegistrationKeys.DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE, bounds.documentTypeName)
        assertEquals(bounds, rows[5].contractBounds)

        // Every slot is persisted under its matching public bytes while the
        // scalar is still non-zero, owner-scoped to this wallet, then scrubbed.
        assertEquals(7, stored.size)
        for ((keyId, p) in previews.withIndex()) {
            assertEquals(p.publicKeyHex, stored[keyId].hex)
            assertArrayEquals(ByteArray(32) { (keyId + 1).toByte() }, stored[keyId].scalar)
            assertArrayEquals(walletId, stored[keyId].owner)
            assertArrayEquals(p.publicKey, rows[keyId].pubkeyBytes)
            assertArrayEquals(ByteArray(32), p.privateKey)
        }
    }

    @Test
    fun `resume provisions only the base four keys`() = runTest {
        val previews = (0 until 4).map(::preview)
        val keySet = DashpayKeyProvisioning.provision(
            previews = previews,
            slots = resumeSlots(),
            walletId = ByteArray(32) { 9 },
            persister = { _, _, _ -> },
        )
        val rows = keySet.rows
        assertEquals(4, rows.size)
        assertTrue(rows.none { it.contractBounds != null })
    }

    @Test
    fun `a persist failure still scrubs every private scalar`() = runTest {
        val previews = (0 until 7).map(::preview)
        val failure = runCatching {
            DashpayKeyProvisioning.provision(
                previews = previews,
                slots = dashPaySlots(),
                walletId = ByteArray(32) { 9 },
                // Fail on the 3rd key — keys 0/1 persisted, 3..6 never reached.
                persister = { _, _, _ ->
                    if (persistCount++ == 2) throw IllegalStateException("keystore down")
                },
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalStateException)
        // No plaintext scalar may survive a partial failure.
        for (p in previews) {
            assertArrayEquals(ByteArray(32), p.privateKey)
        }
    }

    private var persistCount = 0

    @Test
    fun `wrong preview count is rejected before any persist and still scrubs scalars`() = runTest {
        var persisted = false
        val previews = (0 until 5).map(::preview) // not the 7 the layout has
        val failure = runCatching {
            DashpayKeyProvisioning.provision(
                previews = previews,
                slots = dashPaySlots(),
                walletId = ByteArray(32) { 9 },
                persister = { _, _, _ -> persisted = true },
            )
        }
        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(persisted)
        // The count rejection is a wire-skew symptom fired right after
        // derivation — no plaintext scalar may survive it.
        for (p in previews) {
            assertArrayEquals(ByteArray(32), p.privateKey)
        }
    }

    @Test
    fun `mixed identity slots are rejected before persist and all scalars are scrubbed`() = runTest {
        var persisted = false
        val previews = (0 until 4).map(::preview).toMutableList().also {
            it[3] = IdentityKeyPreview(
                identityIndex = 6,
                derivationPath = it[3].derivationPath,
                publicKey = it[3].publicKey,
                privateKey = it[3].privateKey,
            )
        }

        val failure = runCatching {
            DashpayKeyProvisioning.provision(
                previews = previews,
                slots = resumeSlots(),
                walletId = ByteArray(32) { 9 },
                persister = { _, _, _ -> persisted = true },
            )
        }

        assertTrue(failure.exceptionOrNull() is IllegalArgumentException)
        assertFalse(persisted)
        previews.forEach { assertArrayEquals(ByteArray(32), it.privateKey) }
    }

    @Test
    fun `only asset-lock resume excludes DashPay provisioning`() {
        assertTrue(CreateIdentityFundingSource.CoreBalance.includesDashPayKeys)
        assertTrue(CreateIdentityFundingSource.PlatformAddress.includesDashPayKeys)
        assertTrue(CreateIdentityFundingSource.ShieldedBalance.includesDashPayKeys)
        assertFalse(CreateIdentityFundingSource.AssetLockResume.includesDashPayKeys)
    }
}
