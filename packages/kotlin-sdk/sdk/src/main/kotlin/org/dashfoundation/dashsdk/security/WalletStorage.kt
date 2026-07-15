package org.dashfoundation.dashsdk.security

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.util.Base64

private val Context.secretsStore: DataStore<Preferences> by preferencesDataStore(
    name = "org.dashfoundation.wallet.secrets",
)

/**
 * Encrypted-at-rest secret storage — the Android counterpart of
 * `WalletStorage.swift` (iOS Keychain items under service
 * `org.dashfoundation.wallet`).
 *
 * Values are ciphertext under [KeystoreManager]'s non-exportable Keystore
 * keys, stored base64 in a dedicated Preferences DataStore.
 * Key layout mirrors the iOS account naming:
 * - `mnemonic.<walletIdHex>` — wallet mnemonics (master alias, AES-GCM)
 * - `privkey.<pubkeyHex>` — identity private keys (keys alias: RSA
 *   public-key encrypt / auth-gated private-key decrypt)
 */
class WalletStorage(
    context: Context,
    private val keystore: KeystoreManager = KeystoreManager(),
) {
    private val store = context.secretsStore

    /**
     * Serializes every `privkey.*` alias mutation. A single DataStore
     * `edit` is already atomic, but compound sequences (wallet deletion's
     * enumerate → refcount → batch-delete) must not interleave with a
     * concurrent [storePrivateKey] — an alias written after the snapshot
     * would survive deletion and lose its discoverable Room row to the
     * cascade. Writers take this internally; compound readers-then-writers
     * use [withPrivateKeyExclusion]. The mutex is NOT reentrant: code
     * running inside [withPrivateKeyExclusion] must use the scope's own
     * operations, never the public locking entry points.
     */
    private val privateKeyMutex = Mutex()

    /** Operations available while the private-key exclusion is held. */
    interface PrivateKeyExclusion {
        /** [WalletStorage.deletePrivateKeys], lock already held. */
        suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>)
    }

    private val privateKeyExclusionScope = object : PrivateKeyExclusion {
        override suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>) =
            deletePrivateKeysLocked(pubkeyHexes)
    }

    /**
     * Run [block] with the private-key mutation lock held, so no
     * [storePrivateKey] / [deletePrivateKey] can interleave with a
     * compound snapshot-then-delete sequence. The block must not call
     * the public locking entry points (non-reentrant); it must also never
     * call into native code (a persistence callback parked on this lock
     * inside [storePrivateKey] can be holding native locks).
     */
    suspend fun <T> withPrivateKeyExclusion(
        block: suspend PrivateKeyExclusion.() -> T,
    ): T = privateKeyMutex.withLock { privateKeyExclusionScope.block() }

    // ── Mnemonics ─────────────────────────────────────────────────────

    suspend fun storeMnemonic(walletId: ByteArray, mnemonic: String) {
        val blob = keystore.encrypt(mnemonic.encodeToByteArray())
        store.edit { it[mnemonicKey(walletId)] = encode(blob) }
    }

    /**
     * Decrypt the mnemonic as a display `String`. For explicit
     * user-facing reveal flows ONLY (seed backup, biometric reveal) —
     * a String cannot be scrubbed afterwards. Programmatic consumers
     * (the FFI resolver, signers) must use [retrieveMnemonicUtf8].
     */
    suspend fun retrieveMnemonic(walletId: ByteArray): String? {
        val encoded = store.data.first()[mnemonicKey(walletId)] ?: return null
        val plain = keystore.decrypt(decode(encoded))
        val phrase = plain.decodeToString()
        plain.fill(0)
        return phrase
    }

    /**
     * Decrypt the mnemonic as raw UTF-8 bytes, never materializing a JVM
     * `String` (the iOS `retrieveMnemonicUTF8Bytes` discipline). The
     * caller OWNS the returned array and MUST `fill(0)` it as soon as the
     * bytes are consumed — unlike a String, a ByteArray can actually be
     * scrubbed, so the plaintext exposure window is bounded by the call
     * instead of by the garbage collector.
     */
    suspend fun retrieveMnemonicUtf8(walletId: ByteArray): ByteArray? {
        val encoded = store.data.first()[mnemonicKey(walletId)] ?: return null
        return keystore.decrypt(decode(encoded))
    }

    /**
     * Whether a mnemonic is stored for [walletId]. Existence-only — never
     * decrypts, never materializes plaintext (Swift `hasMnemonic(for:)`).
     */
    suspend fun hasMnemonic(walletId: ByteArray): Boolean =
        store.data.first().contains(mnemonicKey(walletId))

    suspend fun deleteMnemonic(walletId: ByteArray) {
        store.edit { it.remove(mnemonicKey(walletId)) }
    }

    /** Wallet ids (hex) that have a stored mnemonic — drives orphan detection. */
    suspend fun listWalletIdsWithMnemonic(): List<String> =
        store.data.first().asMap().keys
            .map { it.name }
            .filter { it.startsWith(MNEMONIC_PREFIX) }
            .map { it.removePrefix(MNEMONIC_PREFIX) }

    // ── Identity private keys ─────────────────────────────────────────

    /**
     * Store raw private-key bytes for [pubkeyHex], encrypted with the
     * [KeystoreManager.KEYS_ALIAS] RSA public key. Public-key encrypt is
     * never auth-gated, so this never prompts and never throws
     * `UserNotAuthenticatedException` — matching iOS's silent identity-key
     * write, and letting the persistence callback (which runs on a Rust
     * Tokio thread under the wallet-manager write lock, where a prompt is
     * impossible) store keys. Per the CLAUDE.md doctrine this is the one
     * allowed Kotlin-side persistence of key material: Rust derives, we
     * encrypt. Reads ([retrievePrivateKey]) still require auth.
     */
    suspend fun storePrivateKey(pubkeyHex: String, privateKey: ByteArray) {
        privateKeyMutex.withLock {
            val blob = keystore.encrypt(privateKey, alias = KeystoreManager.KEYS_ALIAS)
            store.edit { it[privateKeyKey(pubkeyHex)] = encode(blob) }
        }
    }

    /**
     * Decrypt the private key for [pubkeyHex]. Throws
     * `UserNotAuthenticatedException` when the auth window expired — the
     * caller (KeystoreSigner) routes through [BiometricGate] and retries.
     * Callers must zero the returned array after use.
     */
    suspend fun retrievePrivateKey(pubkeyHex: String): ByteArray? {
        val encoded = store.data.first()[privateKeyKey(pubkeyHex)] ?: return null
        return keystore.decrypt(decode(encoded), alias = KeystoreManager.KEYS_ALIAS)
    }

    suspend fun deletePrivateKey(pubkeyHex: String) {
        privateKeyMutex.withLock {
            store.edit { it.remove(privateKeyKey(pubkeyHex)) }
        }
    }

    /**
     * Remove every `privkey.<pubkeyHex>` entry in [pubkeyHexes] in ONE
     * DataStore `edit` — a single atomic commit, so either every alias is
     * removed or none are. The wallet-deletion sweep depends on this:
     * per-key deletes commit independently, and a failure between them
     * would leave a live wallet missing some of its signing keys.
     */
    suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>) {
        privateKeyMutex.withLock { deletePrivateKeysLocked(pubkeyHexes) }
    }

    private suspend fun deletePrivateKeysLocked(pubkeyHexes: Collection<String>) {
        if (pubkeyHexes.isEmpty()) return
        store.edit { prefs ->
            for (pubkeyHex in pubkeyHexes) prefs.remove(privateKeyKey(pubkeyHex))
        }
    }

    suspend fun hasPrivateKey(pubkeyHex: String): Boolean =
        store.data.first().contains(privateKeyKey(pubkeyHex))

    /**
     * Whether the blob stored for [pubkeyHex] is decryptable under the
     * current [KeystoreManager.KEYS_ALIAS] RSA scheme. Blobs written by the
     * pre-RSA AES-GCM scheme survive in the DataStore but lost their key
     * when the RSA pair replaced it, so signing with them can only fail —
     * key-health treats them as missing and offers a re-derive. Structural
     * check only: never decrypts, never prompts.
     */
    suspend fun isPrivateKeyDecryptable(pubkeyHex: String): Boolean {
        val encoded = store.data.first()[privateKeyKey(pubkeyHex)] ?: return false
        return keystore.isKeysBlobDecryptable(decode(encoded))
    }

    /** All entry names (masked listing for the Keystore Explorer screen). */
    suspend fun listEntryNames(): List<String> =
        store.data.first().asMap().keys.map { it.name }.sorted()

    suspend fun deleteAll() {
        // Clears privkey.* entries too — take the same exclusion as the
        // targeted mutators so it can't interleave with a compound sweep.
        privateKeyMutex.withLock {
            store.edit { it.clear() }
        }
    }

    private fun mnemonicKey(walletId: ByteArray) =
        stringPreferencesKey(MNEMONIC_PREFIX + walletId.toHex())

    private fun privateKeyKey(pubkeyHex: String) =
        stringPreferencesKey(PRIVKEY_PREFIX + pubkeyHex.lowercase())

    private fun encode(blob: KeystoreManager.EncryptedBlob): String =
        Base64.getEncoder().encodeToString(blob.encode())

    private fun decode(value: String): KeystoreManager.EncryptedBlob =
        KeystoreManager.EncryptedBlob.decode(Base64.getDecoder().decode(value))

    private companion object {
        const val MNEMONIC_PREFIX = "mnemonic."
        const val PRIVKEY_PREFIX = "privkey."

        fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
    }
}
