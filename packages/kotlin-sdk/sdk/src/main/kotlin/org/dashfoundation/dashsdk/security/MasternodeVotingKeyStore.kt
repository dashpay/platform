package org.dashfoundation.dashsdk.security

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
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
    internal suspend fun store(identityId: ByteArray, privateKey: ByteArray): String {
        val address = votingAddressForTesting?.invoke(privateKey) ?: votingAddress(privateKey)
        val identifier = identifierFor(identityId, address)
        storage.storePrivateKey(identifier, privateKey, ownerWalletId = null)
        return identifier
    }

    /**
     * Make the 32-byte voting [privateKey] the voting key of [identityId]'s row, as one
     * [withVotingKeyLock] section: write the identity's entry from the supplied bytes, point
     * the row at it only while the row still names [expectedIdentifier], then delete the
     * identity's other voting entries.
     *
     * - The row named [expectedIdentifier]: [VotingKeyAssignment.Assigned] with the new
     *   identifier, and whether the other entries were deleted. A failed deletion is not an
     *   error (the next assignment or [deleteOtherEntriesOf] under the lock retries it).
     * - The row is gone or names another key: the entry just written is deleted unless the
     *   row names it, and [VotingKeyAssignment.Changed] carries what the row names now.
     *
     * Entering the key the row already names, with that identifier as [expectedIdentifier],
     * rewrites the entry, which repairs one that no longer reads. [privateKey] stays the
     * caller's to zero.
     *
     * @param expectedIdentifier the identifier the row is expected to name, null for none.
     * @throws Exception when the key cannot be written (nothing is recorded then);
     *   cancellation propagates.
     */
    suspend fun assignToIdentity(
        identityDao: IdentityDao,
        identityId: ByteArray,
        privateKey: ByteArray,
        expectedIdentifier: String?,
        nowMillis: Long = System.currentTimeMillis(),
    ): VotingKeyAssignment = withVotingKeyLock {
        val identifier = store(identityId, privateKey)
        val changed = identityDao.replaceVotingPrivateKeyIdentifier(
            identityId, expectedIdentifier, identifier, nowMillis,
        )
        if (changed == 0) {
            val current = identityDao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier
            if (current != identifier) storage.deletePrivateKey(identifier)
            VotingKeyAssignment.Changed(current)
        } else {
            val swept = try {
                deleteOtherEntriesOf(identityId, keep = identifier)
                true
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                false
            }
            VotingKeyAssignment.Assigned(identifier, otherEntriesRemoved = swept)
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
     * Delete every voting entry of [identityId] except [keep], in one atomic write: the
     * entries a replaced key, or an interrupted load or replacement, left behind. Run after
     * each successful save, so a sweep that failed is repeated by the next one.
     *
     * @param keep the identifier the identity's row names now, or null to delete them all.
     * @throws IllegalArgumentException when [keep] is not one of [identityId]'s identifiers.
     */
    suspend fun deleteOtherEntriesOf(identityId: ByteArray, keep: String?) {
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
        storage.deletePrivateKeys(names)
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
         * Test hook: called just before a caller waits for [votingKeyLock] while another
         * holds it, so a test can see the second operation reached the lock.
         */
        @Volatile
        internal var onLockContendedForTesting: (() -> Unit)? = null

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
         * [assignToIdentity] takes it itself. Not reentrant: [block] must not call it, or
         * [assignToIdentity], again. [delete], [deleteAll] and [deleteOtherEntriesOf] do not
         * take it; their callers compose them under it. Signing does not take it.
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
     * The row names [identifier] now. [otherEntriesRemoved] is false when the identity's
     * other voting entries could not be deleted yet; the next assignment retries.
     */
    data class Assigned(val identifier: String, val otherEntriesRemoved: Boolean) :
        VotingKeyAssignment

    /**
     * The row did not name the expected identifier, so nothing was recorded; it names
     * [currentIdentifier] (null for none, or when the row is gone).
     */
    data class Changed(val currentIdentifier: String?) : VotingKeyAssignment
}
