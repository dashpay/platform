package org.dashfoundation.dashsdk.security

import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.UserNotAuthenticatedException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import java.util.concurrent.ConcurrentHashMap
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.NativeSignerBridge
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.dashfoundation.dashsdk.persistence.dao.PlatformAddressDao

/**
 * Keystore-backed signer — port of `KeychainSigner.swift` (~774 LOC).
 *
 * Rust invokes [signAsync] whenever a state transition needs a signature.
 * The private key for the requested public key is decrypted from
 * [WalletStorage] (Keystore-wrapped, auth-gated alias), the signature is
 * produced by the one-shot Rust helpers
 * (`SignerNative.signWithPrivateKey` — no crypto in Kotlin, per doctrine),
 * the key buffer is zeroed, and the completion fires exactly once.
 *
 * When the Keystore auth window has expired, the [biometricGate] (an
 * Activity-bound BiometricPrompt supplied by the app) is invoked and the
 * decrypt retried — the Rust side tolerates the latency (5-minute bound).
 *
 * Identity keys (`keyType < 5`) are looked up directly from [storage] by
 * public-key hex. Platform-payment addresses (`keyType == 0xFF`,
 * `Signer<PlatformAddress>`) are handled separately: their private keys are
 * NEVER persisted (the production wallet is external-signable and holds only
 * account XPUBs), so they are derived on demand from `(mnemonic,
 * derivationPath)` — the `derivationPath` + `walletId` are resolved from the
 * `PlatformAddressEntity` row via [platformAddressDao], the mnemonic from
 * [storage], and the derive-and-sign happens entirely inside Rust
 * ([SignerNative.signWithMnemonicAndPathInto]). Direct port of
 * `KeychainSigner.signPlatformAddressOnDemand` on iOS.
 */
