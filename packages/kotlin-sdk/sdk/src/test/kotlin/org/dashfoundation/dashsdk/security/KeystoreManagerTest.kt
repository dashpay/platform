package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * [KeystoreManager.isKeysBlobDecryptable] is a pure structural check (no
 * Android Keystore call), unlike the rest of [KeystoreManager] — the only
 * corner of this class runnable on the JVM tier. Pins the boundary
 * [WalletStorage.storeIfAbsent] relies on to decide "usable" vs. "needs a
 * re-derive": present-but-undecryptable legacy blobs must be treated as
 * absent, or a wallet stuck with one would never recover a working key.
 */
class KeystoreManagerTest {

    private val keystore = KeystoreManager()

    @Test
    fun rsaShapedBlobIsDecryptable() {
        // No iv (RSA never carries one), ciphertext exactly one 2048-bit
        // RSA block — the current KEYS_ALIAS scheme's shape.
        val blob = KeystoreManager.EncryptedBlob(
            iv = ByteArray(0),
            ciphertext = ByteArray(256),
        )

        assertTrue(keystore.isKeysBlobDecryptable(blob))
    }

    @Test
    fun legacyAesGcmShapedBlobIsNotDecryptable() {
        // The pre-RSA scheme's shape: a 12-byte GCM nonce in `iv`. Written
        // before KEYS_ALIAS became RSA-2048; lost its key when the RSA pair
        // replaced it, so signing with it can only fail.
        val blob = KeystoreManager.EncryptedBlob(
            iv = ByteArray(12),
            ciphertext = ByteArray(256),
        )

        assertFalse(keystore.isKeysBlobDecryptable(blob))
    }

    @Test
    fun wrongSizedCiphertextIsNotDecryptable() {
        val blob = KeystoreManager.EncryptedBlob(
            iv = ByteArray(0),
            ciphertext = ByteArray(255),
        )

        assertFalse(keystore.isKeysBlobDecryptable(blob))
    }
}
