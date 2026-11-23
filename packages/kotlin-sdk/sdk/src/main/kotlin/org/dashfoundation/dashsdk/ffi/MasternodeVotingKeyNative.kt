package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for a masternode voting private key the host stores: mirrors
 * `rs-unified-sdk-jni/src/masternode_voting_key.rs`, one export per `rs-sdk-ffi` function.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore].
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
}
