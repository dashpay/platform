package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for consensus transaction decoding — mirrors
 * `rs-unified-sdk-jni/src/tx_decode.rs`.
 */
internal object TxDecodeNative {

    /**
     * Consensus-decode raw transaction bytes (key-wallet-ffi
     * `transaction_decode`) into the packed big-endian BLOB documented in
     * `tx_decode.rs` / parsed by
     * [org.dashfoundation.dashsdk.keywallet.TransactionDecoder.parseBlob].
     * [networkOrd] is [org.dashfoundation.dashsdk.Network.ffiValue].
     *
     * @throws DashSDKException on malformed bytes (including trailing
     *   garbage after a valid transaction)
     */
    external fun decodeTransaction(txBytes: ByteArray, networkOrd: Int): ByteArray
}
