package org.dashfoundation.dashsdk.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.PublicKey
import java.security.spec.MGF1ParameterSpec
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.OAEPParameterSpec
import javax.crypto.spec.PSource

/**
 * Android Keystore wrapper — the Keystore counterpart of
 * `KeychainManager.swift`.
 *
 * Android Keystore cannot export key material and does not support
 * secp256k1, so (mirroring iOS, where the Keychain stores raw bytes) the
 * secrets themselves — mnemonics, derived identity private keys — are data
 * encrypted under non-exportable Keystore keys created here:
 *
 * - [MASTER_ALIAS] `org.dashfoundation.wallet.master` — mnemonics and
 *   general wallet secrets, under a non-auth AES-256-GCM key (name parity
 *   with the iOS keychain service `org.dashfoundation.wallet`).
 * - [KEYS_ALIAS] `org.dashfoundation.wallet.keys` — identity private keys,
 *   under an RSA-2048 OAEP(SHA-256) keypair. The PUBLIC key encrypts and is
 *   never auth-gated, so **storing** a key never prompts (parity with iOS,
 *   which stores identity keys with no access control) — crucially the
 *   persistence callback stores from a Rust Tokio thread holding the
 *   wallet-manager write lock, where a prompt is impossible. The PRIVATE key
 *   decrypts under user authentication (strong biometric or device
 *   credential) within [AUTH_VALIDITY_SECONDS] of use, so **signing**
 *   requires auth (the read-with-auth hardening mirrors the iOS seed
 *   policy). A symmetric auth-required key would gate encrypt too and break
 *   those unprompted write paths.
 *
 * StrongBox is used when available, with a software-Keystore fallback.
 */
class KeystoreManager {

    /**
     * Encrypt [plaintext] under [alias]; returns (iv, ciphertext).
     * [MASTER_ALIAS] uses AES-256-GCM (iv is the GCM nonce). [KEYS_ALIAS]
     * uses the RSA public key (no iv — the blob's iv is empty) and never
     * requires authentication, so identity-key writes never prompt.
     */
    fun encrypt(plaintext: ByteArray, alias: String = MASTER_ALIAS): EncryptedBlob {
        if (alias == KEYS_ALIAS) {
            val cipher = Cipher.getInstance(RSA_TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, keysPublicKey(), oaepSpec())
            return EncryptedBlob(iv = ByteArray(0), ciphertext = cipher.doFinal(plaintext))
        }
        val cipher = Cipher.getInstance(AES_TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, secretKey(alias))
        return EncryptedBlob(iv = cipher.iv, ciphertext = cipher.doFinal(plaintext))
    }

    /**
     * Decrypt a blob produced by [encrypt] under the same [alias]. The
     * [KEYS_ALIAS] RSA private-key decrypt throws
     * `UserNotAuthenticatedException` when the [AUTH_VALIDITY_SECONDS] auth
     * window is closed — the caller (`KeystoreSigner`) prompts via the
     * `BiometricGate` and retries.
     */
    fun decrypt(blob: EncryptedBlob, alias: String = MASTER_ALIAS): ByteArray {
        if (alias == KEYS_ALIAS) {
            val cipher = Cipher.getInstance(RSA_TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, keysPrivateKey(), oaepSpec())
            return cipher.doFinal(blob.ciphertext)
        }
        val cipher = Cipher.getInstance(AES_TRANSFORMATION)
        cipher.init(
            Cipher.DECRYPT_MODE,
            secretKey(alias),
            GCMParameterSpec(GCM_TAG_BITS, blob.iv),
        )
        return cipher.doFinal(blob.ciphertext)
    }

    /** IV + ciphertext pair, serialized as `iv.size || iv || ciphertext`. */
    data class EncryptedBlob(val iv: ByteArray, val ciphertext: ByteArray) {
        fun encode(): ByteArray =
            byteArrayOf(iv.size.toByte()) + iv + ciphertext

        override fun equals(other: Any?): Boolean =
            other is EncryptedBlob && iv.contentEquals(other.iv) &&
                ciphertext.contentEquals(other.ciphertext)

        override fun hashCode(): Int = 31 * iv.contentHashCode() + ciphertext.contentHashCode()

        companion object {
            fun decode(encoded: ByteArray): EncryptedBlob {
                val ivLen = encoded[0].toInt() and 0xFF
                return EncryptedBlob(
                    iv = encoded.copyOfRange(1, 1 + ivLen),
                    ciphertext = encoded.copyOfRange(1 + ivLen, encoded.size),
                )
            }
        }
    }

