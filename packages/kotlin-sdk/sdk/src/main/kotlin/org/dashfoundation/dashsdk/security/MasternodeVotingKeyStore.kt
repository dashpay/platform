package org.dashfoundation.dashsdk.security

import androidx.annotation.VisibleForTesting
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.hexToByteArray
import org.dashfoundation.dashsdk.persistence.toHex

/**
 * Stores masternode voting private keys so a vote can be signed by the wallet manager's
 * [KeystoreSigner] instead of a key pasted for each vote. The Android counterpart of the
 * voting key slice of Swift's `KeychainSigner.swift` (`storeSpecialKey(_:identityId:keyType:)`
 * with `.voting`, `masternodeVotingAddress`) and of `KeyValidation.privateKeyBytes(fromText:)`
 * (`Utils/KeyValidation.swift`).
 *
 * Each loaded masternode or evonode identity keeps its voting key in its own [WalletStorage]
 * private-key entry named `mnvoting.<identityIdHex>.<votingAddressHex>` (lowercase hex, the
 * 32-byte identity id and the 20-byte voting address), with no owner wallet. That name is the
 * identifier [assignToIdentity] records in `IdentityEntity.votingPrivateKeyIdentifier`, writing
 * the entry and the row together. It never
 * equals an identity key's entry (the public key or HASH160 hex), so deleting a wallet cannot
 * remove a voting key and replacing a voting key cannot remove a wallet's signing key. Two
 * identities that share one voting key each hold their own entry, so an entry belongs to one
 * identity row and needs no reference counting.
 *
 * Outside a vote nothing resolves the voting address to the entry: the signer reaches it only
 * inside `PlatformWalletManager.withStoredMasternodeVotingKey`, whose per-vote signer serves
 * that one entry for the duration of one call.
 *
 * Persistence and bridge only: Rust forms the voting address
 * (`dash_sdk_masternode_voting_address_from_private_key`) and reads typed key text
 * (`dash_sdk_private_key_bytes_from_text`); nothing is decided here. Each native call loads
 * the native library first, so the store works in a fresh process.
 */
