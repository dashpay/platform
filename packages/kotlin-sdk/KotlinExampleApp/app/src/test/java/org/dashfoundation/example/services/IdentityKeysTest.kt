package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.identity.KeyPurpose
import org.dashfoundation.dashsdk.identity.KeyType
import org.dashfoundation.dashsdk.identity.SecurityLevel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pure-logic coverage for the key-management helpers behind
 * `AddIdentityKeyScreen` / `KeyDetailScreen`:
 * DPP-name normalization for the two Room encodings (stringified rawValue
 * vs enum name) and the add-key slot / validation rules of
 * [IdentityKeyAdditionFlow].
 */
class IdentityKeysTest {

    @Test
    fun normalizesNumericAndNameEncodings() {
        assertEquals("AUTHENTICATION", IdentityKeys.purposeName("0"))
        assertEquals("AUTHENTICATION", IdentityKeys.purposeName("authentication"))
        assertEquals("TRANSFER", IdentityKeys.purposeName("3"))
        assertEquals("MASTER", IdentityKeys.securityLevelName("0"))
        assertEquals("HIGH", IdentityKeys.securityLevelName("high"))
        assertEquals("ECDSA_HASH160", IdentityKeys.keyTypeName("2"))
        // Unknown values pass through upper-cased rather than crashing.
        assertEquals("MYSTERY", IdentityKeys.purposeName("mystery"))
    }

    @Test
    fun securityLevelRankOrdersMasterStrongest() {
        assertEquals(0, IdentityKeys.securityLevelRank("MASTER"))
        assertEquals(1, IdentityKeys.securityLevelRank("1"))
        assertEquals(3, IdentityKeys.securityLevelRank("medium"))
    }

    @Test
    fun nextKeyIdExtendsPastHighestEverUsed() {
        assertEquals(1, IdentityKeyAdditionFlow.nextKeyId(emptyList()))
        // Non-recyclable: a disabled key's hole (id 1) is never reused.
        assertEquals(6, IdentityKeyAdditionFlow.nextKeyId(listOf(0, 2, 5)))
    }

    @Test
    fun keySpecValidationMirrorsDriveRules() {
        assertNotNull(
            IdentityKeyAdditionFlow.validationError(
                IdentityKeyAdditionFlow.KeySpec(
                    KeyType.BLS12_381, KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH,
                ),
            ),
        )
        // Encryption without bounds is refused; authentication needs none.
        assertNotNull(
            IdentityKeyAdditionFlow.validationError(
                IdentityKeyAdditionFlow.KeySpec(
                    KeyType.ECDSA_SECP256K1, KeyPurpose.ENCRYPTION, SecurityLevel.MEDIUM,
                ),
            ),
        )
        assertNull(
            IdentityKeyAdditionFlow.validationError(
                IdentityKeyAdditionFlow.KeySpec(
                    KeyType.ECDSA_SECP256K1, KeyPurpose.AUTHENTICATION, SecurityLevel.HIGH,
                ),
            ),
        )
    }
}