class KeystoreSigner(
    private val storage: WalletStorage,
    private val network: Network,
    private val biometricGate: BiometricGate?,
    private val platformAddressDao: PlatformAddressDao,
    /**
     * Invoked (on the signer's IO scope, best-effort) when a sign attempt
     * classifies a [KeyPermanentlyInvalidatedException] for the given
     * storage-key pubkey hex — the wiring point for durable pending-repair
     * bookkeeping. Load-bearing for LEGACY-alias-backed keys (#4060 round-2
     * finding 3): the legacy aliases are read-only — there is no deletion
     * boundary — so the cheap capability check (`hasLegacyKeysKey`) keeps
     * reporting an invalidated legacy key signable forever and the restart
     * reconstruction never seeds it; this hook is the only signal that
     * makes the repair path reachable outside the health sheet.
     * `PlatformWalletManager` wires it to record the invalidation on the
     * Room rows and re-seed `pendingIdentityKeys`.
     */
    private val onSigningKeyInvalidated: (suspend (pubkeyHex: String) -> Unit)? = null,
) : NativeSignerBridge(), AutoCloseable {

    private val handleRef =
        java.util.concurrent.atomic.AtomicLong(SignerNative.createSigner(this))

    val nativeHandle: Long
        get() = handleRef.get().also { check(it != 0L) { "KeystoreSigner has been closed" } }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /**
     * pubkeyHex → alternate storage key (e.g. platform-address-hash rows).
     * Direct pubkey lookups need no registration.
     */
    private val aliases = ConcurrentHashMap<String, String>()

    /** Register an alternate lookup (address-hash → storage pubkeyHex). */
    fun registerAlias(lookupHex: String, storageKeyHex: String) {
        aliases[lookupHex.lowercase()] = storageKeyHex.lowercase()
    }

    override fun signAsync(
        pubkeyBytes: ByteArray,
        keyType: Int,
        data: ByteArray,
        completionToken: Long,
    ) {
        // Return immediately (vtable contract); work on IO.
        scope.launch {
            // Idempotent completion guard. The native completeSign reclaims the
            // PendingSign box via `Box::from_raw(token)` (rs-unified-sdk-jni),
            // so a SECOND call with the same token is a double-free (undefined
            // behavior) — the ABI requires EXACTLY one completion per token.
            // Routing every completion through this guard makes each path fire
            // at most once, including the cancellation path below that can race
            // with an already-fired typed completion on the invalidation route.
            val completed = java.util.concurrent.atomic.AtomicBoolean(false)
            val complete: (ByteArray?, Int, String?) -> Unit = { sig, code, msg ->
                if (completed.compareAndSet(false, true)) {
                    SignerNative.completeSign(completionToken, sig, code, msg)
                }
            }
            try {
                // Platform-payment addresses (0xFF) have no stored private
                // key — derive on demand from (mnemonic, derivationPath).
                if (keyType == PLATFORM_ADDRESS_HASH_KEY_TYPE) {
                    signPlatformAddressOnDemand(pubkeyBytes, data, complete)
                    return@launch
                }
                signWithStoredKey(pubkeyBytes, data, complete)
            } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
                // Structured-concurrency cancellation: a teardown of the signer's
                // IO scope, or cancellation surfaced by the suspend invalidation
                // callback. The `scope.launch` job is INDEPENDENT of the Rust
                // request, so rethrowing alone does NOT cancel the native
                // receiver — the JNI PendingSign token would leak until the
                // five-minute SIGN_ASYNC_COMPLETION_TIMEOUT. Release it NOW,
                // exactly once: if a typed key-unavailable completion already
                // fired on the invalidation path this is a no-op (guard),
                // otherwise it completes with the generic cancellation result.
                // Then rethrow so the coroutine still unwinds — NEVER swallow
                // cancellation (dashpay/platform#4183 review).
                complete(null, SignerNative.SIGNER_ERROR_CODE_GENERIC, "signing cancelled")
                throw cancellation
            } catch (e: Exception) {
                // Classify before completing: a KeyPermanentlyInvalidatedException
                // anywhere in the sign path means the key is unavailable until
                // re-derived, and must surface as the typed code on the FIRST
                // attempt — not as an opaque generic failure (#4060 round-2
                // finding 2).
                complete(null, completionErrorCodeFor(e), e.message ?: "signing failed")
            }
        }
    }

    /** Identity-key path: decrypt the stored private key and sign. */
    private suspend fun signWithStoredKey(
        pubkeyBytes: ByteArray,
        data: ByteArray,
        complete: (ByteArray?, Int, String?) -> Unit,
    ) {
        var key: ByteArray? = null
        try {
            val storageKey = storageKeyFor(pubkeyBytes)
            key = try {
                retrieveKeyWithAuth(storageKey)
            } catch (e: KeyPermanentlyInvalidatedException) {
                // The Keystore key that wraps this blob was permanently
                // invalidated (biometric/credential re-enrollment; the
                // generation-checked alias cleanup already ran inside
                // KeystoreManager.decrypt for policy aliases). The key is
                // unavailable until re-derived — complete with the TYPED
                // code on this first attempt instead of letting the generic
                // catch-all label it an opaque signing failure (#4060
                // round-2 finding 2).
                val keyUnavailableMessage =
                    "${DashSdkError.PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER} " +
                        "${storageKey.take(16)}… (key permanently invalidated: " +
                        "${e.message ?: "re-enrollment"})"
                // Record the invalidation durably first (finding 3) —
                // best-effort: bookkeeping failure must not eat the typed
                // completion.
                try {
                    onSigningKeyInvalidated?.invoke(storageKey)
                } catch (cancellation: kotlin.coroutines.cancellation.CancellationException) {
                    // The invalidation callback is suspend; runCatching would
                    // have swallowed its cancellation one layer too low and let
                    // the sign continue. The key-unavailable result is ALREADY
                    // known here, so complete the native token with the TYPED
                    // code before unwinding — otherwise the cancellation would
                    // skip the completion below and strand the PendingSign token
                    // until the five-minute timeout. Then rethrow so
                    // structured-concurrency cancellation still propagates. The
                    // outer cancellation catch's completion is a no-op (the
                    // idempotent guard) — no double-free
                    // (dashpay/platform#4183 review).
                    complete(
                        null,
                        SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE,
                        keyUnavailableMessage,
                    )
                    throw cancellation
                } catch (_: Throwable) {
                    // Best-effort bookkeeping: a non-cancellation failure must
                    // not eat the typed completion below.
                }
                complete(
                    null,
                    SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE,
                    keyUnavailableMessage,
                )
                return
            }
            if (key == null) {
                // The typed SIGNER_ERROR_CODE_KEY_UNAVAILABLE rides the
                // completion ABI and comes back as platform-wallet code 31 →
                // DashSdkError.PlatformWallet.SigningKeyUnavailable
                // (dashpay/platform#4060 finding 7). The MESSAGE_MARKER text
                // is ALSO kept for the #4191 merge-order transition (its
                // marker-based classification predates the typed code) and
                // as defense in depth for any conversion path that loses the
                // machine prefix — NOT for mixed old-native/new-Kotlin
                // builds, which the completion JNI arity change makes
                // unsupported outright.
                complete(
                    null,
                    SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE,
                    "${DashSdkError.PlatformWallet.SigningKeyUnavailable.MESSAGE_MARKER} " +
                        "${storageKey.take(16)}…",
                )
                return
            }
            val signature = SignerNative.signWithPrivateKey(key, network.ffiValue, data)
            if (signature != null) {
                complete(
                    signature,
                    SignerNative.SIGNER_ERROR_CODE_GENERIC,
                    null,
                )
            } else {
                complete(
                    null,
                    SignerNative.SIGNER_ERROR_CODE_GENERIC,
                    "signing returned no data",
                )
            }
        } finally {
            key?.fill(0)
        }
    }

    /**
     * Platform-address (`keyType == 0xFF`) path — port of
     * `KeychainSigner.signPlatformAddressOnDemand`. Resolves the
     * `(walletId, derivationPath)` from the `PlatformAddressEntity` row keyed
     * by the 20-byte [addressHash], retrieves the wallet mnemonic, and hands
     * both to the Rust derive-and-sign FFI. The derived key never crosses JNI.
     */
    private suspend fun signPlatformAddressOnDemand(
        addressHash: ByteArray,
        data: ByteArray,
        complete: (ByteArray?, Int, String?) -> Unit,
    ) {
        val hashHex = addressHash.joinToString("") { "%02x".format(it) }
        // The hash may live in several wallets' rows (per-wallet
        // uniqueness); any row with a derivation path and a stored
        // mnemonic derives the same private key — the hash pins the key —
        // so scan for the first signable candidate.
        val rows = platformAddressDao.getAllByAddressHash(addressHash)
        if (rows.isEmpty()) {
            complete(
                null,
                SignerNative.SIGNER_ERROR_CODE_GENERIC,
                "no platform address row for $hashHex",
            )
            return
        }
        val row = rows.firstOrNull {
            it.derivationPath.isNotEmpty() && storage.hasMnemonic(it.walletId)
        }
        if (row == null) {
            complete(
                null,
                SignerNative.SIGNER_ERROR_CODE_GENERIC,
                "no signable platform address row for $hashHex " +
                    "(no candidate has both a derivation path and a stored mnemonic)",
            )
            return
        }
        // The phrase crosses JNI as raw UTF-8 bytes the caller scrubs after the
        // call — never an un-scrubbable JVM String (the resolveMnemonicInto
        // discipline, applied here to the signing path).
        val mnemonicUtf8 = storage.retrieveMnemonicUtf8(row.walletId)
        if (mnemonicUtf8 == null) {
            complete(
                null,
                SignerNative.SIGNER_ERROR_CODE_GENERIC,
                "no mnemonic stored for wallet of platform address $hashHex",
            )
            return
        }
        val signature = signWithScrubbedMnemonic(
            mnemonicUtf8,
            row.derivationPath,
            network.ffiValue,
            data,
        ) { m, path, net, payload ->
            SignerNative.signWithMnemonicAndPathInto(m, path, net, payload)
        }
        if (signature != null) {
            complete(
                signature,
                SignerNative.SIGNER_ERROR_CODE_GENERIC,
                null,
            )
        } else {
            complete(
                null,
                SignerNative.SIGNER_ERROR_CODE_GENERIC,
                "signing returned no data",
            )
        }
    }

    override fun canSignWith(pubkeyBytes: ByteArray, keyType: Int): Boolean = try {
        if (keyType == PLATFORM_ADDRESS_HASH_KEY_TYPE) {
            // Prerequisites for the derive-and-sign path: a row with a
            // non-empty derivation path plus a stored wallet mnemonic.
            // Existence-only — hasMnemonic never decrypts, so no plaintext
            // is materialized for a mere capability check.
            runBlocking {
                platformAddressDao.getAllByAddressHash(pubkeyBytes).any { row ->
                    row.derivationPath.isNotEmpty() && storage.hasMnemonic(row.walletId)
                }
            }
        } else {
            // Ciphertext presence is not signing capability after Android
            // replaces KEYS_ALIAS: the stale blob remains in DataStore but
            // can never be decrypted by the replacement private key.
            runBlocking { storage.isPrivateKeyDecryptable(storageKeyFor(pubkeyBytes)) }
        }
    } catch (_: Exception) {
        false
    }

    private fun storageKeyFor(pubkeyBytes: ByteArray): String {
        val hex = pubkeyBytes.joinToString("") { "%02x".format(it) }
        return aliases[hex] ?: hex
    }

    /**
     * Decrypt the key; on an expired auth window, run the biometric gate
     * once and retry — mirroring KeychainSigner's LAContext flow.
     */
    private suspend fun retrieveKeyWithAuth(storageKey: String): ByteArray? =
        try {
            storage.retrievePrivateKey(storageKey)
        } catch (e: UserNotAuthenticatedException) {
            val gate = biometricGate ?: throw e
            when (gate.authenticate(title = "Authorize signing")) {
                BiometricGate.AuthOutcome.AUTHORIZED -> storage.retrievePrivateKey(storageKey)
                else -> throw e
            }
        }

    override fun close() {
        val h = handleRef.getAndSet(0)
        if (h != 0L) SignerNative.destroySigner(h)
    }

    companion object {
        /**
         * FFI dispatch tag the Rust `Signer<PlatformAddress>` vtable ships
         * instead of a `KeyType` discriminant when the "pubkey bytes" are a
         * 20-byte platform-address hash. Mirrors
         * `SIGNER_KEY_TYPE_PLATFORM_ADDRESS_HASH` (0xFF) in
         * `rs-sdk-ffi/src/signer.rs`; arrives here as the unsigned value 255.
         */
        private const val PLATFORM_ADDRESS_HASH_KEY_TYPE: Int = 0xFF

        /**
         * Structured completion code for a sign-path failure [t]:
         * [KeyPermanentlyInvalidatedException] is a "key unavailable until
         * re-derived" signal and maps to
         * [SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE] (→ platform-wallet
         * code 31 → `DashSdkError.PlatformWallet.SigningKeyUnavailable`);
         * everything else stays [SignerNative.SIGNER_ERROR_CODE_GENERIC].
         * Note `UserNotAuthenticatedException` never reaches this — the
         * biometric-gate retry handles it, and an unhandled one is a generic
         * failure, not a missing key. Factored pure so the classification is
         * unit-testable without the native signer handle (#4060 round-2
         * finding 2).
         */
        internal fun completionErrorCodeFor(t: Throwable): Int =
            if (t is KeyPermanentlyInvalidatedException) {
                SignerNative.SIGNER_ERROR_CODE_KEY_UNAVAILABLE
            } else {
                SignerNative.SIGNER_ERROR_CODE_GENERIC
            }
    }
}

/**
 * Runs [sign] with the caller-owned mnemonic bytes and scrubs them on every
 * exit path (success, null, or throw). The phrase is passed as a [ByteArray]
 * precisely so it can be zeroed — this helper owns that discipline, so the
 * platform-address signing path never leaves the plaintext on the JVM heap
 * past the call. [sign] must consume the bytes synchronously (it does: the JNI
 * call copies them into a Rust-owned scrubbed buffer before returning).
 */
internal fun signWithScrubbedMnemonic(
    mnemonicUtf8: ByteArray,
    derivationPath: String,
    network: Int,
    data: ByteArray,
    sign: (ByteArray, String, Int, ByteArray) -> ByteArray?,
): ByteArray? = try {
    sign(mnemonicUtf8, derivationPath, network, data)
} finally {
    mnemonicUtf8.fill(0)
}