class MasternodeVotingKeyStore private constructor(
    private val storage: WalletStorage,
    /** Forms the voting address instead of Rust, for JVM tests; null in production. */
    private val votingAddressForTesting: ((ByteArray) -> ByteArray)?,
) {

    constructor(storage: WalletStorage) : this(storage, null)

    /**
     * The 32 secret bytes of a private key typed as 64 hex characters or in compressed WIF.
     * Rust decides which forms are accepted. The returned array is the caller's to zero.
     *
     * [text] crosses JNI as a UTF-8 copy that is zeroed before this returns; the [String]
     * itself cannot be zeroed.
     *
     * @throws org.dashfoundation.dashsdk.errors.DashSdkError with Rust's reason when the text
     *   is neither form, an uncompressed WIF, not a secp256k1 secret key, or contains a NUL
     *   character. The reason never contains the text.
     */
    fun privateKeyBytesFromText(text: String): ByteArray {
        NativeLoader.ensureLoaded()
        val utf8 = text.toByteArray(Charsets.UTF_8)
        try {
            return mapNativeErrors { MasternodeVotingKeyNative.privateKeyBytesFromText(utf8) }
        } finally {
            utf8.fill(0)
        }
    }

    /**
     * The 20-byte voting address of the 32-byte voting [privateKey]: the hash160 of its
     * compressed public key, formed by Rust. [privateKey] stays the caller's to zero.
     */
    fun votingAddress(privateKey: ByteArray): ByteArray {
        require(privateKey.size == 32) { "privateKey must be 32 bytes, got ${privateKey.size}" }
        NativeLoader.ensureLoaded()
        return mapNativeErrors { MasternodeVotingKeyNative.votingAddressFromPrivateKey(privateKey) }
    }

    /**
     * Write the 32-byte voting [privateKey] into [identityId]'s voting entry and return its
     * identifier. Always writes from the supplied bytes, replacing whatever the entry held,
     * so storing the key again repairs an entry that can no longer be read. [privateKey]
     * stays the caller's to zero.
     *
     * Internal: an entry written without its row could be cleared or swept away unseen, or
     * outlive a clear; callers use [assignToIdentity], which writes both under the lock.
     *
     * @param identityId the masternode or evonode identity's 32-byte id.
     */
    @JvmSynthetic
    internal suspend fun store(identityId: ByteArray, privateKey: ByteArray): String {
        val address = votingAddressForTesting?.invoke(privateKey) ?: votingAddress(privateKey)
        val identifier = identifierFor(identityId, address)
        storage.storePrivateKey(identifier, privateKey, ownerWalletId = null)
        return identifier
    }

    /**
     * Make the 32-byte voting [privateKey] the voting key of [identityId]'s row on
     * [networkRaw], as one [withVotingKeyLock] section: write the identity's entry from the
     * supplied bytes and point the row at it only while the row is on [networkRaw] and still
     * names [expectedIdentifier].
     *
     * - The row named [expectedIdentifier]: [VotingKeyAssignment.Assigned] with the new
     *   identifier.
     * - The row is gone, on another network (then the result carries null) or names another
     *   key: [VotingKeyAssignment.Changed] with what the row on [networkRaw] names now. The row is checked first, inside the lock, so then nothing is written; the
     *   conditional update after the write checks again, and if that finds a change the
     *   entry just written is deleted unless the row names it.
     *
     * The new entry is recorded as pending (see [pendingIdentifiers]): [removeStaleEntries]
     * keeps it until a row is seen naming it, so a cleanup that runs before an enclosing
     * transaction commits cannot delete it. The [expectedIdentifier] this call finds on the
     * row is no longer pending.
     *
     * Nothing else is deleted, the entry the row named before included: call
     * [removeStaleEntries] once the change has committed. So if the row update joins a Room
     * transaction the caller later rolls back, the row names its old key again and that key
     * is still stored; the new entry is merely stale until the next cleanup.
     *
     * Entering the key the row already names, with that identifier as [expectedIdentifier],
     * rewrites the entry, which repairs one that no longer reads. [privateKey] stays the
     * caller's to zero.
     *
     * @param database the database whose `identities` row records the key. Call it outside
     *   any Room transaction. It refuses when the calling thread is in one, but that check is
     *   only a best-effort safety net: Room carries a transaction in the coroutine context,
     *   which a thread check cannot see after a dispatcher switch. What keeps a rollback safe
     *   is that nothing but the entry this call wrote is ever deleted here.
     * @param networkRaw the `Network.ffiValue` the identity is loaded on.
     * @param expectedIdentifier the identifier the row is expected to name, null for none.
     * @throws IllegalStateException when the calling thread is in a Room transaction, before
     *   anything is written.
     * @throws Exception when the key cannot be written (nothing is recorded then);
     *   cancellation propagates.
     */
    suspend fun assignToIdentity(
        database: DashDatabase,
        identityId: ByteArray,
        networkRaw: Int,
        privateKey: ByteArray,
        expectedIdentifier: String?,
        nowMillis: Long = System.currentTimeMillis(),
    ): VotingKeyAssignment {
        refuseInTransaction(database, "assignToIdentity")
        return withVotingKeyLock {
            assignLocked(database, identityId, networkRaw, privateKey, expectedIdentifier, nowMillis)
        }
    }

    private suspend fun assignLocked(
        database: DashDatabase,
        identityId: ByteArray,
        networkRaw: Int,
        privateKey: ByteArray,
        expectedIdentifier: String?,
        nowMillis: Long,
    ): VotingKeyAssignment {
        val identityDao = database.identityDao()
        // Nothing is written for a row that is gone, on another network, or names another
        // key by now.
        val before = identityDao.getByIdentityId(identityId)?.takeIf { it.networkRaw == networkRaw }
        if (before == null || before.votingPrivateKeyIdentifier != expectedIdentifier) {
            return VotingKeyAssignment.Changed(before?.votingPrivateKeyIdentifier)
        }
        // The row was just seen naming [expectedIdentifier]: outside any transaction of the
        // caller that is a committed (or the caller's own) write, so it is no longer pending.
        // An assignment still uncommitted elsewhere is never seen here and stays pending.
        if (expectedIdentifier != null) pendingIdentifiers -= expectedIdentifier.lowercase()
        val identifier = store(identityId, privateKey)
        // Pending until a cleanup sees a row naming it: the row update below may belong to a
        // transaction that has not committed yet (see [pendingIdentifiers]).
        pendingIdentifiers += identifier
        val changed = identityDao.replaceVotingPrivateKeyIdentifier(
            identityId, networkRaw, expectedIdentifier, identifier, nowMillis,
        )
        if (changed == 0) {
            val row = identityDao.getByIdentityId(identityId)
            // Whether to delete the entry just written is decided by the row whatever its
            // network: entry names hold no network, so a row that moved network but still
            // names this identifier (a same-key re-entry it overwrote) still uses it.
            if (row?.votingPrivateKeyIdentifier != identifier) {
                storage.deletePrivateKey(identifier)
                pendingIdentifiers -= identifier
            }
            return VotingKeyAssignment.Changed(
                row?.takeIf { it.networkRaw == networkRaw }?.votingPrivateKeyIdentifier,
            )
        }
        return VotingKeyAssignment.Assigned(identifier)
    }

    /**
     * Delete every voting entry of [identityId] except the one its row names now, as one
     * [withVotingKeyLock] section: the entry a replaced key left, or one a rolled-back or
     * interrupted assignment left. When the row is gone, all of the identity's entries go.
     * Returns true only when the row's current entry is all that remains of the identity.
     * Returns false when the row could not be read or the entries could not be deleted
     * (nothing is deleted when the read fails), or when a pending entry other than the
     * current one was kept; a later call retries.
     *
     * Entries [assignToIdentity] wrote that no row has been seen to name yet are kept (see
     * [pendingIdentifiers]): an assignment whose row update sits in a caller's transaction
     * that has not committed is invisible to this read, and deleting its entry would leave
     * the committed row naming a key that is gone. Once the row names a pending entry, it is
     * no longer pending. A pending entry whose transaction rolled back stays on disk for
     * the life of the process; the next process's first cleanup removes it.
     *
     * Call it after the write that changed the row's identifier has committed, never inside
     * a Room transaction: run against an uncommitted row, it could delete the key a rollback
     * restores. It refuses when the calling thread is in a transaction; like the check in
     * [assignToIdentity], that is a best-effort, thread-bound safety net.
     *
     * @throws IllegalStateException when the calling thread is in a Room transaction.
     */
    suspend fun removeStaleEntries(database: DashDatabase, identityId: ByteArray): Boolean {
        refuseInTransaction(database, "removeStaleEntries")
        return withVotingKeyLock {
            try {
                val keep = database.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier
                // A row naming a pending entry shows its write is committed and visible.
                if (keep != null) pendingIdentifiers -= keep.lowercase()
                // False when a pending entry other than the current one had to stay.
                deleteOtherEntriesOf(identityId, keep, alsoKeep = pendingIdentifiers.toSet())
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                false
            }
        }
    }

    private fun refuseInTransaction(database: DashDatabase, operation: String) {
        check(!database.inTransaction()) {
            "$operation cannot run inside a Room transaction: its secure-storage writes " +
                "cannot roll back with it"
        }
    }

    /**
     * Whether a readable voting key is stored under [identifier].
     *
     * @throws IllegalArgumentException when [identifier] is not a voting key identifier, so
     *   a bare address or public key hex never reaches a wallet key's entry.
     */
    suspend fun isStored(identifier: String): Boolean {
        parse(identifier)
        return storage.isPrivateKeyDecryptable(identifier)
    }

    /**
     * Delete the voting key stored under [identifier]; nothing happens when there is none.
     *
     * @throws IllegalArgumentException when [identifier] is not a voting key identifier, so
     *   a bare address or public key hex never deletes a wallet key's entry.
     */
    suspend fun delete(identifier: String) {
        parse(identifier)
        storage.deletePrivateKey(identifier)
    }

    /**
     * Delete every voting entry of [identityId] except [keep] and [alsoKeep], in one atomic
     * write. Internal: callers use [removeStaleEntries], which reads [keep] from the row
     * under the lock.
     *
     * @param keep the identifier the identity's row names now, or null to delete them all.
     * @param alsoKeep identifiers that must survive too (pending assignments).
     * @return true when no entry but [keep] remains; false when an entry of [alsoKeep] did.
     * @throws IllegalArgumentException when [keep] is not one of [identityId]'s identifiers.
     */
    @JvmSynthetic
    internal suspend fun deleteOtherEntriesOf(
        identityId: ByteArray,
        keep: String?,
        alsoKeep: Set<String> = emptySet(),
    ): Boolean {
        require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
        val keepName = keep?.lowercase()?.also {
            require(parse(it).first.contentEquals(identityId)) {
                "keep must be a voting key identifier of this identity"
            }
        }
        val prefix = STORAGE_ENTRY_PREFIX + IDENTIFIER_PREFIX + identityId.toHex() + "."
        val names = storage.listEntryNames()
            .filter { it.startsWith(prefix) }
            .map { it.removePrefix(STORAGE_ENTRY_PREFIX) }
            .filter { it != keepName }
        storage.deletePrivateKeys(names.filter { it !in alsoKeep })
        return names.none { it in alsoKeep }
    }

    /**
     * Delete every stored voting key, in one atomic write: what clearing the identities of
     * this device removes along with their rows. Identity keys and mnemonics are not touched.
     */
    suspend fun deleteAll() {
        val names = storage.listEntryNames()
            .filter { it.startsWith(STORAGE_ENTRY_PREFIX + IDENTIFIER_PREFIX) }
            .map { it.removePrefix(STORAGE_ENTRY_PREFIX) }
        storage.deletePrivateKeys(names)
    }

    /**
     * The 20-byte voting address an [identifier] from [store] names, the `votingAddress` the
     * signer forms of the votes take.
     *
     * @throws IllegalArgumentException when [identifier] is not a voting key identifier.
     */
    fun votingAddressOf(identifier: String): ByteArray = parse(identifier).second

    /**
     * The 32-byte id of the identity an [identifier] from [store] belongs to.
     *
     * @throws IllegalArgumentException when [identifier] is not a voting key identifier.
     */
    fun identityIdOf(identifier: String): ByteArray = parse(identifier).first

    companion object {
        /** Serializes every voting key change that spans an entry and an identity row. */
        private val votingKeyLock = Mutex()

        /**
         * Identifiers [assignToIdentity] wrote and recorded that no row has yet been seen to
         * name (by [removeStaleEntries], or by a later assignment's check that the row names
         * the identifier it replaces), which that cleanup never deletes. Read and written
         * only while holding [votingKeyLock].
         *
         * Why: an assignment's row update can join a caller's Room transaction that commits
         * later, and an independent cleanup reading the committed row in between would delete
         * the new entry the commit then names. Neither guard can see such a transaction: the
         * thread-bound `RoomDatabase.inTransaction()` misses a coroutine transaction after a
         * dispatcher switch, and Room 2.7.1's `useWriterConnection { it.inTransaction() }`
         * reports false even directly inside the legacy `withTransaction` this app uses
         * (measured under Robolectric). In-process only: an entry left pending by a
         * transaction that rolled back stays on disk until the next process, whose first
         * cleanup removes it.
         */
        private val pendingIdentifiers = HashSet<String>()

        /** Test hook: forget every pending identifier, as a new process starts. */
        @VisibleForTesting
        @JvmSynthetic
        internal suspend fun resetPendingForTesting() {
            withVotingKeyLock { pendingIdentifiers.clear() }
        }

        /**
         * TEST ONLY; production code never sets it. Called just before a caller of
         * [withVotingKeyLock] waits for the lock while another holds it, so a test (in this
         * module or an app's) can see the second operation reached the lock. Reset it to null
         * after the test.
         */
        @Volatile
        @VisibleForTesting
        var onLockContendedForTesting: (() -> Unit)? = null

        /** A store whose voting addresses come from [votingAddress], for JVM tests. */
        @JvmSynthetic
        internal fun forTesting(
            storage: WalletStorage,
            votingAddress: (ByteArray) -> ByteArray,
        ): MasternodeVotingKeyStore = MasternodeVotingKeyStore(storage, votingAddress)

        /**
         * Run [block] as the only voting key change in this process. Hold it around every
         * sequence that writes an entry and the identity row naming it, or that reads one to
         * act on the other: storing a key then recording it on the row, sweeping an
         * identity's other entries against the identifier its row names, and clearing all
         * entries together with the rows. Without it, a reload's sweep could delete a key a
         * replacement just stored but has not yet recorded, and a clear could miss a key
         * an import stores between its listing and its row delete.
         *
         * [assignToIdentity] and [removeStaleEntries] take it themselves. Not reentrant:
         * [block] must not call it, [assignToIdentity] or [removeStaleEntries] again. [delete]
         * and [deleteAll] do not take it; their callers compose them under it. Signing does
         * not take it.
         */
        suspend fun <T> withVotingKeyLock(block: suspend () -> T): T {
            if (votingKeyLock.isLocked) onLockContendedForTesting?.invoke()
            return votingKeyLock.withLock { block() }
        }

        /** The start of every voting key identifier. */
        const val IDENTIFIER_PREFIX = "mnvoting."

        /** `WalletStorage`'s private-key entry prefix, as `listEntryNames` shows it. */
        private const val STORAGE_ENTRY_PREFIX = "privkey."

        private const val IDENTITY_ID_HEX_LENGTH = 64
        private const val VOTING_ADDRESS_HEX_LENGTH = 40

        /**
         * The identifier of [identityId]'s voting key whose voting address is
         * [votingAddress]: `mnvoting.<identityIdHex>.<votingAddressHex>`, lowercase.
         */
        fun identifierFor(identityId: ByteArray, votingAddress: ByteArray): String {
            require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
            require(votingAddress.size == 20) {
                "votingAddress must be 20 bytes, got ${votingAddress.size}"
            }
            return IDENTIFIER_PREFIX + identityId.toHex() + "." + votingAddress.toHex()
        }

        private fun parse(identifier: String): Pair<ByteArray, ByteArray> {
            val parts = identifier.lowercase().split('.')
            require(
                parts.size == 3 &&
                    parts[0] + "." == IDENTIFIER_PREFIX &&
                    parts[1].length == IDENTITY_ID_HEX_LENGTH && parts[1].isHex() &&
                    parts[2].length == VOTING_ADDRESS_HEX_LENGTH && parts[2].isHex(),
            ) {
                "not a voting key identifier: expected " +
                    "${IDENTIFIER_PREFIX}<64 hex characters>.<40 hex characters>"
            }
            return parts[1].hexToByteArray() to parts[2].hexToByteArray()
        }

        private fun String.isHex(): Boolean = all { it in '0'..'9' || it in 'a'..'f' }
    }
}

/**
 * No readable masternode voting key is stored for the identity a vote was asked to sign for
 * (`PlatformWalletManager.withStoredMasternodeVotingKey`). Swift
 * `KeychainSigner.Error.votingKeyNotStored`. Load the identity again with its voting key.
 */
class MasternodeVotingKeyNotStoredException :
    IllegalStateException(
        "No voting key is stored for this masternode identity. Load it again with its " +
            "voting private key.",
    )

/** What [MasternodeVotingKeyStore.assignToIdentity] did. */
sealed interface VotingKeyAssignment {
    /**
     * The row names [identifier] now. The entry it named before is still stored; remove it
     * with [MasternodeVotingKeyStore.removeStaleEntries] once the change has committed.
     */
    data class Assigned(val identifier: String) : VotingKeyAssignment

    /**
     * The row did not name the expected identifier, so nothing was recorded; it names
     * [currentIdentifier] (null for none, or when the row is gone).
     */
    data class Changed(val currentIdentifier: String?) : VotingKeyAssignment
}
