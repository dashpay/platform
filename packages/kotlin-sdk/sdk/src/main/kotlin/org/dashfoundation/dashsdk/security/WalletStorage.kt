package org.dashfoundation.dashsdk.security

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
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
 * - `privkey.<pubkeyHex>` — identity private keys (the [keystore]'s
 *   [KeystoreManager.keysAlias]: RSA public-key encrypt / private-key
 *   decrypt that is auth-gated or not per the keystore's
 *   [KeySecurityPolicy])
 *
 * The identity-key security policy is fixed by the [keystore] this storage
 * wraps; use the policy-taking constructor to opt into
 * [KeySecurityPolicy.DEVICE_BOUND] (see [KeySecurityPolicy] for the
 * semantics and the stability requirement). The default is the historical
 * [KeySecurityPolicy.AUTH_GATED] behavior, unchanged.
 */
class WalletStorage(
    context: Context,
    private val keystore: KeystoreManager = KeystoreManager(),
) {
    /**
     * Construct with an explicit identity-key [keySecurityPolicy] —
     * convenience for host apps that don't otherwise need to touch
     * [KeystoreManager]. `WalletStorage(context)` keeps the
     * [KeySecurityPolicy.AUTH_GATED] default.
     */
    constructor(context: Context, keySecurityPolicy: KeySecurityPolicy) :
        this(context, KeystoreManager(keySecurityPolicy))

    private val store = context.secretsStore

    /** The identity-key security policy this storage was constructed with. */
    val keySecurityPolicy: KeySecurityPolicy get() = keystore.keySecurityPolicy

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

        /**
         * Drop a wallet's owner-index entry after its aliases were swept.
         * Aliases retained by the sweep (shared with another wallet) stay
         * discoverable through the OTHER wallet's index / Room rows.
         */
        suspend fun deleteOwnerIndex(walletId: ByteArray)
    }

    private val privateKeyExclusionScope = object : PrivateKeyExclusion {
        override suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>) =
            deletePrivateKeysLocked(pubkeyHexes)

        override suspend fun deleteOwnerIndex(walletId: ByteArray) {
            store.edit { it.remove(ownerIndexKey(walletId.toHex())) }
        }
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
     * [KeystoreManager.keysAlias] RSA public key. Public-key encrypt is
     * never auth-gated (under either [KeySecurityPolicy]), so this never
     * prompts and never throws `UserNotAuthenticatedException` — matching
     * iOS's silent identity-key write, and letting the persistence callback
     * (which runs on a Rust Tokio thread under the wallet-manager write
     * lock, where a prompt is impossible) store keys. Per the CLAUDE.md
     * doctrine this is the one allowed Kotlin-side persistence of key
     * material: Rust derives, we encrypt. Reads ([retrievePrivateKey])
     * require auth only under [KeySecurityPolicy.AUTH_GATED].
     */
    /**
     * @param ownerWalletId when given, the alias is also recorded in the
     *   wallet's DURABLE owner index (`privkeyowners.<walletIdHex>` — a
     *   string-set entry written in the SAME atomic edit). The index is
     *   what makes the alias discoverable by wallet deletion when no
     *   committed `public_keys` row references it yet: app-prestored keys
     *   for an in-flight registration, and deriver writes orphaned by
     *   process death (the in-memory pending-alias fence does not survive
     *   termination). Pass it whenever the owning wallet is known.
     */
    suspend fun storePrivateKey(
        pubkeyHex: String,
        privateKey: ByteArray,
        ownerWalletId: ByteArray? = null,
    ) {
        privateKeyMutex.withLock {
            val blob = keystore.encrypt(privateKey, alias = keystore.keysAlias)
            store.edit {
                it[privateKeyKey(pubkeyHex)] = encode(blob)
                if (ownerWalletId != null) {
                    val indexKey = ownerIndexKey(ownerWalletId.toHex())
                    it[indexKey] = (it[indexKey] ?: emptySet()) + pubkeyHex.lowercase()
                }
            }
        }
    }

    /**
     * The wallet's durable owner-index entries — pubkey hexes of aliases
     * stored on its behalf (see [storePrivateKey]). Read-only snapshot;
     * call inside [withPrivateKeyExclusion] when it must be consistent
     * with a following delete.
     */
    suspend fun ownedPrivateKeyAliases(walletId: ByteArray): Set<String> =
        store.data.first()[ownerIndexKey(walletId.toHex())] ?: emptySet()

    /**
     * Decrypt the private key for [pubkeyHex]. Under
     * [KeySecurityPolicy.AUTH_GATED] this throws
     * `UserNotAuthenticatedException` when the auth window expired — the
     * caller (KeystoreSigner) routes through [BiometricGate] and retries;
     * under [KeySecurityPolicy.DEVICE_BOUND] it never auth-gates.
     * Callers must zero the returned array after use.
     */
    suspend fun retrievePrivateKey(pubkeyHex: String): ByteArray? {
        val encoded = store.data.first()[privateKeyKey(pubkeyHex)] ?: return null
        return keystore.decrypt(decode(encoded), alias = keystore.keysAlias)
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
        val normalized = pubkeyHexes.map { it.lowercase() }.toSet()
        store.edit { prefs ->
            for (pubkeyHex in normalized) prefs.remove(privateKeyKey(pubkeyHex))
            // Keep every owner index accurate in the same atomic commit:
            // a deleted alias must leave all wallets' index sets, or a
            // later wallet deletion would "discover" a ghost.
            prefs.asMap().keys
                .filter { it.name.startsWith(PRIVKEY_OWNERS_PREFIX) }
                .forEach { prefKey ->
                    val setKey = stringSetPreferencesKey(prefKey.name)
                    val current = prefs[setKey] ?: return@forEach
                    val next = current - normalized
                    if (next.size != current.size) {
                        if (next.isEmpty()) prefs.remove(setKey) else prefs[setKey] = next
                    }
                }
        }
    }

    suspend fun hasPrivateKey(pubkeyHex: String): Boolean =
        store.data.first().contains(privateKeyKey(pubkeyHex))

    /**
     * Whether the blob stored for [pubkeyHex] is decryptable under the
     * current [KeystoreManager.keysAlias] RSA scheme. Blobs written by the
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

    private fun ownerIndexKey(walletIdHex: String) =
        stringSetPreferencesKey(PRIVKEY_OWNERS_PREFIX + walletIdHex.lowercase())

    private fun encode(blob: KeystoreManager.EncryptedBlob): String =
        Base64.getEncoder().encodeToString(blob.encode())

    private fun decode(value: String): KeystoreManager.EncryptedBlob =
        KeystoreManager.EncryptedBlob.decode(Base64.getDecoder().decode(value))

    private companion object {
        const val MNEMONIC_PREFIX = "mnemonic."
        const val PRIVKEY_PREFIX = "privkey."

        /** Durable wallet → alias-hex-set owner index (string-set entries). */
        const val PRIVKEY_OWNERS_PREFIX = "privkeyowners."

        fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
    }
}
