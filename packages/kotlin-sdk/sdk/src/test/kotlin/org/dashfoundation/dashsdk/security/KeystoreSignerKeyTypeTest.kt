package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins how [KeystoreSigner.canSignWith] answers for an identity key. Rust
 * chooses the signing key by that answer, so a key type Rust's predicate
 * refuses (BLS, EdDSA) must not be reported signable even with a stored
 * private key. Which types Rust signs is pinned by the Rust tests of
 * `dash_sdk_resolver_supports_key_type`; this JVM test cannot load the
 * native library, so it stubs the predicate.
 */
class KeystoreSignerKeyTypeTest {

    @Test
    fun refusesAKeyTypeRustDoesNotSignWithoutConsultingStorage() {
        var storageConsulted = false
        val signable = KeystoreSigner.identityKeySignable(
            keyType = 1,
            supportsKeyType = { false },
        ) {
            storageConsulted = true
            true
        }
        assertFalse(signable)
        assertFalse(storageConsulted)
    }

    @Test
    fun asksRustAboutTheKeyTypeItWasGiven() {
        var askedFor: Int? = null
        KeystoreSigner.identityKeySignable(
            keyType = 2,
            supportsKeyType = {
                askedFor = it
                true
            },
        ) { true }
        assertEquals(2, askedFor)
    }

    @Test
    fun reportsAKeyTypeRustSignsSignableOnlyWhenItsKeyDecrypts() {
        assertTrue(KeystoreSigner.identityKeySignable(0, { true }) { true })
        assertFalse(KeystoreSigner.identityKeySignable(0, { true }) { false })
    }
}
