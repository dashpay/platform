package org.dashfoundation.dashsdk.security

import android.security.keystore.UserNotAuthenticatedException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import java.util.concurrent.ConcurrentHashMap
import org.dashfoundation.dashsdk.Network
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
 * ([SignerNative.signWithMnemonicAndPath]). Direct port of
 * `KeychainSigner.signPlatformAddressOnDemand` on iOS.
 */
class KeystoreSigner(
    private val storage: WalletStorage,
    private val network: Network,
    private val biometricGate: BiometricGate?,
    private val platformAddressDao: PlatformAddressDao,
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
            try {
                // Platform-payment addresses (0xFF) have no stored private
                // key — derive on demand from (mnemonic, derivationPath).
                if (keyType == PLATFORM_ADDRESS_HASH_KEY_TYPE) {
                    signPlatformAddressOnDemand(pubkeyBytes, data, completionToken)
                    return@launch
                }
                signWithStoredKey(pubkeyBytes, data, completionToken)
            } catch (e: Exception) {
                SignerNative.completeSign(
                    completionToken,
                    null,
                    e.message ?: "signing failed",
                )
            }
        }
    }

    /** Identity-key path: decrypt the stored private key and sign. */
    private suspend fun signWithStoredKey(
        pubkeyBytes: ByteArray,
        data: ByteArray,
        completionToken: Long,
    ) {
        var key: ByteArray? = null
        try {
            val storageKey = storageKeyFor(pubkeyBytes)
            key = retrieveKeyWithAuth(storageKey)
            if (key == null) {
                SignerNative.completeSign(
                    completionToken,
                    null,
                    "no private key stored for ${storageKey.take(16)}…",
                )
                return
            }
            val signature = SignerNative.signWithPrivateKey(key, network.ffiValue, data)
            if (signature != null) {
                SignerNative.completeSign(completionToken, signature, null)
            } else {
                SignerNative.completeSign(completionToken, null, "signing returned no data")
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
        completionToken: Long,
    ) {
        val hashHex = addressHash.joinToString("") { "%02x".format(it) }
        val row = platformAddressDao.getByAddressHash(addressHash)
        if (row == null) {
            SignerNative.completeSign(
                completionToken,
                null,
                "no platform address row for $hashHex",
            )
            return
        }
        if (row.derivationPath.isEmpty()) {
            SignerNative.completeSign(
                completionToken,
                null,
                "platform address $hashHex has no derivation path",
            )
            return
        }
        val mnemonic = storage.retrieveMnemonic(row.walletId)
        if (mnemonic == null) {
            SignerNative.completeSign(
                completionToken,
                null,
                "no mnemonic stored for wallet of platform address $hashHex",
            )
            return
        }
        val signature = SignerNative.signWithMnemonicAndPath(
            mnemonic,
            row.derivationPath,
            network.ffiValue,
            data,
        )
        if (signature != null) {
            SignerNative.completeSign(completionToken, signature, null)
        } else {
            SignerNative.completeSign(completionToken, null, "signing returned no data")
        }
    }

    override fun canSignWith(pubkeyBytes: ByteArray, keyType: Int): Boolean = try {
        if (keyType == PLATFORM_ADDRESS_HASH_KEY_TYPE) {
            // Prerequisites for the derive-and-sign path: a row with a
            // non-empty derivation path plus a stored wallet mnemonic.
            // Existence-only — hasMnemonic never decrypts, so no plaintext
            // is materialized for a mere capability check.
            runBlocking {
                val row = platformAddressDao.getByAddressHash(pubkeyBytes)
                row != null &&
                    row.derivationPath.isNotEmpty() &&
                    storage.hasMnemonic(row.walletId)
            }
        } else {
            runBlocking { storage.hasPrivateKey(storageKeyFor(pubkeyBytes)) }
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

    private companion object {
        /**
         * FFI dispatch tag the Rust `Signer<PlatformAddress>` vtable ships
         * instead of a `KeyType` discriminant when the "pubkey bytes" are a
         * 20-byte platform-address hash. Mirrors
         * `SIGNER_KEY_TYPE_PLATFORM_ADDRESS_HASH` (0xFF) in
         * `rs-sdk-ffi/src/signer.rs`; arrives here as the unsigned value 255.
         */
        const val PLATFORM_ADDRESS_HASH_KEY_TYPE: Int = 0xFF
    }
}
