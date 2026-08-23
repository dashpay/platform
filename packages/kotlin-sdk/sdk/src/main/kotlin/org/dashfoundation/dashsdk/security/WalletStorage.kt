package org.dashfoundation.dashsdk.security

import android.app.KeyguardManager
import android.content.Context
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.UserNotAuthenticatedException
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.security.GeneralSecurityException
import java.util.Base64
import kotlin.coroutines.cancellation.CancellationException

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
 * Consuming apps should exclude this DataStore from Android's default app-data
 * backup — Keystore keys are device-bound and never restored, so a backed-up
 * blob can never be decrypted on the new device. See
 * `res/xml/dash_sdk_backup_rules.xml` and `res/xml/dash_sdk_data_extraction_rules.xml`
 * for ready-made exclusion rules and the manifest snippet to reference them.
 *
 * The identity-key security policy is fixed by the [keystore] this storage
 * wraps; use the policy-taking constructor to opt into
 * [KeySecurityPolicy.DEVICE_BOUND] (see [KeySecurityPolicy] for the
 * semantics and the stability requirement). The default is the historical
 * [KeySecurityPolicy.AUTH_GATED] behavior, unchanged.
 */
class WalletStorage(
    context: Context,
    private val keystore: KeystoreManager =
        KeystoreManager(
            deviceSecureProbe = deviceSecureProbe(context),
            deviceLockStateProbe = deviceLockStateProbe(context),
        ),
) {
    /**
     * Construct with an explicit identity-key [keySecurityPolicy] —
     * convenience for host apps that don't otherwise need to touch
     * [KeystoreManager]. `WalletStorage(context)` keeps the
     * [KeySecurityPolicy.AUTH_GATED] default.
     */
    constructor(context: Context, keySecurityPolicy: KeySecurityPolicy) :
        this(
            context,
            KeystoreManager(
                keySecurityPolicy,
                deviceSecureProbe(context),
                deviceLockStateProbe = deviceLockStateProbe(context),
            ),
        )

    private val store = context.secretsStore

    /** The identity-key security policy this storage was constructed with. */
    val keySecurityPolicy: KeySecurityPolicy get() = keystore.keySecurityPolicy

    /**
     * The [KeySecurityPolicy] identity keys are EFFECTIVELY protected with
     * right now — [KeySecurityPolicy.DEVICE_BOUND] while a requested
     * [KeySecurityPolicy.AUTH_GATED] key cannot be (or was not) provisioned
     * with its authentication gate, i.e. on a device with no secure lock
     * screen. See [KeystoreManager.effectiveKeySecurityPolicy]
     * (dashpay/platform#4060).
     */
    fun effectiveKeySecurityPolicy(): KeySecurityPolicy =
        keystore.effectiveKeySecurityPolicy()

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

    // ── Device lock state ─────────────────────────────────────────────

    /**
     * Fail fast with [KeystoreDeviceLockedException] if `KeyguardManager`
     * reports the device LOCKED right now (`isDeviceLocked` — locked AND
     * secured). A [MASTER_ALIAS][KeystoreManager.MASTER_ALIAS] Keystore
     * operation on a lock-screen device would be denied anyway
     * (`setUnlockedDeviceRequired`), so callers about to do irreversible
     * orchestration around such an operation — `PlatformWalletManager.
     * createWallet`, whose native create precedes [storeMnemonic] — call
     * this FIRST and fail with nothing created and nothing to roll back.
     * Prompt-free, no Keystore access; a no-op when the device is unlocked
     * (including keyguard-showing-but-not-secured states).
     */
    fun ensureDeviceUnlocked(operation: String) {
        val state = keystore.sampleDeviceLockState()
        if (state.isDeviceLocked) {
            throw KeystoreDeviceLockedException(
                alias = KeystoreManager.MASTER_ALIAS,
                operation = operation,
                lockState = state,
            )
        }
    }

    // ── Mnemonics ─────────────────────────────────────────────────────

    /**
     * Encrypt and persist the mnemonic under the
     * [MASTER_ALIAS][KeystoreManager.MASTER_ALIAS] AES key.
     *
     * Retries the FALSE-LOCKED Keystore denial only: when the encrypt is
     * denied as device-locked but the sampled `KeyguardManager` state says
     * the device is NOT actually locked ([KeystoreDeviceLockedException]
     * with `deviceReportsLocked == false` — the Keystore2 lock-state
     * misreporting defect, hit on two QA devices during wallet creation),
     * the store is retried up to 3 times over ~2s (the
     * [DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS] backoff schedule) before the
     * exception propagates. A GENUINELY locked
     * device (`deviceReportsLocked == true`) fails fast with no retry —
     * waiting 2s cannot unlock a phone; the caller retries after unlock.
     */
    suspend fun storeMnemonic(walletId: ByteArray, mnemonic: String) {
        val plaintext = mnemonic.encodeToByteArray()
        var attempt = 0
        while (true) {
            try {
                val blob = keystore.encrypt(plaintext)
                store.edit { it[mnemonicKey(walletId)] = encode(blob) }
                return
            } catch (e: KeystoreDeviceLockedException) {
                if (e.deviceReportsLocked || attempt >= DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS.size) {
                    throw e
                }
                val delayMs = DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS[attempt]
                attempt++
                Log.w(
                    TAG,
                    "storeMnemonic: Keystore denied encrypt as device-locked but " +
                        "KeyguardManager reports UNLOCKED (${e.lockState}) — the false-locked " +
                        "Keystore2 defect; retry $attempt/" +
                        "${DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS.size} in ${delayMs}ms",
                    e,
                )
                delay(delayMs)
            }
        }
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
     * FORCED variant of [storeIfAbsent] for the repair path
     * (dashpay/platform#4060 finding 6): derive and store [pubkeyHex]'s
     * private key UNCONDITIONALLY, replacing whatever entry exists — never
     * the [addOwnerIfUsableLocked] short-circuit, whose shape+fingerprint
     * usability check can be satisfied by a blob that does not actually
     * decrypt (a stale-but-matching corner) and would silently skip the
     * re-derive a repair exists to perform. Blob, fingerprint, and alias tag
     * are replaced in ONE atomic edit; ownership is recorded like any store.
     *
     * Same outside-the-lock derive discipline as [storeIfAbsent] ([derive]
     * is a native FFI call — never run it holding [privateKeyMutex]), and
     * the same scrubbing contract: this function does NOT zero the bytes
     * [derive] returns; the caller owns that on every path. The tombstone
     * check runs both before the derive (fail fast) and again under the
     * write lock (a deletion can win the race while deriving).
     *
     * Throws [WalletTombstonedException] if [ownerWalletId] was deleted.
     */
    suspend fun replacePrivateKey(
        pubkeyHex: String,
        ownerWalletId: ByteArray,
        derive: suspend () -> ByteArray,
    ) {
        privateKeyMutex.withLock { rejectIfTombstonedLocked(ownerWalletId) }
        val derived = derive()
        privateKeyMutex.withLock {
            rejectIfTombstonedLocked(ownerWalletId)
            storePrivateKeyEntryLocked(pubkeyHex, derived, ownerWalletId)
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

    /**
     * Encrypt-and-write [privateKey] for [pubkeyHex]; lock must already be
     * held. The encrypt resolves the EFFECTIVE write alias (the policy alias,
     * or its lockless DEVICE_BOUND degradation — see
     * [KeystoreManager.encryptForIdentityKeys]); blob, write-time key
     * fingerprint, and the producing-alias tag land in ONE atomic edit so
     * reads can route the decrypt to the exact alias that wrote the blob.
     */
    private suspend fun storePrivateKeyEntryLocked(
        pubkeyHex: String,
        privateKey: ByteArray,
        ownerWalletId: ByteArray?,
    ) {
        val encrypted = keystore.encryptForIdentityKeys(privateKey)
        val blob = encrypted.blob
        val fingerprint = encrypted.keyFingerprint
        val alias = encrypted.alias
        store.edit {
            it[privateKeyKey(pubkeyHex)] = encode(blob)
            it[privateKeyFingerprintKey(pubkeyHex)] = fingerprint
            it[privateKeyAliasKey(pubkeyHex)] = alias
            if (ownerWalletId != null) {
                val indexKey = ownerIndexKey(ownerWalletId.toHex())
                it[indexKey] = (it[indexKey] ?: emptySet()) + pubkeyHex.lowercase()
            }
        }
    }

    /**
     * The RSA identity-keys alias recorded as having written [pubkeyHex]'s
     * blob (`privkeyalias.<pubkeyHex>`), falling back to the current policy
     * alias when the tag is missing (blobs written before the tag existed —
     * backward compatible) or names something that is not a policy alias
     * (never trusted: only the two RSA policy aliases are ever accepted as
     * decrypt targets).
     */
    private fun recordedKeysAliasFor(pubkeyHex: String, prefs: Preferences): String {
        val tagged = prefs[privateKeyAliasKey(pubkeyHex)]
        return if (tagged != null && KeystoreManager.isIdentityKeysAlias(tagged)) {
            tagged
        } else {
            keystore.keysAlias
        }
    }

    /**
     * Whether the stored blob for [pubkeyHex] is both structurally an RSA
     * blob and was encrypted under the [KeystoreManager.keysAlias] keypair
     * currently in the Keystore — see [KeystoreManager.keysAliasFingerprint].
     * A missing fingerprint (written before this check existed) is treated
     * as unusable rather than trusted, since a stale RSA-shaped blob is
     * indistinguishable from a current one by shape alone. Read-only: it uses
     * the non-generating [KeystoreManager.keysAliasFingerprintOrNull], so an
     * absent alias (e.g. just deleted by invalidation cleanup) returns `false`
     * without regenerating a keypair — a capability probe on the Rust signer
     * callback must never block that thread or mutate Keystore state.
     */
    private fun isCurrentKeysBlob(
        pubkeyHex: String,
        encoded: String,
        prefs: Preferences,
    ): Boolean {
        if (!keystore.isKeysBlobDecryptable(decode(encoded))) return false
        val fingerprint = prefs[privateKeyFingerprintKey(pubkeyHex)] ?: return false
        val alias = recordedKeysAliasFor(pubkeyHex, prefs)
        val current = keystore.keysAliasFingerprintOrNull(alias) ?: return false
        return fingerprint == current
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
     * Decrypt the private key for [pubkeyHex] — the LAYERED retrieve ladder
     * (dashpay/platform#4060). Under [KeySecurityPolicy.AUTH_GATED] this
     * throws `UserNotAuthenticatedException` when the auth window expired —
     * the caller (KeystoreSigner) routes through [BiometricGate] and
     * retries; under [KeySecurityPolicy.DEVICE_BOUND] it never auth-gates.
     * Callers must zero the returned array after use.
     *
     * Ladder, in order:
     *  1. **Legacy AES-GCM** blob (non-empty IV) — shape-dispatch FIRST: the
     *     write-time fingerprint gate is NEVER applied to legacy blobs (they
     *     predate it, or carry a superseded key's fingerprint); the retained
     *     [KeystoreManager.KEYS_ALIAS] AES key recovers the value
     *     ([KeystoreManager.decryptLegacyKeysBlob]) and the entry is migrated
     *     to the policy alias. An absent legacy key → `null` (unrecoverable,
     *     the key-health/repair path takes over).
     *  2. **Empty-IV RSA, fingerprint fast path** — when the stored
     *     fingerprint matches the RECORDED alias's current key (a
     *     non-generating read; a blob written during a lockless
     *     AUTH_GATED→DEVICE_BOUND degradation carries its producing alias in
     *     the `privkeyalias.` tag), decrypt under that alias. A
     *     `KeyPermanentlyInvalidatedException` there has already run the
     *     generation-checked alias cleanup inside [KeystoreManager.decrypt]
     *     and RETHROWS — after the deletion the stored fingerprint can no
     *     longer match, so subsequent reads and the repair path re-DERIVE
     *     instead of trusting a stale certificate (the brick-loop fix). Any
     *     other unexpected crypto failure falls to the recovery ladder
     *     (defense in depth).
     *  3. **Recovery ladder** — a mismatching or missing fingerprint (or an
     *     absent/unprovisioned recorded alias) is a ROUTING signal, not an
     *     immediate `null`: pre-alias-split blobs carry the FORMER
     *     [KeystoreManager.KEYS_ALIAS] RSA key's fingerprint and must reach
     *     [KeystoreManager.decryptLegacyRsaKeysBlob]. The policy alias is
     *     never touched here (no OAEP attempt with an unrelated key, no
     *     keypair generation on a read). A wrong-key crypto failure is "not
     *     this key"; nothing opening the blob → `null` (re-derive signal).
     *
     * Recovered legacy values are re-encrypted under the current policy
     * alias and rewritten (see [migrateToPolicyAlias]), so subsequent reads
     * take the fast path. `UserNotAuthenticatedException` propagates from
     * every rung — a closed auth window is never a wrong-key signal.
     */
    suspend fun retrievePrivateKey(pubkeyHex: String): ByteArray? {
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return null
        val blob = decode(encoded)
        if (keystore.isLegacyKeysBlob(blob)) {
            // Rung 1 — legacy AES-GCM blob: recover with the retained legacy
            // AES key (may throw UserNotAuthenticatedException — the legacy
            // key was auth-gated — which the signer handles exactly as the
            // RSA path), or null if that key is already gone (unrecoverable).
            val plain = keystore.decryptLegacyKeysBlob(blob) ?: return null
            migrateToPolicyAlias(pubkeyHex, plain, encoded)
            return plain
        }
        if (!keystore.isKeysBlobDecryptable(blob)) return null
        val recordedAlias = recordedKeysAliasFor(pubkeyHex, prefs)
        val storedFingerprint = prefs[privateKeyFingerprintKey(pubkeyHex)]
        val currentFingerprint = keystore.keysAliasFingerprintOrNull(recordedAlias)
        if (storedFingerprint != null && currentFingerprint != null &&
            storedFingerprint == currentFingerprint &&
            keystore.hasIdentityKeysKey(recordedAlias)
        ) {
            // Rung 2 — the blob is CURRENT under the recorded alias.
            //
            // TOCTOU window (pre-existing #4172 parity, accepted): the alias
            // can rotate between the fingerprint read above and this
            // decrypt. The stale decrypt then fails as a wrong-key crypto
            // error and falls to the recovery ladder below (→ null when no
            // former key opens it — a re-derive signal) — never stale
            // plaintext, and never an uncaught crypto exception.
            return try {
                keystore.decrypt(blob, alias = recordedAlias)
            } catch (e: UserNotAuthenticatedException) {
                throw e // closed auth window — prompt and retry, never recovery
            } catch (e: KeyPermanentlyInvalidatedException) {
                // KeystoreManager.decrypt already ran the generation-checked
                // alias deletion; the typed signal must escape so the signer
                // suppresses the biometric retry and the next write/repair
                // regenerates the alias.
                throw e
            } catch (e: GeneralSecurityException) {
                // Rotation race / provider quirk: fall through to the
                // recovery ladder rather than failing the read outright.
                recoverEmptyIvRsaBlob(pubkeyHex, blob, encoded)
            }
        }
        // Rung 3 — fingerprint mismatch/missing, or the recorded alias is
        // absent (e.g. just deleted by invalidation cleanup): do NOT touch
        // the policy alias; try the retained former KEYS_ALIAS RSA keypair.
        return recoverEmptyIvRsaBlob(pubkeyHex, blob, encoded)
    }

    /**
     * Rung-3 recovery of an empty-IV RSA blob: the retained former
     * pre-alias-split RSA keypair at [KeystoreManager.KEYS_ALIAS] either
     * opens it (→ migrate forward + return) or the blob is unrecoverable
     * here (→ null, the key-health/repair path takes over).
     */
    private suspend fun recoverEmptyIvRsaBlob(
        pubkeyHex: String,
        blob: KeystoreManager.EncryptedBlob,
        sourceEncoded: String,
    ): ByteArray? {
        val recovered = tryFormerRsaRecovery(blob) ?: return null
        migrateToPolicyAlias(pubkeyHex, recovered, sourceEncoded)
        return recovered
    }

    /**
     * Attempt recovery of an empty-IV RSA blob with the retained former
     * pre-alias-split RSA keypair at [KeystoreManager.KEYS_ALIAS], converting a
     * wrong-key crypto failure to `null` ("not this key",
     * dashpay/platform#4060). [KeystoreManager.decryptLegacyRsaKeysBlob] returns
     * `null` when that key is absent and throws a JCE `BadPaddingException` when
     * the key is present but did not write the blob — presence alone is not proof
     * of origin, so that throw must be absorbed here rather than escaping
     * uncaught. `UserNotAuthenticatedException` is a closed-auth-window signal,
     * never a wrong key, so it propagates unchanged —
     * and `KeyPermanentlyInvalidatedException` must propagate too: it means the
     * retained former-RSA key EXISTS but was invalidated (lock-screen /
     * biometric change), which is not "not this key" — swallowing it to `null`
     * would hide the invalidation from the signer's classifier and the durable
     * repair seeding (the finding-3 hook) for pre-alias-split blobs.
     */
    private fun tryFormerRsaRecovery(blob: KeystoreManager.EncryptedBlob): ByteArray? =
        try {
            keystore.decryptLegacyRsaKeysBlob(blob)
        } catch (e: UserNotAuthenticatedException) {
            throw e
        } catch (e: KeyPermanentlyInvalidatedException) {
            throw e
        } catch (e: GeneralSecurityException) {
            null
        }

    /**
     * Best-effort re-encrypt [plain] under the current effective policy
     * alias (a never-auth-gated public-key encrypt) and rewrite the stored
     * blob + fingerprint + alias tag, migrating a recovered legacy value
     * forward so subsequent reads take the fingerprint fast path. A rewrite
     * failure must not lose the value the caller just recovered, so this
     * stays best-effort (migration retries on the next read).
     *
     * The rewrite is CONDITIONAL on the entry still holding [sourceEncoded] —
     * the exact encoded blob the caller read and recovered. [retrievePrivateKey]
     * runs without [privateKeyMutex], so between its read and this rewrite a
     * wallet deletion can win [withPrivateKeyExclusion], sweep the alias plus
     * its owner-index entry, and cascade the Room rows; an unconditional edit
     * would then RESURRECT `privkey.<pubkeyHex>` as undiscoverable ciphertext
     * with no owner-index or database reference, violating removeWallet's
     * no-surviving-ciphertext guarantee (dashpay/platform#4060, finding
     * 1049be675782). DataStore serializes edits, so the still-present check and
     * the write commit atomically against the deletion's edit: if the deletion
     * (or any concurrent overwrite — e.g. a [storePrivateKey] racing in a newer
     * value) got there first, the migration is skipped; the caller still
     * returns the plaintext it legitimately recovered.
     */
    private suspend fun migrateToPolicyAlias(
        pubkeyHex: String,
        plain: ByteArray,
        sourceEncoded: String,
    ) {
        try {
            val migrated = keystore.encryptForIdentityKeys(plain)
            store.edit {
                val key = privateKeyKey(pubkeyHex)
                if (it[key] == sourceEncoded) {
                    it[key] = encode(migrated.blob)
                    // Keep the write-time fingerprint + alias tag coherent
                    // with the re-encrypted blob, so [isCurrentKeysBlob] (the
                    // storeIfAbsent usability check) and the read fast path
                    // recognize the migrated entry instead of re-deriving it.
                    it[privateKeyFingerprintKey(pubkeyHex)] = migrated.keyFingerprint
                    it[privateKeyAliasKey(pubkeyHex)] = migrated.alias
                }
            }
        } catch (cancellation: CancellationException) {
            // NEVER swallow structured-concurrency cancellation: if the caller's
            // coroutine was cancelled during the encrypt / store.edit suspend
            // points, rethrow so the cancellation propagates. Only genuine
            // rewrite failures below stay best-effort (retry on the next read).
            throw cancellation
        } catch (_: Throwable) {
            // Best-effort: a rewrite failure must not lose the value the caller
            // just recovered — migration retries on the next read.
        }
    }

    suspend fun deletePrivateKey(pubkeyHex: String) {
        privateKeyMutex.withLock {
            store.edit {
                it.remove(privateKeyKey(pubkeyHex))
                it.remove(privateKeyFingerprintKey(pubkeyHex))
                it.remove(privateKeyAliasKey(pubkeyHex))
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
                prefs.remove(privateKeyAliasKey(pubkeyHex))
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
     * CHEAP signing-capability check: whether the blob stored for
     * [pubkeyHex] is plausibly recoverable by [retrievePrivateKey]. It never
     * decrypts, prompts, or generates a keypair (an absent alias just
     * returns `false`) — [KeystoreSigner.canSignWith] runs this under
     * `runBlocking` on a Rust callback thread, which must not block on
     * Keystore crypto or mutate Keystore state.
     *
     * Structure mirrors the retrieve ladder with PRESENCE checks in place of
     * decrypts: a legacy AES blob is signable iff the retained legacy AES
     * key still exists; a current-fingerprint RSA blob (under its recorded
     * alias) is signable; any other RSA-shaped blob is signable iff the
     * former [KeystoreManager.KEYS_ALIAS] RSA key survives (the recovery
     * ladder can then still open it — optimistic, disproved only by the
     * real-decrypt [probeIdentityKeyRecoverability], which is deliberately
     * NOT reachable from here).
     */
    suspend fun isPrivateKeyDecryptable(pubkeyHex: String): Boolean {
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return false
        val blob = decode(encoded)
        return when {
            keystore.isLegacyKeysBlob(blob) -> keystore.hasLegacyKeysKey()
            !keystore.isKeysBlobDecryptable(blob) -> false
            isCurrentKeysBlob(pubkeyHex, encoded, prefs) -> true
            // Former-RSA blobs are recoverable via the retrieve ladder as
            // long as the retained legacy RSA key survives — presence only.
            else -> keystore.hasLegacyRsaKeysKey()
        }
    }

    /**
     * PROBING key-health check: whether the blob stored for [pubkeyHex] can
     * ACTUALLY be recovered. This probes the same candidate keys
     * [retrievePrivateKey] would use and returns true only when a present key
     * really opens the blob — NOT a bare key-presence check, which reported a
     * stranded/sibling-alias blob "healthy" merely because an unrelated key of
     * the right shape existed (dashpay/platform#4060, finding e17e265dc680),
     * so `WalletKeyHealthSheet` never offered the re-derive/repair path this
     * check exists to drive. Deliberately SEPARATE from the cheap
     * [isPrivateKeyDecryptable]: real decrypts are far too heavy for the
     * signer's synchronous `canSignWith` callback thread — callers are the
     * key-health UI and the repair verification, never a signing path.
     *
     * The probe never prompts: [KeystoreManager.decrypt] /
     * [KeystoreManager.decryptLegacyRsaKeysBlob] / [KeystoreManager.decryptLegacyKeysBlob]
     * are bare Cipher operations — the biometric prompt is driven only by
     * `KeystoreSigner`/`BiometricGate`, never here. An auth-gated key whose auth
     * window is closed therefore throws `UserNotAuthenticatedException` (rather
     * than showing UI), which counts as RECOVERABLE: the key is present and the
     * value would recover after the user authenticates, so a health check must
     * not report it strandable. Only a wrong-key crypto failure (BadPadding /
     * AEAD tag) or an absent key yields "not recoverable". Recovered plaintext
     * is scrubbed immediately — a health check must not leave key bytes on the
     * heap.
     *
     *  - **Legacy AES-GCM** blob (non-empty IV): probe [KeystoreManager.decryptLegacyKeysBlob].
     *  - **Empty-IV RSA** blob: first let a prompt-free DEVICE_BOUND sibling
     *    DISPROVE ownership (see below — skipped when the blob's recorded
     *    alias IS the DEVICE_BOUND alias, where the sibling legitimately owns
     *    it), then probe the recorded alias (only if provisioned — an
     *    unprovisioned alias can't have written it), then the retained former
     *    KEYS_ALIAS RSA keypair. A structurally non-RSA blob is not
     *    recoverable.
     *
     * **AUTH_GATED residual (dashpay/platform#4060, finding b80a15c93339).** A
     * locked auth-gated alias throws `UserNotAuthenticatedException` at
     * `cipher.init` — before the ciphertext is examined — so a bare catch cannot
     * tell a locked *legitimate owner* from a locked *wrong* alias, and would
     * mis-report a sibling-written blob as recoverable. The prompt-free
     * DEVICE_BOUND sibling ([KeystoreManager.opensUnderNonGatedDeviceBoundSibling])
     * resolves the common case: if that non-gated sibling opens an
     * un-tagged blob, the current (auth-gated) policy alias does NOT own it,
     * and since [retrievePrivateKey] never falls back to an un-tagged sibling
     * the blob is genuinely strandable → `false` (drives the re-derive/repair
     * path). The irreducible residual is the symmetric one — a locked
     * auth-gated FORMER RSA key at KEYS_ALIAS whose ownership can't be
     * disproved prompt-free: it is still reported recoverable until the first
     * real unlock surfaces the BadPadding, at which point
     * [retrievePrivateKey]'s fallback→null drives the same repair.
     */
    suspend fun probeIdentityKeyRecoverability(pubkeyHex: String): Boolean {
        val prefs = store.data.first()
        val encoded = prefs[privateKeyKey(pubkeyHex)] ?: return false
        val blob = decode(encoded)
        if (keystore.isLegacyKeysBlob(blob)) {
            return probeOpensBlob { keystore.decryptLegacyKeysBlob(blob) }
        }
        if (!keystore.isKeysBlobDecryptable(blob)) return false
        val recordedAlias = recordedKeysAliasFor(pubkeyHex, prefs)
        // A prompt-free sibling proves an un-tagged blob belongs to the
        // non-gated DEVICE_BOUND alias, not the (possibly locked) auth-gated
        // alias the retrieve ladder would target — and retrieve never tries
        // the sibling for such a blob — so it is unrecoverable here (finding
        // b80a15c93339). When the blob is TAGGED as DEVICE_BOUND the sibling
        // is its recorded owner and the normal probe below covers it.
        if (recordedAlias != KeystoreManager.KEYS_ALIAS_DEVICE_BOUND &&
            keystore.opensUnderNonGatedDeviceBoundSibling(blob)
        ) {
            return false
        }
        // A closed auth window (UserNotAuthenticatedException, thrown at
        // cipher.init BEFORE the ciphertext is examined) proves nothing
        // about ownership. Count it as recoverable ONLY while the stored
        // write-time fingerprint still matches the recorded alias's CURRENT
        // key — a prompt-free disproof of the replaced-alias case: after a
        // Keystore loss + regeneration, the fresh auth-gated key's window is
        // closed almost always (it is only ever open ~AUTH_VALIDITY_SECONDS
        // after an auth), so without the fingerprint gate the probe would
        // report a blob that key can never open as "healthy" and the
        // key-health sheet would offer no repair while pendingIdentityKeys
        // simultaneously lists the same key (#4060 round-2 finding).
        val storedFingerprint = prefs[privateKeyFingerprintKey(pubkeyHex)]
        val unaeProvesRecoverable = storedFingerprint != null &&
            storedFingerprint == keystore.keysAliasFingerprintOrNull(recordedAlias)
        return (
            keystore.hasIdentityKeysKey(recordedAlias) &&
                probeOpensBlob(unaeProvesRecoverable) { keystore.decrypt(blob, recordedAlias) }
            ) ||
            (
                keystore.hasLegacyRsaKeysKey() &&
                    probeOpensBlob { keystore.decryptLegacyRsaKeysBlob(blob) }
                )
    }

    /**
     * True iff [decrypt] recovers the blob with a PRESENT key (plaintext
     * scrubbed immediately), or the key is auth-gated with a closed window
     * (`UserNotAuthenticatedException`) AND [unaeProvesRecoverable] — the
     * caller's prompt-free evidence that the gated key actually owns the
     * blob (the stored write-time fingerprint matches the alias's current
     * key). UNAE is thrown at `cipher.init`, before the ciphertext is
     * examined, so without that evidence a locked REPLACEMENT key would be
     * indistinguishable from a locked legitimate owner. A wrong-key crypto
     * failure or an absent key (`null`) is false. Prompt-free by
     * construction — see [probeIdentityKeyRecoverability]. Used only by the
     * non-prompting key-health probe, never on a signing path.
     */
    private fun probeOpensBlob(
        unaeProvesRecoverable: Boolean = true,
        decrypt: () -> ByteArray?,
    ): Boolean =
        try {
            val plain = decrypt()
            if (plain != null) {
                plain.fill(0)
                true
            } else {
                false
            }
        } catch (e: UserNotAuthenticatedException) {
            unaeProvesRecoverable
        } catch (e: GeneralSecurityException) {
            false
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

    private fun privateKeyAliasKey(pubkeyHex: String) =
        stringPreferencesKey(PRIVKEY_ALIAS_PREFIX + pubkeyHex.lowercase())

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

        /**
         * Per-blob record of the RSA identity-keys alias that produced the
         * ciphertext (`privkeyalias.<pubkeyHex>`), written atomically with the
         * blob. Routes reads to the exact producing alias after a lockless
         * AUTH_GATED→DEVICE_BOUND write degradation (dashpay/platform#4060);
         * a missing tag means "the current policy alias" (blobs written
         * before the tag existed).
         */
        const val PRIVKEY_ALIAS_PREFIX = "privkeyalias."

        /** Durable wallet → alias-hex-set owner index (string-set entries). */
        const val PRIVKEY_OWNERS_PREFIX = "privkeyowners."

        fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

        /**
         * A prompt-free probe of whether the device currently has a secure lock
         * screen (`KeyguardManager.isDeviceSecure`), captured against the
         * application context so it re-reads live state at each key generation
         * (a lock can be added/removed at any time). Handed to [KeystoreManager]
         * so it can degrade the lock-screen-bound key-gen parameters when no
         * lock is configured — the wallet must work without a screen lock
         * (dashpay/platform#4060).
         */
        fun deviceSecureProbe(context: Context): () -> Boolean {
            val appContext = context.applicationContext
            return {
                (appContext.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager)
                    ?.isDeviceSecure == true
            }
        }

        /**
         * A prompt-free sampler of `KeyguardManager`'s CURRENT lock state
         * (`isDeviceLocked` / `isKeyguardLocked`), captured against the
         * application context like [deviceSecureProbe] so each call reads
         * live state. Handed to [KeystoreManager] so a device-locked
         * Keystore denial can record — at throw time — whether the OS
         * agreed the device was locked, separating a genuine lock from the
         * false-locked Keystore2 defect (see
         * [KeystoreDeviceLockedException]). A missing KeyguardManager
         * samples as unlocked.
         */
        fun deviceLockStateProbe(context: Context): () -> DeviceLockState {
            val appContext = context.applicationContext
            return {
                val keyguard =
                    appContext.getSystemService(Context.KEYGUARD_SERVICE) as? KeyguardManager
                DeviceLockState(
                    isDeviceLocked = keyguard?.isDeviceLocked == true,
                    isKeyguardLocked = keyguard?.isKeyguardLocked == true,
                )
            }
        }

        /**
         * Backoff schedule for [storeMnemonic]'s FALSE-LOCKED retry (the
         * Keystore denied the master-alias encrypt as device-locked while
         * `KeyguardManager` reported the device unlocked): 3 retries,
         * ~2s total. Genuinely-locked denials never retry.
         */
        internal val DEVICE_FALSE_LOCKED_RETRY_DELAYS_MS = longArrayOf(250, 750, 1000)

        private const val TAG = "WalletStorage"
    }
}
