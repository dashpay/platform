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
 * [lookupPubkeyHex] maps raw pubkey bytes + key type to the storage key;
 * key_type 0xFF (platform-address-hash) rows are registered by the wallet
 * manager as addresses are derived, mirroring the PersistentPlatformAddress
 * lookup in Swift.
 */
class KeystoreSigner(
    private val storage: WalletStorage,
    private val network: Network,
    private val biometricGate: BiometricGate?,
) : NativeSignerBridge(), AutoCloseable {

    val nativeHandle: Long = SignerNative.createSigner(this)

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
                    return@launch
                }
                val signature = SignerNative.signWithPrivateKey(key, network.ffiValue, data)
                if (signature != null) {
                    SignerNative.completeSign(completionToken, signature, null)
                } else {
                    SignerNative.completeSign(completionToken, null, "signing returned no data")
                }
            } catch (e: Exception) {
                SignerNative.completeSign(
                    completionToken,
                    null,
                    e.message ?: "signing failed",
                )
            } finally {
                key?.fill(0)
            }
        }
    }

    override fun canSignWith(pubkeyBytes: ByteArray, keyType: Int): Boolean = try {
        runBlocking { storage.hasPrivateKey(storageKeyFor(pubkeyBytes)) }
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
        SignerNative.destroySigner(nativeHandle)
    }
}
