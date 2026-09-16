package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the key-type-aware ownership check used by the identity-key repair
 * path ([IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches],
 * dashpay/platform#4183 review).
 *
 * The repair derives the KEYPAIR and must prove its public half is the key it
 * was asked to restore BEFORE persisting. For `ECDSA_HASH160` /
 * `EDDSA_25519_HASH160` keys DPP stores the 20-byte HASH160 of the pubkey as
 * the on-chain data, not the pubkey itself — so a raw byte compare (33-byte
 * derived pubkey vs 20-byte stored hash) can NEVER match, and these keys were
 * permanently un-repairable. The check now HASH160s the derived pubkey for
 * those types.
 */
class IdentityKeyPrivateKeyDeriverMatchTest {

    private fun unhex(s: String): ByteArray =
        s.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

    // Canonical vector: a compressed secp256k1 pubkey and its HASH160.
    private val compressedPubkey =
        unhex("0250863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352")
    private val pubkeyHash160 = unhex("f54a5851e9372b87810a8e60cdd2e7cfd80b6e31")

    private val ecdsaSecp256k1 = 0
    private val ecdsaHash160 = 2
    private val eddsa25519Hash160 = 4

    @Test
    fun hash160KeyRepairs_derivedPubkeyMatchesStoredHash() {
        // The regression the reviewer flagged: a HASH160-typed key whose
        // stored data is the 20-byte hash of the derived pubkey MUST verify,
        // so the repair proceeds instead of failing forever.
        assertTrue(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = pubkeyHash160,
                keyType = ecdsaHash160,
            ),
        )
        assertTrue(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = pubkeyHash160,
                keyType = eddsa25519Hash160,
            ),
        )
    }

    @Test
    fun hash160KeyRepair_rejectsWrongHash() {
        val wrongHash = ByteArray(20) { 0x11 }
        assertFalse(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = wrongHash,
                keyType = ecdsaHash160,
            ),
        )
    }

    @Test
    fun hash160KeyRepair_rejectsRawPubkeyAsExpected() {
        // The pre-fix bug shape: comparing the derived pubkey against a stored
        // value that is the full pubkey (not its hash) must NOT be treated as a
        // match for a HASH160 key type — the on-chain data is the hash.
        assertFalse(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = compressedPubkey,
                keyType = ecdsaHash160,
            ),
        )
    }

    @Test
    fun nonHash160KeyStillComparesRawBytes() {
        // Every non-HASH160 key type keeps the plain content comparison.
        assertTrue(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = compressedPubkey,
                keyType = ecdsaSecp256k1,
            ),
        )
        assertFalse(
            IdentityKeyPrivateKeyDeriver.derivedPublicKeyMatches(
                derived = compressedPubkey,
                expected = pubkeyHash160,
                keyType = ecdsaSecp256k1,
            ),
        )
    }
}
