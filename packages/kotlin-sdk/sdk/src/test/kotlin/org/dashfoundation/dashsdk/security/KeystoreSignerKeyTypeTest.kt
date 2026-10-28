package org.dashfoundation.dashsdk.security

import org.dashfoundation.dashsdk.identity.KeyType
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the key types [KeystoreSigner.canSignWith] may claim for an identity
 * key. Rust chooses the signing key by that answer, so a BLS or EdDSA key
 * with a stored private key must not be reported signable: the stored-key
 * path signs secp256k1 ECDSA only.
 */
class KeystoreSignerKeyTypeTest {

    @Test
    fun signsOnlyEcdsaIdentityKeyTypes() {
        val signable = KeyType.values().filter {
            KeystoreSigner.signsIdentityKeyType(it.ffiValue)
        }
        assertEquals(listOf(KeyType.ECDSA_SECP256K1, KeyType.ECDSA_HASH160), signable)
    }
}
