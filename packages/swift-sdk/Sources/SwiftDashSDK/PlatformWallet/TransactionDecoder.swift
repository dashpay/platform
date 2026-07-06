import Foundation
import DashSDKFFI

/// A consensus-decoded Dash transaction: per-output `(address, value, script)`
/// plus per-input previous outpoints with a best-effort P2PKH sender address.
///
/// Wallet persistence stores every transaction's raw bytes but only
/// materializes TXO rows for the wallet's own outputs, so matching arbitrary
/// transactions against external addresses (e.g. the CrowdNode on-chain API:
/// "did a tx pay amount X to address Y?") requires decoding the stored bytes.
/// `TransactionDecoder.decode` is that opener.
public struct DecodedTransaction: Sendable, Equatable {
    public struct Input: Sendable, Equatable {
        /// Previous output's txid in consensus (internal) byte order —
        /// reverse for explorer-style display (see `prevTxidDisplayHex`).
        public let prevTxid: Data
        /// Previous output's index.
        public let prevVout: UInt32
        /// Sender address recovered from a P2PKH-shaped scriptSig; nil for
        /// coinbase / non-P2PKH / unparseable script sigs. Unauthenticated —
        /// derived from a script push the spender fully controls, with no
        /// signature verification against the spent UTXO. Use it for display
        /// or as a matching hint only, never for authentication or
        /// authorization; the attacker-resistant matching primitive is the
        /// per-output `(address, valueDuffs)` pair.
        public let address: String?

        /// Explorer-style (reversed, hex) rendering of `prevTxid`.
        public var prevTxidDisplayHex: String {
            prevTxid.reversed().map { String(format: "%02x", $0) }.joined()
        }
    }

    public struct Output: Sendable, Equatable {
        /// Destination address for standard scripts (P2PKH / P2SH); nil for
        /// non-standard scripts (OP_RETURN, bare multisig, …).
        public let address: String?
        /// Output value in duffs.
        public let valueDuffs: UInt64
        /// Raw scriptPubKey bytes.
        public let scriptPubkey: Data
    }

    /// Transaction id in consensus (internal) byte order — reverse for
    /// explorer-style display (see `txidDisplayHex`).
    public let txid: Data
    public let inputs: [Input]
    public let outputs: [Output]

    /// Explorer-style (reversed, hex) rendering of `txid`.
    public var txidDisplayHex: String {
        txid.reversed().map { String(format: "%02x", $0) }.joined()
    }
}

/// Pure utility — decodes raw transaction bytes via
/// `platform_wallet_decode_transaction`. No wallet handle or state involved.
public enum TransactionDecoder {
    /// Consensus-decode raw transaction bytes.
    /// - Parameters:
    ///   - txData: Serialized transaction bytes (e.g. from persisted rows).
    ///   - network: Network used to render addresses (base58 version bytes).
    /// - Returns: The decoded transaction.
    /// - Throws: `PlatformWalletError.deserialization` for malformed bytes
    ///   (including trailing garbage), `.invalidParameter` for empty input.
    public static func decode(_ txData: Data, network: Network) throws -> DecodedTransaction {
        guard !txData.isEmpty else {
            throw PlatformWalletError.invalidParameter("txData is empty")
        }

        var outDecoded: UnsafeMutablePointer<DecodedTransactionFFI>? = nil
        let res = txData.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_decode_transaction(
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                UInt(txData.count),
                network.ffiValue,
                &outDecoded
            )
        }
        try res.check()
        guard let decoded = outDecoded else {
            throw PlatformWalletError.nullPointer("decode returned success but no result")
        }
        defer { platform_wallet_decoded_transaction_free(decoded) }

        var entry = decoded.pointee
        let txid = withUnsafeBytes(of: &entry.txid) { Data($0) }

        var inputs: [DecodedTransaction.Input] = []
        if let ptr = entry.inputs, entry.inputs_count > 0 {
            inputs = (0..<Int(entry.inputs_count)).map { i in
                var input = ptr[i]
                let prevTxid = withUnsafeBytes(of: &input.prev_txid) { Data($0) }
                let address = input.address.map { String(cString: $0) }
                return DecodedTransaction.Input(
                    prevTxid: prevTxid,
                    prevVout: input.prev_vout,
                    address: address
                )
            }
        }

        var outputs: [DecodedTransaction.Output] = []
        if let ptr = entry.outputs, entry.outputs_count > 0 {
            outputs = (0..<Int(entry.outputs_count)).map { i in
                let output = ptr[i]
                let address = output.address.map { String(cString: $0) }
                let script: Data
                if let sptr = output.script_pubkey, output.script_pubkey_len > 0 {
                    script = Data(bytes: sptr, count: Int(output.script_pubkey_len))
                } else {
                    script = Data()
                }
                return DecodedTransaction.Output(
                    address: address,
                    valueDuffs: output.value_duffs,
                    scriptPubkey: script
                )
            }
        }

        return DecodedTransaction(txid: txid, inputs: inputs, outputs: outputs)
    }
}
