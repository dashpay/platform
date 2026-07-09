package org.dashfoundation.dashsdk.security

import android.os.Build
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
 * - [KEYS_ALIAS_DEVICE_BOUND] `org.dashfoundation.wallet.keys.devicebound` —
 *   the [KeySecurityPolicy.DEVICE_BOUND] variant of the identity-keys
 *   alias: the same RSA-2048 OAEP wrapping pair, but withOUT the
 *   user-authentication gate on the private key, for host apps that gate
 *   signing behind their own auth model (see [KeySecurityPolicy]).
 *
 * Which identity-keys alias a manager instance writes/reads is selected by
 * the [keySecurityPolicy] constructor parameter (default: the historical
 * [KeySecurityPolicy.AUTH_GATED]) and exposed as [keysAlias].
 *
 * StrongBox is used when available, with a software-Keystore fallback.
 *
 * @param keySecurityPolicy security policy for the identity-keys alias.
 *   Defaults to [KeySecurityPolicy.AUTH_GATED], which preserves the
 *   historical behavior exactly.
 */
class KeystoreManager(
    val keySecurityPolicy: KeySecurityPolicy = KeySecurityPolicy.AUTH_GATED,
) {

    /**
     * The identity-keys alias this manager targets, per
     * [keySecurityPolicy]: [KEYS_ALIAS] (auth-gated decrypt) or
     * [KEYS_ALIAS_DEVICE_BOUND] (non-gated decrypt). [WalletStorage] passes
     * this to [encrypt]/[decrypt] for identity-key material.
     */
    val keysAlias: String
        get() = when (keySecurityPolicy) {
            KeySecurityPolicy.AUTH_GATED -> KEYS_ALIAS
            KeySecurityPolicy.DEVICE_BOUND -> KEYS_ALIAS_DEVICE_BOUND
        }

    /**
     * Encrypt [plaintext] under [alias]; returns (iv, ciphertext).
     * [MASTER_ALIAS] uses AES-256-GCM (iv is the GCM nonce). The
     * identity-keys aliases ([KEYS_ALIAS] / [KEYS_ALIAS_DEVICE_BOUND])
     * use the RSA public key (no iv — the blob's iv is empty) and never
     * require authentication, so identity-key writes never prompt.
     */
    fun encrypt(plaintext: ByteArray, alias: String = MASTER_ALIAS): EncryptedBlob {
        if (isIdentityKeysAlias(alias)) {
            val cipher = Cipher.getInstance(RSA_TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, keysPublicKey(alias), oaepSpec())
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
     * `BiometricGate` and retries. The [KEYS_ALIAS_DEVICE_BOUND] decrypt is
     * never auth-gated (see [KeySecurityPolicy.DEVICE_BOUND]).
     */
    fun decrypt(blob: EncryptedBlob, alias: String = MASTER_ALIAS): ByteArray {
        if (isIdentityKeysAlias(alias)) {
            val cipher = Cipher.getInstance(RSA_TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, keysPrivateKey(alias), oaepSpec())
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

    /**
     * Whether [blob] is structurally a [KEYS_ALIAS] RSA blob the current
     * scheme can decrypt: no iv (RSA blobs never carry one) and exactly one
     * RSA block of ciphertext. Blobs written by the pre-RSA AES-GCM scheme
     * carry a GCM nonce in `iv` and became undecryptable when the AES key
     * was dropped for the RSA pair — they need a re-derive. Never decrypts,
     * so it never prompts for authentication.
     */
    fun isKeysBlobDecryptable(blob: EncryptedBlob): Boolean =
        blob.iv.isEmpty() && blob.ciphertext.size == RSA_KEY_SIZE / 8

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

    // ── Identity-keys aliases: RSA-2048 OAEP keypair per alias ──
    // Public key encrypts (never auth-gated → unprompted writes); the
    // KEYS_ALIAS private key decrypts under user auth within
    // AUTH_VALIDITY_SECONDS (signing prompts), while the
    // KEYS_ALIAS_DEVICE_BOUND private key decrypts without a gate
    // (KeySecurityPolicy.DEVICE_BOUND).

    private fun keysPublicKey(alias: String): PublicKey =
        androidKeyStore().getCertificate(alias)?.publicKey ?: ensureKeysKeyPair(alias).public

    private fun keysPrivateKey(alias: String): PrivateKey =
        (androidKeyStore().getKey(alias, null) as? PrivateKey) ?: ensureKeysKeyPair(alias).private

    /**
     * Return the RSA identity-keys keypair for [alias], creating it on
     * first use. The user-authentication gate is applied only to
     * [KEYS_ALIAS]; [KEYS_ALIAS_DEVICE_BOUND] is generated without one.
     *
     * Serialized on a process-wide lock (the AndroidKeyStore alias is
     * process-global, and this manager is instantiated per [WalletStorage])
     * and double-checked under it: if a valid RSA pair is already present —
     * because another thread raced us to first use — it is reused, never
     * deleted. Only an absent or wrong-type (stale symmetric) entry is
     * dropped and regenerated. Without this, two concurrent first-use writes
     * could both regenerate, and the second's `deleteEntry` would orphan the
     * public key the first already encrypted with, leaving that stored
     * private key undecryptable by the surviving alias.
     */
    private fun ensureKeysKeyPair(alias: String): KeyPair = synchronized(KEYS_ALIAS_LOCK) {
        val authGated = alias == KEYS_ALIAS
        val keyStore = androidKeyStore()
        val existingPrivate = keyStore.getKey(alias, null) as? PrivateKey
        val existingCert = keyStore.getCertificate(alias)
        if (existingPrivate != null && existingCert != null) {
            // A valid RSA pair already exists (possibly just created by a
            // thread that raced us) — reuse it, never delete it.
            return@synchronized KeyPair(existingCert.publicKey, existingPrivate)
        }
        // Absent, or a stale symmetric entry from an earlier build (which
        // gated encrypt too and broke unprompted writes) — drop and recreate.
        runCatching { keyStore.deleteEntry(alias) }

        fun spec(strongBox: Boolean): KeyGenParameterSpec {
            val builder = KeyGenParameterSpec.Builder(
                alias,
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
            if (authGated) {
                builder.setUserAuthenticationRequired(true)
                // setUserAuthenticationParameters is API 30+ (Android 11); on
                // the minSdk-29 (Android 10) floor fall back to the deprecated
                // pre-30 time-bound API. Pre-30 the key accepts any enrolled
                // authenticator for the window; the STRONG|DEVICE_CREDENTIAL
                // restriction (and the AuthPrompt that requests it) still
                // applies on 30+.
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    builder.setUserAuthenticationParameters(
                        AUTH_VALIDITY_SECONDS,
                        KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                    )
                } else {
                    @Suppress("DEPRECATION")
                    builder.setUserAuthenticationValidityDurationSeconds(AUTH_VALIDITY_SECONDS)
                }
            }
            if (strongBox) builder.setIsStrongBoxBacked(true)
            return builder.build()
        }

        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_RSA,
            ANDROID_KEYSTORE,
        )
        try {
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

        /**
         * Non-auth-gated identity-keys alias, selected by
         * [KeySecurityPolicy.DEVICE_BOUND]. Distinct from [KEYS_ALIAS]
         * because Keystore auth parameters are fixed at generation — the two
         * policies can never share one alias.
         */
        const val KEYS_ALIAS_DEVICE_BOUND = "org.dashfoundation.wallet.keys.devicebound"

        /** Auth window for the auth-gated identity-keys alias, in seconds. */
        const val AUTH_VALIDITY_SECONDS = 30

        /** Whether [alias] is one of the RSA-wrapped identity-keys aliases. */
        fun isIdentityKeysAlias(alias: String): Boolean =
            alias == KEYS_ALIAS || alias == KEYS_ALIAS_DEVICE_BOUND

        // Guards first-use creation/migration of the process-global
        // identity-keys entries across concurrent callers (and across the
        // per-WalletStorage KeystoreManager instances).
        private val KEYS_ALIAS_LOCK = Any()

        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val AES_TRANSFORMATION = "AES/GCM/NoPadding"
        private const val RSA_TRANSFORMATION = "RSA/ECB/OAEPWithSHA-256AndMGF1Padding"
        private const val RSA_KEY_SIZE = 2048
        private const val GCM_TAG_BITS = 128
    }
}
