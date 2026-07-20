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
 * - `privkey.<pubkeyHex>` — identity private keys (keys alias: RSA
 *   public-key encrypt / auth-gated private-key decrypt)
 *
 * Consuming apps should exclude this DataStore from Android's default app-data
 * backup — Keystore keys are device-bound and never restored, so a backed-up
 * blob can never be decrypted on the new device. See
 * `res/xml/dash_sdk_backup_rules.xml` and `res/xml/dash_sdk_data_extraction_rules.xml`
 * for ready-made exclusion rules and the manifest snippet to reference them.
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

    /**
     * Deleted-wallet ids rejected by [storePrivateKey] / [storeIfAbsent]
     * until [clearTombstone] un-marks them — guarded by [privateKeyMutex]
     * like every other mutation here. Process-lifetime only: NOT persisted
     * across process restart, and deliberately cleared on (re-)creation
     * rather than kept forever, because wallet ids are deterministic
     * functions of seed+network — deleting a wallet and re-importing the
     * same recovery phrase later in the same process reuses the same id,
     * and that is a real, supported recovery flow, not a resurrection bug.
     */
    private val tombstonedWalletIds = mutableSetOf<String>()

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

        /**
         * True if any wallet OTHER than [excludingWalletId] still claims
         * [pubkeyHex] in its durable owner index — i.e. a sibling wallet
         * pre-stored this alias but hasn't (yet) committed a `public_keys`
         * row for it, so a committed-row-only reference check would miss
         * it and delete a sibling's key out from under it.
         */
        suspend fun isOwnedByAnotherWallet(pubkeyHex: String, excludingWalletId: ByteArray): Boolean

        /**
         * Mark [walletId] as deleted. Subsequent [storePrivateKey] /
         * [storeIfAbsent] calls for it are rejected (thrown as
         * [WalletTombstonedException]) until [clearTombstone] un-marks it —
         * closes the window where an app-level coroutine that started
         * before deletion (e.g. an in-flight identity-key preview/derive)
         * completes its store AFTER deletion finished, resurrecting the
         * wallet's owner-index entry with fresh ciphertext.
         */
        suspend fun tombstoneWallet(walletId: ByteArray)
    }

    private val privateKeyExclusionScope = object : PrivateKeyExclusion {
        override suspend fun deletePrivateKeys(pubkeyHexes: Collection<String>) =
            deletePrivateKeysLocked(pubkeyHexes)

        override suspend fun deleteOwnerIndex(walletId: ByteArray) {
            store.edit { it.remove(ownerIndexKey(walletId.toHex())) }
        }

        override suspend fun isOwnedByAnotherWallet(
            pubkeyHex: String,
            excludingWalletId: ByteArray,
        ): Boolean {
            val excludingHex = excludingWalletId.toHex()
            val normalized = pubkeyHex.lowercase()
            return store.data.first().asMap().any { (key, value) ->
                key.name.startsWith(PRIVKEY_OWNERS_PREFIX) &&
                    key.name.removePrefix(PRIVKEY_OWNERS_PREFIX) != excludingHex &&
                    (value as? Set<*>)?.contains(normalized) == true
            }
        }

        override suspend fun tombstoneWallet(walletId: ByteArray) {
            tombstonedWalletIds += walletId.toHex()
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

    /**
     * Clear a deletion tombstone for [walletId] — called when a wallet with
     * that (deterministic, seed-derived) id is (re-)created, so a prior
     * delete-then-reimport doesn't permanently reject its stores. A no-op
     * if [walletId] was never tombstoned.
     */
    suspend fun clearTombstone(walletId: ByteArray) {
        privateKeyMutex.withLock { tombstonedWalletIds -= walletId.toHex() }
    }

    private fun rejectIfTombstonedLocked(ownerWalletId: ByteArray) {
        if (ownerWalletId.toHex() in tombstonedWalletIds) {
            throw WalletTombstonedException(ownerWalletId)
        }
    }

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
            // No tombstone check when ownerWalletId is null: a null owner
            // was never recorded in any owner index either, so it can't be
            // resurrecting a deleted wallet's discoverable state the way a
            // durable-owner write could. No caller on the derive/register
            // path passes null today.
            if (ownerWalletId != null) rejectIfTombstonedLocked(ownerWalletId)
            storePrivateKeyEntryLocked(pubkeyHex, privateKey, ownerWalletId)
        }
    }

    /**
     * Delete each of [pubkeyHexes] that no wallet OTHER than
     * [excludingWalletId] durably owns, ATOMICALLY with that ownership
     * check — a rolled-back changeset round's cleanup needs this, not two
     * separate calls (a check via [PrivateKeyExclusion.isOwnedByAnotherWallet]
     * then a delete via [deletePrivateKeys]): a sibling wallet's
     * [storeIfAbsent] could adopt one of these aliases in the window
     * between two separately-locked calls and lose its just-adopted key
     * to this delete anyway. Returns the hexes actually deleted (a subset
     * of [pubkeyHexes] — the rest are retained because another wallet
     * owns them, not because anything failed).
     *
     * A retained alias (owned elsewhere) still gets [excludingWalletId]
     * removed from ITS OWN owner-index entry for that alias — this is a
     * rollback: [excludingWalletId]'s round that created the alias failed,
     * so it never legitimately owned it, only the OTHER wallet that
     * adopted it via [storeIfAbsent] does. Leaving [excludingWalletId]'s
     * claim in place would strand a phantom owner: a later delete of the
     * real owner would see [excludingWalletId] still listed, wrongly
     * retain the ciphertext, and never clean up either index.
     */
    suspend fun deleteUnownedPrivateKeys(
        pubkeyHexes: Collection<String>,
        excludingWalletId: ByteArray,
    ): Set<String> {
        if (pubkeyHexes.isEmpty()) return emptySet()
        return privateKeyMutex.withLock {
            val toDelete = pubkeyHexes.filterTo(mutableSetOf()) { hex ->
                !privateKeyExclusionScope.isOwnedByAnotherWallet(hex, excludingWalletId)
            }
            if (toDelete.isNotEmpty()) deletePrivateKeysLocked(toDelete)
            val retained = pubkeyHexes.toSet() - toDelete
            if (retained.isNotEmpty()) removeFromOwnerIndexLocked(excludingWalletId, retained)
            toDelete
        }
    }

    /**
     * Drop just [pubkeyHexes] from [walletId]'s owner-index entry, leaving
     * any other aliases it owns intact (unlike [deleteOwnerIndex], which
     * drops the whole entry). Lock must already be held.
     */
    private suspend fun removeFromOwnerIndexLocked(walletId: ByteArray, pubkeyHexes: Collection<String>) {
        if (pubkeyHexes.isEmpty()) return
        val normalized = pubkeyHexes.map { it.lowercase() }.toSet()
        val indexKey = ownerIndexKey(walletId.toHex())
        store.edit { prefs ->
            val current = prefs[indexKey] ?: return@edit
            val next = current - normalized
            if (next.size != current.size) {
                if (next.isEmpty()) prefs.remove(indexKey) else prefs[indexKey] = next
            }
        }
    }

    /**
     * If [pubkeyHex] has no *usable* stored ciphertext — absent, or present
     * but not [isPrivateKeyDecryptable] (a legacy pre-RSA blob) — derive it
     * via [derive] and store it; either way record [ownerWalletId] in the
     * owner index. Returns whether a derive+store actually happened (the
     * "existed before" complement the identity-key persist callback needs).
     *
     * [derive] runs OUTSIDE the private-key lock (it's a native FFI call —
     * [withPrivateKeyExclusion]'s own contract forbids native calls while
     * holding it, since a callback parked on this lock can be holding
     * native locks), so this isn't one atomic transaction: a concurrent
     * caller can derive the same alias in parallel. The lock is retaken
     * before the write and the existence check re-run — the loser's
     * derived bytes are discarded and only its ownership is recorded, so
     * two racing derivations settle on one stored copy either way.
     *
     * This function does NOT scrub the bytes [derive] returns — on every
     * path (stored, discarded as the race loser, or a
     * [WalletTombstonedException] thrown before either) the caller still
     * holds the array `derive` returned and owns zeroing it.
     * [IdentityKeyPrivateKeyDeriver.deriveAndStore], the only caller today,
     * does this in a `finally`; a future second caller must too.
     *
     * Throws [WalletTombstonedException] if [ownerWalletId] was deleted.
     */
    suspend fun storeIfAbsent(
        pubkeyHex: String,
        ownerWalletId: ByteArray,
        derive: suspend () -> ByteArray,
    ): Boolean {
        if (privateKeyMutex.withLock { addOwnerIfUsableLocked(pubkeyHex, ownerWalletId) }) {
            return false
        }
        val derived = derive()
        return privateKeyMutex.withLock {
            if (addOwnerIfUsableLocked(pubkeyHex, ownerWalletId)) {
                false // another writer won the race while this derived
            } else {
                storePrivateKeyEntryLocked(pubkeyHex, derived, ownerWalletId)
                true
            }
        }
    }

    /**
     * If [pubkeyHex] already has a decryptable ciphertext entry (under any
     * owner), record [ownerWalletId]'s ownership and return `true`;
     * otherwise leave everything untouched and return `false`. Lock must
     * already be held.
     */
    private suspend fun addOwnerIfUsableLocked(pubkeyHex: String, ownerWalletId: ByteArray): Boolean {
        rejectIfTombstonedLocked(ownerWalletId)
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return false
        if (!isCurrentKeysBlob(pubkeyHex, encoded, prefs)) return false
        val indexKey = ownerIndexKey(ownerWalletId.toHex())
        store.edit { it[indexKey] = (it[indexKey] ?: emptySet()) + pubkeyHex.lowercase() }
        return true
    }

    /** Encrypt-and-write [privateKey] for [pubkeyHex]; lock must already be held. */
    private suspend fun storePrivateKeyEntryLocked(
        pubkeyHex: String,
        privateKey: ByteArray,
        ownerWalletId: ByteArray?,
    ) {
        val blob = keystore.encrypt(privateKey, alias = KeystoreManager.KEYS_ALIAS)
        val fingerprint = checkNotNull(blob.keyFingerprint) {
            "KEYS_ALIAS encryption must identify the public key that produced its ciphertext"
        }
        store.edit {
            it[privateKeyKey(pubkeyHex)] = encode(blob)
            it[privateKeyFingerprintKey(pubkeyHex)] = fingerprint
            if (ownerWalletId != null) {
                val indexKey = ownerIndexKey(ownerWalletId.toHex())
                it[indexKey] = (it[indexKey] ?: emptySet()) + pubkeyHex.lowercase()
            }
        }
    }

    /**
     * Whether the stored blob for [pubkeyHex] is both structurally an RSA
     * blob and was encrypted under the [KeystoreManager.KEYS_ALIAS] keypair
     * currently in the Keystore — see [KeystoreManager.keysAliasFingerprint].
     * A missing fingerprint (written before this check existed) is treated
     * as unusable rather than trusted, since a stale RSA-shaped blob is
     * indistinguishable from a current one by shape alone. Computing the
     * current fingerprint may generate a missing alias and can therefore
     * fail until a secure lock screen is configured; this is a capability
     * check, not a promise of side-effect-free Keystore access.
     */
    private fun isCurrentKeysBlob(
        pubkeyHex: String,
        encoded: String,
        prefs: Preferences,
    ): Boolean {
        if (!keystore.isKeysBlobDecryptable(decode(encoded))) return false
        val fingerprint = prefs[privateKeyFingerprintKey(pubkeyHex)] ?: return false
        return fingerprint == keystore.keysAliasFingerprint()
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
     * Decrypt the private key for [pubkeyHex]. A stable blob whose stored
     * key fingerprint no longer matches the current [KeystoreManager.KEYS_ALIAS]
     * returns `null`, allowing the caller to re-derive it instead of trying
     * OAEP with an unrelated replacement key. Rotation can still occur
     * between this check and decrypt; that race deliberately remains a
     * fail-closed crypto exception rather than returning stale plaintext.
     *
     * Throws
     * `UserNotAuthenticatedException` when the auth window expired — the
     * caller (KeystoreSigner) routes through [BiometricGate] and retries.
     * Callers must zero the returned array after use.
     */
    suspend fun retrievePrivateKey(pubkeyHex: String): ByteArray? {
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return null
        if (!isCurrentKeysBlob(pubkeyHex, encoded, prefs)) return null
        return keystore.decrypt(decode(encoded), alias = KeystoreManager.KEYS_ALIAS)
    }

    suspend fun deletePrivateKey(pubkeyHex: String) {
        privateKeyMutex.withLock {
            store.edit {
                it.remove(privateKeyKey(pubkeyHex))
                it.remove(privateKeyFingerprintKey(pubkeyHex))
            }
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
            for (pubkeyHex in normalized) {
                prefs.remove(privateKeyKey(pubkeyHex))
                prefs.remove(privateKeyFingerprintKey(pubkeyHex))
            }
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
     * current [KeystoreManager.KEYS_ALIAS] RSA scheme. Blobs written by the
     * pre-RSA AES-GCM scheme survive in the DataStore but lost their key
     * when the RSA pair replaced it, so signing with them can only fail —
     * key-health treats them as missing and offers a re-derive. This never
     * decrypts or prompts, but fingerprint lookup may generate a
     * missing alias and can fail while no secure lock screen is configured.
     */
    suspend fun isPrivateKeyDecryptable(pubkeyHex: String): Boolean {
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return false
        return isCurrentKeysBlob(pubkeyHex, encoded, prefs)
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

    private fun privateKeyFingerprintKey(pubkeyHex: String) =
        stringPreferencesKey(PRIVKEY_FINGERPRINT_PREFIX + pubkeyHex.lowercase())

    private fun ownerIndexKey(walletIdHex: String) =
        stringSetPreferencesKey(PRIVKEY_OWNERS_PREFIX + walletIdHex.lowercase())

    private fun encode(blob: KeystoreManager.EncryptedBlob): String =
        Base64.getEncoder().encodeToString(blob.encode())

    private fun decode(value: String): KeystoreManager.EncryptedBlob =
        KeystoreManager.EncryptedBlob.decode(Base64.getDecoder().decode(value))

    private companion object {
        const val MNEMONIC_PREFIX = "mnemonic."
        const val PRIVKEY_PREFIX = "privkey."

        /** Per-alias [KeystoreManager.keysAliasFingerprint] snapshot, taken at write time. */
        const val PRIVKEY_FINGERPRINT_PREFIX = "privkeyfp."

        /** Durable wallet → alias-hex-set owner index (string-set entries). */
        const val PRIVKEY_OWNERS_PREFIX = "privkeyowners."

        fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
    }
}
