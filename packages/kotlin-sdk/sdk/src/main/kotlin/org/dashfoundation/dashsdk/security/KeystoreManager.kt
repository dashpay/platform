package org.dashfoundation.dashsdk.security

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.util.Log
import java.security.GeneralSecurityException
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.ProviderException
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
 * - [KEYS_ALIAS_AUTH_GATED] `org.dashfoundation.wallet.keys.authgated` —
 *   identity private keys under the default [KeySecurityPolicy.AUTH_GATED],
 *   wrapped by an RSA-2048 OAEP(SHA-256) keypair. The PUBLIC key encrypts and
 *   is never auth-gated, so **storing** a key never prompts (parity with iOS,
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
 * - [KEYS_ALIAS] `org.dashfoundation.wallet.keys` — the **legacy** pre-RSA
 *   alias that wrapped identity keys under an auth-gated AES-256-GCM key.
 *   Retained read-only so existing installs' AES-GCM blobs stay recoverable:
 *   [WalletStorage.retrievePrivateKey] decrypts a legacy blob with it and
 *   migrates the value to [KEYS_ALIAS_AUTH_GATED]. Never written by new keys.
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
// `open` purely so unit tests can substitute a fake that simulates Keystore
// crypto: AndroidKeyStore has no Robolectric provider, so the real encrypt/
// decrypt cannot run on the JVM (see KeySecurityPolicyTest). The seam lets
// WalletStorage's upgrade-recovery ROUTING be exercised across the full blob
// matrix without a device. Production always uses this concrete implementation.
open class KeystoreManager(
    val keySecurityPolicy: KeySecurityPolicy = KeySecurityPolicy.AUTH_GATED,
    /**
     * Whether the device currently has a secure lock screen configured
     * (`KeyguardManager.isDeviceSecure`). Supplied by [WalletStorage] (which
     * holds a `Context`); defaults to `true` for the no-`Context` /
     * unit-test construction path. Consulted at key GENERATION only, to decide
     * whether the lock-screen-bound Keystore parameters are enforceable — see
     * [generateWithLockScreenDegradation] and [lockBoundKeyParamsSupported]
     * (dashpay/platform#4060, no-secure-lock-screen key-gen failure).
     */
    private val deviceSecureProbe: () -> Boolean = { true },
) {

    /**
     * The identity-keys alias this manager targets, per
     * [keySecurityPolicy]: [KEYS_ALIAS_AUTH_GATED] (auth-gated decrypt) or
     * [KEYS_ALIAS_DEVICE_BOUND] (non-gated decrypt). [WalletStorage] passes
     * this to [encrypt]/[decrypt] for identity-key material.
     */
    open val keysAlias: String
        get() = when (keySecurityPolicy) {
            KeySecurityPolicy.AUTH_GATED -> KEYS_ALIAS_AUTH_GATED
            KeySecurityPolicy.DEVICE_BOUND -> KEYS_ALIAS_DEVICE_BOUND
        }

    /**
     * Encrypt [plaintext] under [alias]; returns (iv, ciphertext).
     * [MASTER_ALIAS] uses AES-256-GCM (iv is the GCM nonce). The RSA
     * identity-keys aliases ([KEYS_ALIAS_AUTH_GATED] / [KEYS_ALIAS_DEVICE_BOUND])
     * use the RSA public key (no iv — the blob's iv is empty) and never
     * require authentication, so identity-key writes never prompt. (The legacy
     * [KEYS_ALIAS] is never an encrypt target — new keys use the RSA aliases.)
     */
    open fun encrypt(plaintext: ByteArray, alias: String = MASTER_ALIAS): EncryptedBlob {
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
     * [KEYS_ALIAS_AUTH_GATED] RSA private-key decrypt throws
     * `UserNotAuthenticatedException` when the [AUTH_VALIDITY_SECONDS] auth
     * window is closed — the caller (`KeystoreSigner`) prompts via the
     * `BiometricGate` and retries. The [KEYS_ALIAS_DEVICE_BOUND] decrypt is
     * never auth-gated (see [KeySecurityPolicy.DEVICE_BOUND]). Legacy pre-RSA
     * AES-GCM blobs are NOT handled here — [WalletStorage.retrievePrivateKey]
     * detects them ([isLegacyKeysBlob]) and routes them through
     * [decryptLegacyKeysBlob].
     */
    open fun decrypt(blob: EncryptedBlob, alias: String = MASTER_ALIAS): ByteArray {
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
     * Whether [blob] is structurally an RSA identity-keys blob the current
     * scheme can decrypt: no iv (RSA blobs never carry one) and exactly one
     * RSA block of ciphertext. Legacy pre-RSA AES-GCM blobs carry a GCM nonce
     * in `iv` (see [isLegacyKeysBlob]) and are handled by the migration
     * fallback instead. Never decrypts, so it never prompts for authentication.
     */
    open fun isKeysBlobDecryptable(blob: EncryptedBlob): Boolean =
        blob.iv.isEmpty() && blob.ciphertext.size == RSA_KEY_SIZE / 8

    /**
     * Whether [blob] is a legacy pre-RSA identity-key blob — one written by the
     * old scheme that wrapped identity keys under the auth-gated AES-256-GCM key
     * at [KEYS_ALIAS]. Such blobs carry a GCM nonce in `iv`; the RSA scheme
     * never does. Structural check only — never decrypts, never prompts.
     * Decrypt these via [decryptLegacyKeysBlob].
     */
    open fun isLegacyKeysBlob(blob: EncryptedBlob): Boolean = blob.iv.isNotEmpty()

    /**
     * Whether the legacy AES key at [KEYS_ALIAS] still exists, so a legacy
     * AES-GCM identity-key blob is recoverable via [decryptLegacyKeysBlob] (and
     * can be migrated to the RSA scheme). A Keystore presence check only — no
     * decrypt, no prompt. False once an older build already deleted the key, in
     * which case any surviving legacy AES blob is unrecoverable and needs a
     * re-derive. Note [KEYS_ALIAS] is single-typed: it holds EITHER this AES key
     * OR the former RSA keypair ([hasLegacyRsaKeysKey]) OR nothing — never both.
     */
    open fun hasLegacyKeysKey(): Boolean =
        (androidKeyStore().getKey(KEYS_ALIAS, null) as? SecretKey) != null

    /**
     * Whether [KEYS_ALIAS] currently holds the **former RSA identity-keys
     * keypair** from the pre-alias-split scheme (dashpay/platform#4060) — the
     * intermediate scheme that wrapped identity keys as empty-IV RSA/OAEP blobs
     * under [KEYS_ALIAS] before the AUTH_GATED/DEVICE_BOUND alias split moved new
     * keys to dedicated aliases. Only this key can open those blobs, so
     * [WalletStorage.retrievePrivateKey] uses it as the recovery fallback and
     * migrates the value to the current policy alias. Keystore presence check
     * only — never decrypts, never prompts.
     */
    open fun hasLegacyRsaKeysKey(): Boolean =
        (androidKeyStore().getKey(KEYS_ALIAS, null) as? PrivateKey) != null

    /**
     * Whether the current identity-keys [alias] (one of the RSA policy aliases)
     * already holds an RSA private key — a NON-generating presence check, unlike
     * [decrypt]/[keysPrivateKey] which provision the keypair on first use. Lets
     * [WalletStorage.isPrivateKeyDecryptable] report an empty-IV RSA blob's
     * recoverability, and [WalletStorage.retrievePrivateKey] route the upgrade
     * fast path, without ever creating a key. Never decrypts, never prompts.
     */
    open fun hasIdentityKeysKey(alias: String): Boolean =
        (androidKeyStore().getKey(alias, null) as? PrivateKey) != null

    /**
     * Decrypt a legacy pre-RSA identity-key [blob] with the retained AES-GCM key
     * at [KEYS_ALIAS], or return `null` if that key is gone (the blob is then
     * unrecoverable). Deliberately fetches the existing key WITHOUT generating a
     * fresh one — a new key could never open an old blob. The legacy key was
     * auth-gated, so this throws `UserNotAuthenticatedException` when the auth
     * window is closed, exactly as the RSA auth-gated decrypt does; the caller
     * ([KeystoreSigner]) prompts and retries.
     */
    open fun decryptLegacyKeysBlob(blob: EncryptedBlob): ByteArray? {
        val legacyKey = androidKeyStore().getKey(KEYS_ALIAS, null) as? SecretKey ?: return null
        val cipher = Cipher.getInstance(AES_TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, legacyKey, GCMParameterSpec(GCM_TAG_BITS, blob.iv))
        return cipher.doFinal(blob.ciphertext)
    }

    /**
     * Decrypt an empty-IV RSA identity-key [blob] with the retained **former RSA
     * keypair** at [KEYS_ALIAS] (the pre-alias-split scheme,
     * dashpay/platform#4060), or return `null` if [KEYS_ALIAS] no longer holds an
     * RSA private key. Deliberately fetches the existing key WITHOUT generating a
     * fresh one. Like the current-scheme RSA decrypt this key was auth-gated, so
     * it throws `UserNotAuthenticatedException` when the auth window is closed —
     * the caller ([KeystoreSigner]) prompts and retries. A blob that this key
     * cannot open (e.g. it actually belongs to the current policy alias) surfaces
     * as a JCE `BadPaddingException`, which [WalletStorage.retrievePrivateKey]
     * treats as "not this key".
     */
    open fun decryptLegacyRsaKeysBlob(blob: EncryptedBlob): ByteArray? {
        val legacyRsaPrivate =
            androidKeyStore().getKey(KEYS_ALIAS, null) as? PrivateKey ?: return null
        val cipher = Cipher.getInstance(RSA_TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, legacyRsaPrivate, oaepSpec())
        return cipher.doFinal(blob.ciphertext)
    }

    /**
     * Whether [blob] opens under the non-auth-gated [KEYS_ALIAS_DEVICE_BOUND]
     * sibling — probed PROMPT-FREE (that alias never gates) and WITHOUT
     * generating a key (guarded on [hasIdentityKeysKey] presence; recovered
     * plaintext is scrubbed immediately).
     *
     * An RSA-OAEP ciphertext opens under exactly one keypair, so a positive
     * result PROVES the blob belongs to the DEVICE_BOUND sibling rather than to
     * the current (auth-gated) policy alias. The key-health check
     * ([WalletStorage.isPrivateKeyDecryptable]) uses this to DISPROVE a locked
     * auth-gated policy alias's ownership of a sibling-written blob: without it,
     * that alias's `UserNotAuthenticatedException` (thrown at `cipher.init` before
     * the ciphertext is ever examined) is indistinguishable from a locked but
     * legitimate owner and would be mis-reported as "decryptable"
     * (dashpay/platform#4060, finding b80a15c93339). Because
     * [WalletStorage.retrievePrivateKey] under the current policy never falls back
     * to the sibling alias, such a blob is genuinely unrecoverable and must drive
     * the re-derive/repair path.
     *
     * Returns false when the current policy already IS
     * [KeySecurityPolicy.DEVICE_BOUND] (the sibling would be the policy alias
     * itself, already probed on the normal path) or when no DEVICE_BOUND key is
     * present. Residual: the symmetric case — a locked auth-gated FORMER RSA key
     * at [KEYS_ALIAS] whose ownership can't be disproved without a prompt — is
     * still reported decryptable until the first real unlock surfaces the
     * BadPadding (documented in [WalletStorage.isPrivateKeyDecryptable]).
     */
    open fun opensUnderNonGatedDeviceBoundSibling(blob: EncryptedBlob): Boolean {
        if (keySecurityPolicy == KeySecurityPolicy.DEVICE_BOUND) return false
        if (!hasIdentityKeysKey(KEYS_ALIAS_DEVICE_BOUND)) return false
        return try {
            decrypt(blob, KEYS_ALIAS_DEVICE_BOUND).fill(0)
            true
        } catch (e: GeneralSecurityException) {
            false
        }
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
        fun spec(strongBox: Boolean, lockBound: Boolean): KeyGenParameterSpec {
            val builder = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
            // `setUnlockedDeviceRequired(true)` binds the key to a screen-lock
            // (an "unlocked device" is only meaningful when a secure lock screen
            // exists); KeyMint rejects generate_key for it on a lockless device.
            // Dropped when no secure lock screen is configured (see
            // [generateWithLockScreenDegradation]).
            if (lockBound) builder.setUnlockedDeviceRequired(true)
            if (strongBox) builder.setIsStrongBoxBacked(true)
            return builder.build()
        }

        val generator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            ANDROID_KEYSTORE,
        )
        return generateWithLockScreenDegradation(alias) { strongBox, lockBound ->
            generator.init(spec(strongBox, lockBound))
            generator.generateKey()
        }
    }

    // ── Identity-keys aliases: RSA-2048 OAEP keypair per alias ──
    // Public key encrypts (never auth-gated → unprompted writes); the
    // KEYS_ALIAS_AUTH_GATED private key decrypts under user auth within
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
     * [KEYS_ALIAS_AUTH_GATED]; [KEYS_ALIAS_DEVICE_BOUND] is generated without
     * one.
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
        val authGated = alias == KEYS_ALIAS_AUTH_GATED
        val keyStore = androidKeyStore()
        val existingPrivate = keyStore.getKey(alias, null) as? PrivateKey
        val existingCert = keyStore.getCertificate(alias)
        if (existingPrivate != null && existingCert != null) {
            // A valid RSA pair already exists (possibly just created by a
            // thread that raced us) — reuse it, never delete it.
            return@synchronized KeyPair(existingCert.publicKey, existingPrivate)
        }
        // Absent, or a partial/wrong-type entry at THIS RSA alias — drop and
        // recreate. Note [alias] is one of the RSA aliases
        // ([KEYS_ALIAS_AUTH_GATED] / [KEYS_ALIAS_DEVICE_BOUND]), never the
        // legacy [KEYS_ALIAS], so the pre-RSA AES key that still wraps existing
        // installs' identity-key blobs is never deleted here.
        runCatching { keyStore.deleteEntry(alias) }

        fun spec(strongBox: Boolean, lockBound: Boolean): KeyGenParameterSpec {
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
            // Both lock-screen-bound parameters — `setUnlockedDeviceRequired`
            // and (auth-gated only) `setUserAuthenticationRequired` — require a
            // secure lock screen to exist; KeyMint rejects generate_key for them
            // otherwise. On a lockless device both are dropped: the AUTH_GATED
            // alias then degrades to a non-gated key (there is no lock-screen
            // authenticator to gate on anyway). See
            // [generateWithLockScreenDegradation].
            if (lockBound) {
                builder.setUnlockedDeviceRequired(true)
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
                            KeyProperties.AUTH_BIOMETRIC_STRONG or
                                KeyProperties.AUTH_DEVICE_CREDENTIAL,
                        )
                    } else {
                        @Suppress("DEPRECATION")
                        builder.setUserAuthenticationValidityDurationSeconds(AUTH_VALIDITY_SECONDS)
                    }
                }
            }
            if (strongBox) builder.setIsStrongBoxBacked(true)
            return builder.build()
        }

        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_RSA,
            ANDROID_KEYSTORE,
        )
        generateWithLockScreenDegradation(alias) { strongBox, lockBound ->
            generator.initialize(spec(strongBox, lockBound))
            generator.generateKeyPair()
        }
    }

    /**
     * Run [generate] — which builds+initializes the spec for `(strongBox,
     * lockBound)` and produces the key — degrading gracefully when the device
     * has no secure lock screen.
     *
     * THE APP MUST WORK WITHOUT A SCREEN LOCK (product decision,
     * dashpay/platform#4060). The lock-screen-bound Keystore parameters
     * (`setUnlockedDeviceRequired`, and for the auth-gated alias
     * `setUserAuthenticationRequired`) require a secure lock screen; without one
     * KeyMint fails `generate_key` (observed on-device: `ProviderException`
     * "Keystore key generation failed" wrapping `KeyStoreException` system error,
     * KeyMint 10309). Strategy:
     *   1. If [deviceSecureProbe] reports NO secure lock screen, build the key
     *      WITHOUT the lock-bound params up front and log the downgrade.
     *   2. Otherwise attempt with the lock-bound params; if generation still
     *      fails with the no-secure-lock-screen signature (a race — the lock was
     *      removed after the probe — or an OEM that rejects it despite a probe
     *      saying secure), retry once WITHOUT them and log the downgrade.
     * Each attempt keeps the existing StrongBox→TEE fallback.
     *
     * Existing keys are never regenerated (callers check presence first), so this
     * only affects FIRST-USE creation on a lockless device; a key already
     * provisioned with the lock-bound params is untouched.
     */
    private fun <T> generateWithLockScreenDegradation(
        alias: String,
        generate: (strongBox: Boolean, lockBound: Boolean) -> T,
    ): T {
        fun withStrongBoxFallback(lockBound: Boolean): T =
            try {
                generate(true, lockBound)
            } catch (_: StrongBoxUnavailableException) {
                generate(false, lockBound)
            }

        val lockBoundSupported = lockBoundKeyParamsSupported(deviceSecureProbe())
        if (!lockBoundSupported) {
            Log.w(
                TAG,
                "No secure lock screen (KeyguardManager.isDeviceSecure=false); generating " +
                    "'$alias' WITHOUT lock-screen binding so the wallet works without a " +
                    "screen lock (dashpay/platform#4060).",
            )
            return withStrongBoxFallback(lockBound = false)
        }
        return try {
            withStrongBoxFallback(lockBound = true)
        } catch (e: ProviderException) {
            if (!isNoSecureLockScreenKeyGenFailure(e)) throw e
            Log.w(
                TAG,
                "Key generation for '$alias' was rejected for requiring a secure lock " +
                    "screen even though the device reported secure (lock removed mid-flight, " +
                    "or an OEM quirk); retrying WITHOUT lock-screen binding " +
                    "(dashpay/platform#4060).",
                e,
            )
            withStrongBoxFallback(lockBound = false)
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

        /**
         * **Legacy** identity-keys alias. Across the SDK's history this single
         * alias has held, in turn, two now-superseded wrapping keys, so on an
         * upgraded install it may currently contain EITHER:
         *  - the original **auth-gated AES-256-GCM** key (the pre-RSA scheme —
         *    blobs carry a GCM nonce; recovered via [decryptLegacyKeysBlob],
         *    gated by [hasLegacyKeysKey]), OR
         *  - the **former RSA-2048 OAEP keypair** (the intermediate
         *    pre-alias-split scheme — empty-IV blobs; recovered via
         *    [decryptLegacyRsaKeysBlob], gated by [hasLegacyRsaKeysKey],
         *    dashpay/platform#4060).
         * It is single-typed (one key at a time) and retained (never deleted,
         * never regenerated) purely so existing installs' blobs stay decryptable
         * across the upgrade to the aliased RSA scheme:
         * [WalletStorage.retrievePrivateKey] falls back to whichever key is
         * present and migrates the recovered value to [keysAlias]. New identity
         * keys are NEVER written here — see [keysAlias].
         */
        const val KEYS_ALIAS = "org.dashfoundation.wallet.keys"

        /**
         * Auth-gated identity-keys alias: the RSA-2048 OAEP wrapping pair for
         * [KeySecurityPolicy.AUTH_GATED]. A **new** alias distinct from the
         * legacy [KEYS_ALIAS] so upgrading installs keep their old AES key (and
         * thus their old blobs) intact instead of having it deleted out from
         * under them. Fixed Keystore auth parameters also mean it can never
         * share an alias with [KEYS_ALIAS_DEVICE_BOUND].
         */
        const val KEYS_ALIAS_AUTH_GATED = "org.dashfoundation.wallet.keys.authgated"

        /**
         * Non-auth-gated identity-keys alias, selected by
         * [KeySecurityPolicy.DEVICE_BOUND]. Distinct from the other identity
         * aliases because Keystore auth parameters are fixed at generation — the
         * two policies can never share one alias.
         */
        const val KEYS_ALIAS_DEVICE_BOUND = "org.dashfoundation.wallet.keys.devicebound"

        /** Auth window for the auth-gated identity-keys alias, in seconds. */
        const val AUTH_VALIDITY_SECONDS = 30

        /**
         * Whether [alias] is one of the RSA-wrapped identity-keys aliases new
         * keys are written under. The legacy [KEYS_ALIAS] (AES-GCM) is
         * deliberately excluded — it is read-only, reached only via the
         * migration fallback, never through the RSA encrypt/decrypt path.
         */
        fun isIdentityKeysAlias(alias: String): Boolean =
            alias == KEYS_ALIAS_AUTH_GATED || alias == KEYS_ALIAS_DEVICE_BOUND

        /**
         * Whether the lock-screen-bound key-generation parameters
         * (`setUnlockedDeviceRequired`, and for the auth-gated alias
         * `setUserAuthenticationRequired`) may be applied. They require a secure
         * lock screen — KeyMint rejects `generate_key` for them otherwise — so
         * they are only usable when [deviceSecure]
         * (`KeyguardManager.isDeviceSecure`) is true. Pure so the
         * parameter-selection logic is unit-testable without an Android runtime
         * (dashpay/platform#4060).
         */
        internal fun lockBoundKeyParamsSupported(deviceSecure: Boolean): Boolean = deviceSecure

        /**
         * Whether [t]'s cause chain is the KeyMint "generate_key needs a secure
         * lock screen" rejection: a key-generation `ProviderException` (message
         * "Keystore key generation failed") and/or an `android.security`
         * `KeyStoreException` system error raised from `generate_key` (observed
         * on-device as internal Keystore code 4 / KeyMint 10309 after the lock
         * screen was removed). Used only as the retry-decision for
         * [generateWithLockScreenDegradation]'s safety net. Pure and
         * JVM-testable — matches by exception type name and message so it needs
         * no Android classes (dashpay/platform#4060).
         */
        internal fun isNoSecureLockScreenKeyGenFailure(t: Throwable): Boolean {
            var cur: Throwable? = t
            var sawKeyGenFailure = false
            while (cur != null) {
                val name = cur::class.java.name
                val msg = cur.message.orEmpty()
                if (name.endsWith("ProviderException") &&
                    msg.contains("key generation failed", ignoreCase = true)
                ) {
                    sawKeyGenFailure = true
                }
                if (name.endsWith("KeyStoreException")) {
                    // A Keystore-side generation system error reached via a
                    // key-gen ProviderException, or one that names generate_key /
                    // a lock-screen requirement directly.
                    if (sawKeyGenFailure ||
                        msg.contains("generate_key", ignoreCase = true) ||
                        msg.contains("lock screen", ignoreCase = true)
                    ) {
                        return true
                    }
                }
                cur = cur.cause
            }
            return false
        }

        private const val TAG = "KeystoreManager"

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
