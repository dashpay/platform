package org.dashfoundation.dashsdk.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Android Keystore wrapper — the Keystore counterpart of
 * `KeychainManager.swift`.
 *
 * Android Keystore cannot export key material and does not support
 * secp256k1, so (mirroring iOS, where the Keychain stores raw bytes) the
 * secrets themselves — mnemonics, derived identity private keys — are data
 * encrypted under non-exportable AES-256-GCM master keys created here:
 *
 * - [MASTER_ALIAS] `org.dashfoundation.wallet.master` — mnemonics and
 *   general wallet secrets (name parity with the iOS keychain service
 *   `org.dashfoundation.wallet`).
 * - [KEYS_ALIAS] `org.dashfoundation.wallet.keys` — identity private keys;
 *   requires user authentication (biometric or device credential) within
 *   [AUTH_VALIDITY_SECONDS] of use, matching the iOS access policy.
 *
 * StrongBox is used when available, with a software-Keystore fallback.
 */
class KeystoreManager {

    /** Encrypt [plaintext] under the master key; returns (iv, ciphertext). */
    fun encrypt(plaintext: ByteArray, alias: String = MASTER_ALIAS): EncryptedBlob {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, masterKey(alias))
        val ciphertext = cipher.doFinal(plaintext)
        return EncryptedBlob(iv = cipher.iv, ciphertext = ciphertext)
    }

    /** Decrypt a blob produced by [encrypt] under the same alias. */
    fun decrypt(blob: EncryptedBlob, alias: String = MASTER_ALIAS): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(
            Cipher.DECRYPT_MODE,
            masterKey(alias),
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

    private fun masterKey(alias: String): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keyStore.getKey(alias, null) as? SecretKey)?.let { return it }
        return generateMasterKey(alias)
    }

    private fun generateMasterKey(alias: String): SecretKey {
        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val builder = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setUnlockedDeviceRequired(true)
            if (alias == KEYS_ALIAS) {
                builder
                    .setUserAuthenticationRequired(true)
                    .setUserAuthenticationParameters(
                        AUTH_VALIDITY_SECONDS,
                        KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                    )
            }
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

    companion object {
        const val MASTER_ALIAS = "org.dashfoundation.wallet.master"
        const val KEYS_ALIAS = "org.dashfoundation.wallet.keys"

        /** Auth window for the identity-keys alias, in seconds. */
        const val AUTH_VALIDITY_SECONDS = 30

        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_TAG_BITS = 128
    }
}
