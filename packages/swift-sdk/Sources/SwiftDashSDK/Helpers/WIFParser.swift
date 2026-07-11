import Foundation
import DashSDKFFI

/// Helper for parsing WIF (Wallet Import Format) private keys.
///
/// Encoding delegates to the Rust FFI's `dash_sdk_private_key_to_wif`
/// — `dashcore::PrivateKey::to_wif()` is the single source of truth
/// for Dash WIF (correct version bytes — `0xCC` mainnet / `0xEF`
/// testnet, NOT Bitcoin's `0x80` / `0xEF` — and the trailing `0x01`
/// compression flag every modern Dash wallet emits). Implementing
/// the byte layout in Swift was the original shape of this file; it
/// silently produced Bitcoin-shaped WIFs (mainnet starting with
/// `5…` / `K…` instead of `7…` / `X…`), so the Swift encoder is
/// gone.
public enum WIFParser {

    /// Parse a WIF-encoded private key
    /// - Parameter wif: The WIF string
    /// - Returns: The raw private key data (32 bytes) if valid, nil otherwise
    public static func parseWIF(_ wif: String) -> Data? {
        // WIF format:
        // - Mainnet: starts with '7' (uncompressed) or 'X' (compressed)
        // - Testnet: starts with 'c' (uncompressed) or 'c' (compressed)

        guard !wif.isEmpty else { return nil }

        // Decode from Base58
        guard let decoded = decodeBase58(wif) else { return nil }

        // WIF structure:
        // - 1 byte: version (0xCC for testnet, 0xD2 for mainnet)
        // - 32 bytes: private key
        // - (optional) 1 byte: 0x01 for compressed public key
        // - 4 bytes: checksum

        let minLength = 1 + 32 + 4  // version + key + checksum
        let maxLength = minLength + 1  // + compression flag

        guard decoded.count >= minLength && decoded.count <= maxLength else {
            return nil
        }

        // Verify checksum
        let checksumStart = decoded.count - 4
        let dataToCheck = decoded[0..<checksumStart]
        let checksum = decoded[checksumStart...]

        let hash = sha256(sha256(dataToCheck))
        guard hash.prefix(4) == checksum else {
            return nil
        }

        // Extract private key (skip version byte)
        let privateKey = decoded[1..<33]

        return Data(privateKey)
    }

    /// Encode a private key to Dash WIF format.
    ///
    /// Delegates to `dash_sdk_private_key_to_wif` so the Dash
    /// version bytes (`0xCC` mainnet / `0xEF` testnet) and the
    /// compressed-key `0x01` flag come from `dashcore`'s canonical
    /// implementation rather than a hand-rolled Swift duplicate.
    /// Returns `nil` on any FFI error (bad key length, allocation
    /// failure, etc.) — matches the prior Swift signature.
    ///
    /// - Parameters:
    ///   - privateKey: The raw 32-byte secp256k1 secret scalar.
    ///   - network: Which network the key belongs to (default: testnet).
    ///     Mainnet uses version byte `0xCC`; every other network uses
    ///     the testnet `0xEF` byte.
    public static func encodeToWIF(_ privateKey: Data, network: Network = .testnet) -> String? {
        guard privateKey.count == 32 else { return nil }

        // The FFI takes a NUL-terminated hex string for the private
        // key. Encode → call → free → parse the returned C string.
        let privateKeyHex = privateKey.toHexString()
        let result = privateKeyHex.withCString { hexPtr in
            dash_sdk_private_key_to_wif(hexPtr, network.ffiValue)
        }

        if let error = result.error {
            dash_sdk_error_free(error)
            return nil
        }
        guard let dataPtr = result.data else { return nil }
        let cstrPtr = dataPtr.assumingMemoryBound(to: CChar.self)
        defer { dash_sdk_string_free(cstrPtr) }
        return String(cString: cstrPtr)
    }

    /// Decode a Base58 string
    private static func decodeBase58(_ string: String) -> Data? {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        var result = Data()
        var multi = Data([0])

        for char in string {
            guard let index = alphabet.firstIndex(of: char) else { return nil }
            let digit = alphabet.distance(from: alphabet.startIndex, to: index)

            // Multiply existing result by 58
            var carry = 0
            for i in (0..<multi.count).reversed() {
                let value = Int(multi[i]) * 58 + carry
                multi[i] = UInt8(value & 0xFF)
                carry = value >> 8
            }

            while carry > 0 {
                multi.insert(UInt8(carry & 0xFF), at: 0)
                carry >>= 8
            }

            // Add the digit
            carry = digit
            for i in (0..<multi.count).reversed() {
                let value = Int(multi[i]) + carry
                multi[i] = UInt8(value & 0xFF)
                carry = value >> 8
            }

            while carry > 0 {
                multi.insert(UInt8(carry & 0xFF), at: 0)
                carry >>= 8
            }
        }

        // Count leading '1's (zeros)
        let zeroCount = string.prefix(while: { $0 == "1" }).count

        // Remove leading zeros from multi
        while multi.count > 1 && multi[0] == 0 {
            multi.remove(at: 0)
        }

        // Add back the leading zeros
        result = Data(repeating: 0, count: zeroCount) + multi

        return result
    }

    /// Simple SHA256 implementation using CommonCrypto
    private static func sha256(_ data: Data) -> Data {
        var hash = [UInt8](repeating: 0, count: 32)
        data.withUnsafeBytes { buffer in
            _ = CC_SHA256(buffer.baseAddress, CC_LONG(data.count), &hash)
        }
        return Data(hash)
    }
}

// Import CommonCrypto for SHA256
import CommonCrypto
