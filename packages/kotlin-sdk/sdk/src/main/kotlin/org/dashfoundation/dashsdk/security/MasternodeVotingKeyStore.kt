package org.dashfoundation.dashsdk.security

import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.persistence.hexToByteArray
import org.dashfoundation.dashsdk.persistence.toHex

/**
 * Stores masternode voting private keys so a vote can be signed by the wallet manager's
 * [KeystoreSigner] instead of a key pasted for each vote. The Android counterpart of the
 * voting key slice of Swift's `KeychainSigner.swift` (`masternodeVotingAddress`) and of
 * `KeyValidation.privateKeyBytes(fromText:)` (`Utils/KeyValidation.swift`).
 *
 * A voting key is stored in [WalletStorage] as an ordinary identity-key entry under the
 * lowercase hex of its 20-byte voting address, with no owner wallet. That hex is the
 * identifier [store] returns and `IdentityEntity.votingPrivateKeyIdentifier` records. It is
 * exactly the entry the signer reads: for a vote Rust builds the ECDSA_HASH160 voting key
 * from the voting address and asks the signer to sign with it, so [KeystoreSigner] receives
 * the 20 address bytes and looks the key up by their hex, as for any identity key.
 *
 * Persistence and bridge only: Rust forms the voting address
 * (`dash_sdk_masternode_voting_address_from_private_key`) and reads typed key text
 * (`dash_sdk_private_key_bytes_from_text`); nothing is decided here. Which identities use a
 * key, and when a replaced key may be deleted, is the caller's
 * (`IdentityDao.countByVotingPrivateKeyIdentifier`).
 *
 * Wallet deletion does not remove these entries: they belong to no wallet's owner index.
 * It does remove an entry whose hex equals a `public_keys` row of one of the deleted
 * wallet's own identities, as it does for every identity key.
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
        return mapNativeErrors { MasternodeVotingKeyNative.votingAddressFromPrivateKey(privateKey) }
    }

    /**
     * Store the 32-byte voting [privateKey] under its voting address and return the
     * identifier (the lowercase address hex). A key already stored there and readable is
     * left as it is; the same key always lands under the same address. [privateKey] stays
     * the caller's to zero.
     */
    suspend fun store(privateKey: ByteArray): String {
        val identifier = votingAddress(privateKey).toHex()
        if (!storage.isPrivateKeyDecryptable(identifier)) {
            storage.storePrivateKey(identifier, privateKey, ownerWalletId = null)
        }
        return identifier
    }

    /** Whether a readable voting key is stored under [identifier]. */
    suspend fun isStored(identifier: String): Boolean = storage.isPrivateKeyDecryptable(identifier)

    /** Delete the voting key stored under [identifier]; nothing happens when there is none. */
    suspend fun delete(identifier: String) {
        storage.deletePrivateKey(identifier)
    }

    /**
     * The 20-byte voting address an [identifier] from [store] names, the `votingAddress` the
     * signer forms of the votes take.
     *
     * @throws IllegalArgumentException when [identifier] is not 40 hex characters.
     */
    fun votingAddressOf(identifier: String): ByteArray {
        require(identifier.length == VOTING_ADDRESS_HEX_LENGTH) {
            "a voting key identifier is $VOTING_ADDRESS_HEX_LENGTH hex characters, " +
                "got ${identifier.length}"
        }
        return identifier.hexToByteArray()
    }

    private companion object {
        /** A 20-byte voting address as hex. */
        const val VOTING_ADDRESS_HEX_LENGTH = 40
    }
}
