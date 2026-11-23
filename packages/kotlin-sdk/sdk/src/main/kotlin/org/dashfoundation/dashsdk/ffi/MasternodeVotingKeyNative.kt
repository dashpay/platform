package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for a masternode voting private key the host stores: mirrors
 * `rs-unified-sdk-jni/src/masternode_voting_key.rs`, one export per `rs-sdk-ffi` function.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore]
 * and, for the check against Platform and its rules,
 * [org.dashfoundation.dashsdk.voting.MasternodeVotingKeyChecks].
 * Rust zeroes its copies of the key and the text; the arrays passed in and returned are the
 * caller's to zero. Errors throw [DashSDKException] carrying Rust's reason, which never
 * contains the key or the text.
 */
internal object MasternodeVotingKeyNative {

    /**
     * The 20-byte voting address of the 32-byte [privateKey]: the hash160 of its compressed
     * public key. Bridges `dash_sdk_masternode_voting_address_from_private_key` (Swift
     * `KeychainSigner.masternodeVotingAddress`).
     */
    external fun votingAddressFromPrivateKey(privateKey: ByteArray): ByteArray

    /**
     * The 32 secret bytes of a private key typed as 64 hex characters or in compressed WIF.
     * Bridges `dash_sdk_private_key_bytes_from_text` (Swift
     * `KeyValidation.privateKeyBytes(fromText:)`).
     *
     * @param textUtf8 the text as UTF-8 bytes, so the caller can zero it after the call.
     */
    external fun privateKeyBytesFromText(textUtf8: ByteArray): ByteArray

    /**
     * Whether the voting key with the 20-byte [votingAddress] can vote for the masternode
     * [wireOrderProTxHash] (32 bytes, wire order), by one proof-verified fetch of its voter
     * identity. Bridges `dash_sdk_masternode_check_voting_key` (Swift
     * `SDK.checkMasternodeVotingKey(proTxHash:votingAddress:)`). Blocks for one Platform round
     * trip.
     *
     * @return `{"verdict":0|1|2,"message":string|null}`: Valid, Invalid, Undetermined, and the
     *   sentence Rust wrote for the last two.
     */
    external fun checkVotingKey(
        sdkHandle: Long,
        wireOrderProTxHash: ByteArray,
        votingAddress: ByteArray,
    ): String?

    /**
     * What a host does with a checked voting key, given what its slot holds. Bridges
     * `dash_sdk_masternode_voting_key_check_outcome` (Swift
     * `MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`).
     *
     * @param verdict 0 Valid, 1 Invalid, 2 Undetermined.
     * @param slot 0 Empty, 1 SameKey, 2 DifferentKey.
     * @param recordedCheck 0 none, 1 Valid, 2 Invalid, 3 Undetermined.
     * @param recordedVotingAddress the 20-byte address the record was made of; null for none.
     * @return `{"storeKey":bool,"record":null|{"check":1|2|3,"votingAddress":hex}}`.
     */
    external fun votingKeyCheckOutcome(
        verdict: Int,
        votingAddress: ByteArray,
        slot: Int,
        recordedCheck: Int,
        recordedVotingAddress: ByteArray?,
    ): String?

    /**
     * How a host may use a stored voting key, from its record. Bridges
     * `dash_sdk_masternode_recorded_voting_key_use` (Swift
     * `MasternodeRecordedVotingKeyUse.init(recorded:storedVotingAddress:)`).
     *
     * @param storedVotingAddress the stored key's 20-byte address; null when not read.
     * @return `{"canVote":bool,"checkBeforeUse":bool}`.
     */
    external fun recordedVotingKeyUse(
        recordedCheck: Int,
        recordedVotingAddress: ByteArray?,
        storedVotingAddress: ByteArray?,
    ): String?
}
