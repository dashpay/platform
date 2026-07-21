package org.dashfoundation.dashsdk.keywallet

import java.nio.ByteBuffer
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TxDecodeNative

/**
 * A consensus-decoded Dash transaction: per-output `(address, value,
 * script)` plus per-input previous outpoints with a best-effort P2PKH
 * sender address — port of `SwiftDashSDK/KeyWallet/TransactionDecoder.swift`
 * (platform PR #3981) over key-wallet-ffi's `transaction_decode`.
 *
 * Wallet persistence stores every transaction's raw bytes
 * (`TransactionEntity.transactionData`) but only materializes TXO rows for
 * the wallet's own outputs, so rendering or matching a transaction's FULL
 * input/output detail requires decoding the stored bytes.
 * [TransactionDecoder.decode] is that opener.
 */
class DecodedTransaction(
    /**
     * Transaction id in consensus (internal) byte order — reverse for
     * explorer-style display (see [txidDisplayHex]).
     */
    val txid: ByteArray,
    val inputs: List<Input>,
    val outputs: List<Output>,
) {
    class Input(
        /**
         * Previous output's txid in consensus (internal) byte order —
         * reverse for explorer-style display (see [prevTxidDisplayHex]).
         */
        val prevTxid: ByteArray,
        /** Previous output's index. */
        val prevVout: Int,
        /**
         * Sender address recovered from a P2PKH-shaped scriptSig; null for
         * coinbase / non-P2PKH / unparseable script sigs. Unauthenticated —
         * derived from a script push the spender fully controls, with no
         * signature verification against the spent UTXO. Use it for display
         * or as a matching hint only, never for authentication or
         * authorization; the attacker-resistant matching primitive is the
         * per-output `(address, valueDuffs)` pair.
         */
        val address: String?,
    ) {
        /** Explorer-style (reversed, hex) rendering of [prevTxid]. */
        val prevTxidDisplayHex: String get() = displayHex(prevTxid)
    }

    class Output(
        /**
         * Destination address for standard scripts (P2PKH / P2SH); null
         * for non-standard scripts (OP_RETURN, bare multisig, …).
         */
        val address: String?,
        /** Output value in duffs (unsigned semantics). */
        val valueDuffs: Long,
        /** Raw scriptPubKey bytes. */
        val scriptPubkey: ByteArray,
    )

    /** Explorer-style (reversed, hex) rendering of [txid]. */
    val txidDisplayHex: String get() = displayHex(txid)

    private companion object {
        fun displayHex(wire: ByteArray): String =
            wire.reversedArray().joinToString("") { "%02x".format(it) }
    }
}

/**
 * Pure utility — decodes raw transaction bytes via key-wallet-ffi's
 * `transaction_decode`. No wallet handle or state involved.
 */
object TransactionDecoder {

    /**
     * Consensus-decode raw transaction bytes.
     *
     * @param txData serialized transaction bytes (e.g. a persisted
     *   `TransactionEntity.transactionData`).
     * @param network network used to render addresses (base58 version bytes).
     * @throws org.dashfoundation.dashsdk.errors.DashSdkError.InvalidParameter
     *   for malformed or empty bytes (including trailing garbage).
     */
    fun decode(txData: ByteArray, network: Network): DecodedTransaction {
        Sdk.initialize()
        val blob = mapNativeErrors { TxDecodeNative.decodeTransaction(txData, network.ffiValue) }
        return parseBlob(blob)
    }

    /**
     * Parse the packed big-endian BLOB produced by
     * `rs-unified-sdk-jni/src/tx_decode.rs` (layout documented there; the
     * Rust test `fixture_blob_hex_is_pinned_for_kotlin` and
     * `TransactionDecoderTest` pin the same fixture bytes from both sides).
     * Internal + pure so it is host-JVM unit-testable without the native
     * library.
     */
    internal fun parseBlob(blob: ByteArray): DecodedTransaction {
        val buf = ByteBuffer.wrap(blob) // big-endian by default
        val txid = ByteArray(32).also { buf.get(it) }

        val inputCount = buf.int
        require(inputCount >= 0) { "malformed decode blob: negative input count" }
        val inputs = ArrayList<DecodedTransaction.Input>(inputCount)
        repeat(inputCount) {
            val prevTxid = ByteArray(32).also { buf.get(it) }
            val prevVout = buf.int
            inputs += DecodedTransaction.Input(prevTxid, prevVout, readStringOpt(buf))
        }

        val outputCount = buf.int
        require(outputCount >= 0) { "malformed decode blob: negative output count" }
        val outputs = ArrayList<DecodedTransaction.Output>(outputCount)
        repeat(outputCount) {
            val valueDuffs = buf.long
            val address = readStringOpt(buf)
            val scriptLen = buf.int
            require(scriptLen >= 0) { "malformed decode blob: negative script length" }
            val script = ByteArray(scriptLen).also { buf.get(it) }
            outputs += DecodedTransaction.Output(address, valueDuffs, script)
        }

        require(!buf.hasRemaining()) { "malformed decode blob: trailing bytes" }
        return DecodedTransaction(txid, inputs, outputs)
    }

    /** `u16 len + UTF-8 bytes`; len 0 = null (addresses are never empty). */
    private fun readStringOpt(buf: ByteBuffer): String? {
        val len = buf.short.toInt() and 0xFFFF
        if (len == 0) return null
        val bytes = ByteArray(len).also { buf.get(it) }
        return String(bytes, Charsets.UTF_8)
    }
}
