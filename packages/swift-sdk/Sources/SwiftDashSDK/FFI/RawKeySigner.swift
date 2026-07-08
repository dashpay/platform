import Foundation
import DashSDKFFI

/// Stateless ECDSA message signing with a caller-supplied raw private
/// key — the v1 sign primitive extracted from `KeychainSigner.ffiSign`
/// so callers that already hold a key scalar (e.g. one derived via
/// `Wallet.derivePrivateKey(path:)`) can sign without a keychain
/// round-trip.
///
/// The FFI signer SHA256d-hashes `data` internally and produces the
/// 65-byte compact recoverable signature Dash Core's `signmessage`
/// emits, so callers pass the raw (framed) message bytes, not a digest.
public enum RawKeySigner {
    public enum Error: Swift.Error, LocalizedError {
        case signerCreationFailed(message: String)
        case signFailed(message: String)

        public var errorDescription: String? {
            switch self {
            case .signerCreationFailed(let message):
                return "Failed to construct FFI signer: \(message)"
            case .signFailed(let message):
                return "FFI sign call failed: \(message)"
            }
        }
    }

    /// Sign `data` with the raw 32-byte ECDSA scalar `privateKey`.
    /// Wraps the key in a throwaway FFI signer just long enough to
    /// produce a signature; the local key copy is zeroed as soon as
    /// the FFI call returns.
    public static func sign(data: Data, privateKey: Data, network: Network) throws -> Data {
        // Defensive copy into a mutable buffer we can zero on exit.
        var keyCopy = [UInt8](privateKey)
        defer {
            keyCopy.withUnsafeMutableBufferPointer { buf in
                if let base = buf.baseAddress {
                    memset_s(UnsafeMutableRawPointer(base), buf.count, 0, buf.count)
                }
            }
        }

        let signerResult = keyCopy.withUnsafeBufferPointer { keyBuf -> DashSDKResult in
            dash_sdk_signer_create_from_private_key(
                keyBuf.baseAddress!,
                UInt(keyBuf.count),
                network.ffiValue
            )
        }

        if let errPtr = signerResult.error {
            let message = errPtr.pointee.message.map { String(cString: $0) } ?? "unknown"
            dash_sdk_error_free(errPtr)
            throw Error.signerCreationFailed(message: message)
        }
        guard let rawSigner = signerResult.data else {
            throw Error.signerCreationFailed(message: "null handle")
        }
        let signerHandle = rawSigner.assumingMemoryBound(to: SignerHandle.self)
        defer { dash_sdk_signer_destroy(signerHandle) }

        let signResult = data.withUnsafeBytes { dataBuf -> DashSDKResult in
            dash_sdk_signer_sign(
                signerHandle,
                dataBuf.bindMemory(to: UInt8.self).baseAddress,
                UInt(dataBuf.count)
            )
        }

        if let errPtr = signResult.error {
            let message = errPtr.pointee.message.map { String(cString: $0) } ?? "unknown"
            dash_sdk_error_free(errPtr)
            throw Error.signFailed(message: message)
        }
        guard let sigPtr = signResult.data else {
            throw Error.signFailed(message: "null signature")
        }

        let sigStruct = sigPtr.assumingMemoryBound(to: DashSDKSignature.self)
        defer { dash_sdk_signature_free(sigStruct) }

        guard let bytes = sigStruct.pointee.signature else {
            throw Error.signFailed(message: "empty signature")
        }
        return Data(bytes: bytes, count: Int(sigStruct.pointee.signature_len))
    }
}
