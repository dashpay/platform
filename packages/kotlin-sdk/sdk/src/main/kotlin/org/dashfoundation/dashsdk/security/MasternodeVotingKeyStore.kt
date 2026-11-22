package org.dashfoundation.dashsdk.security

import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
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
 * identifier [store] returns and `IdentityEntity.votingPrivateKeyIdentifier` records. It never
 * equals an identity key's entry (the public key or HASH160 hex), so deleting a wallet cannot
 * remove a voting key and replacing a voting key cannot remove a wallet's signing key. Two
 * identities that share one voting key each hold their own entry, so an entry belongs to one
 * identity row and needs no reference counting.
 *
 * Outside a vote nothing resolves the voting address to the entry: the signer reaches it only
 * inside `PlatformWalletManager.withStoredMasternodeVotingKey`, which maps the address to
 * the entry for the duration of one call.
 *
 * Persistence and bridge only: Rust forms the voting address
 * (`dash_sdk_masternode_voting_address_from_private_key`) and reads typed key text
 * (`dash_sdk_private_key_bytes_from_text`); nothing is decided here. Each native call loads
 * the native library first, so the store works in a fresh process.
 */
class MasternodeVotingKeyStore(private val storage: WalletStorage) {

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
     * @param identityId the masternode or evonode identity's 32-byte id.
     */
    suspend fun store(identityId: ByteArray, privateKey: ByteArray): String {
        val identifier = identifierFor(identityId, votingAddress(privateKey))
        storage.storePrivateKey(identifier, privateKey, ownerWalletId = null)
        return identifier
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
