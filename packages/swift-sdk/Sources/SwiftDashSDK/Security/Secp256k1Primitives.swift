import Foundation
import DashSDKFFI

/// Standalone secp256k1 primitives exposed by the Platform FFI.
///
/// These are handle-free on purpose: DashConnect validates scanned QR payloads
/// before a wallet is resolved, so the URI path cannot depend on any wallet instance.
public enum Secp256k1Primitives {
    /// True when `pubKey` is a well-formed 33-byte compressed secp256k1 point.
    public static func isValidCompressedPoint(_ pubKey: Data) -> Bool {
        guard pubKey.count == 33 else {
            return false
        }

        return pubKey.withUnsafeBytes { rawBuffer in
            let bytes = rawBuffer.bindMemory(to: UInt8.self)
            return platform_wallet_secp256k1_verify_compressed_point(
                bytes.baseAddress,
                UInt(bytes.count)
            ) == 1
        }
    }

    /// The 33-byte compressed public key for a 32-byte secp256k1 private key.
    public static func compressedPublicKey(privateKey: Data) throws -> Data {
        var output = [UInt8](repeating: 0, count: 33)

        let result = privateKey.withUnsafeBytes { rawBuffer -> PlatformWalletFFIResult in
            let bytes = rawBuffer.bindMemory(to: UInt8.self)
            return output.withUnsafeMutableBufferPointer { outputBuffer in
                platform_wallet_secp256k1_compressed_public_key(
                    bytes.baseAddress,
                    UInt(bytes.count),
                    outputBuffer.baseAddress
                )
            }
        }

        try result.check()
        return Data(output)
    }

    /// The 32-byte raw affine X coordinate of `privateKey * publicKey`.
    public static func ecdhSharedX(privateKey: Data, publicKey: Data) throws -> Data {
        // The FFI writes the shared secret directly into the returned buffer:
        // a separate scratch array would leave a second, unscrubbed copy of
        // the session secret in allocator memory once it was copied into `Data`.
        var output = Data(count: 32)

        let result = output.withUnsafeMutableBytes { outputBuffer -> PlatformWalletFFIResult in
            privateKey.withUnsafeBytes { privateBuffer in
                publicKey.withUnsafeBytes { publicBuffer in
                    let privateBytes = privateBuffer.bindMemory(to: UInt8.self)
                    let publicBytes = publicBuffer.bindMemory(to: UInt8.self)
                    return platform_wallet_secp256k1_ecdh_shared_x(
                        privateBytes.baseAddress,
                        UInt(privateBytes.count),
                        publicBytes.baseAddress,
                        UInt(publicBytes.count),
                        outputBuffer.bindMemory(to: UInt8.self).baseAddress
                    )
                }
            }
        }

        try result.check()
        return output
    }
}