    private fun androidKeyStore(): KeyStore =
        KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    // ── MASTER_ALIAS: non-auth AES-256-GCM (mnemonics / general secrets) ──

    private fun secretKey(alias: String): SecretKey {
        (androidKeyStore().getKey(alias, null) as? SecretKey)?.let { return it }
        return generateAesKey(alias)
    }

    private fun generateAesKey(alias: String): SecretKey {
        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val builder = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setUnlockedDeviceRequired(true)
            if (strongBox) builder.setIsStrongBoxBacked(true)
            return builder.build()
        }

        val generator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEYSTORE,
        )
        return try {
            generator.init(spec(strongBox = true))
            generator.generateKey()
        } catch (_: StrongBoxUnavailableException) {
            generator.init(spec(strongBox = false))
            generator.generateKey()
        }
    }

    // ── KEYS_ALIAS: RSA-2048 OAEP keypair (identity private keys) ──
    // Public key encrypts (never auth-gated → unprompted writes); private key
    // decrypts under user auth within AUTH_VALIDITY_SECONDS (signing prompts).

    private fun keysPublicKey(): PublicKey =
        androidKeyStore().getCertificate(KEYS_ALIAS)?.publicKey ?: generateKeysKeyPair().public

    private fun keysPrivateKey(): PrivateKey =
        (androidKeyStore().getKey(KEYS_ALIAS, null) as? PrivateKey) ?: generateKeysKeyPair().private

    private fun generateKeysKeyPair(): KeyPair {
        // Migration: an earlier build created KEYS_ALIAS as a symmetric AES key,
        // which gated encrypt too and broke the unprompted identity-key writes.
        // Drop any such stale entry so the alias is cleanly re-created as the
        // RSA keypair. The accessors above only reach here when the entry is
        // absent or the wrong type (a `SecretKeyEntry` has no certificate and
        // is not a `PrivateKey`), so a valid RSA keypair is never destroyed.
        runCatching { androidKeyStore().deleteEntry(KEYS_ALIAS) }

        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val builder = KeyGenParameterSpec.Builder(
                KEYS_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setKeySize(RSA_KEY_SIZE)
                // SHA-1 must be permitted alongside SHA-256: AndroidKeyStore's
                // OAEP MGF1 is always SHA-1, so the key has to allow SHA-1 as
                // the MGF1 digest even though the OAEP digest is SHA-256. See
                // [oaepSpec] — both encrypt and decrypt pin MGF1 = SHA-1.
                .setDigests(KeyProperties.DIGEST_SHA256, KeyProperties.DIGEST_SHA1)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_RSA_OAEP)
                .setUnlockedDeviceRequired(true)
                .setUserAuthenticationRequired(true)
                .setUserAuthenticationParameters(
                    AUTH_VALIDITY_SECONDS,
                    KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                )
            if (strongBox) builder.setIsStrongBoxBacked(true)
            return builder.build()
        }

        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_RSA,
            ANDROID_KEYSTORE,
        )
        return try {
            generator.initialize(spec(strongBox = true))
            generator.generateKeyPair()
        } catch (_: StrongBoxUnavailableException) {
            generator.initialize(spec(strongBox = false))
            generator.generateKeyPair()
        }
    }

    // Pin MGF1 = SHA-1 to match AndroidKeyStore. The public-key encrypt runs
    // on the default JCE provider (the certificate's public key is a plain
    // RSAPublicKey), which would otherwise default MGF1 to SHA-256; the
    // keystore private-key decrypt uses MGF1 = SHA-1, so an unpinned encrypt
    // produces ciphertext the decrypt cannot open. Pinning SHA-1 on both keeps
    // them consistent.
    private fun oaepSpec(): OAEPParameterSpec =
        OAEPParameterSpec("SHA-256", "MGF1", MGF1ParameterSpec.SHA1, PSource.PSpecified.DEFAULT)

    companion object {
        const val MASTER_ALIAS = "org.dashfoundation.wallet.master"
        const val KEYS_ALIAS = "org.dashfoundation.wallet.keys"

        /** Auth window for the identity-keys alias, in seconds. */
        const val AUTH_VALIDITY_SECONDS = 30

        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val AES_TRANSFORMATION = "AES/GCM/NoPadding"
        private const val RSA_TRANSFORMATION = "RSA/ECB/OAEPWithSHA-256AndMGF1Padding"
        private const val RSA_KEY_SIZE = 2048
        private const val GCM_TAG_BITS = 128
    }
}
