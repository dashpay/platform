import Foundation

/// Helper for parsing WIF (Wallet Import Format) private keys
enum WIFParser {
    
    /// Parse a WIF-encoded private key
    /// - Parameter wif: The WIF string
    /// - Returns: The raw private key data (32 bytes) if valid, nil otherwise
    static func parseWIF(_ wif: String) -> Data? {
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
    
    /// Encode a private key to WIF format
    /// - Parameters:
    ///   - privateKey: The raw private key data (32 bytes)
    ///   - isTestnet: Whether to encode for testnet (default true)
    /// - Returns: The WIF-encoded string if successful, nil otherwise
    static func encodeToWIF(_ privateKey: Data, isTestnet: Bool = true) -> String? {
        guard privateKey.count == 32 else { return nil }
        
        // Version byte: 0xef for testnet, 0x80 for mainnet
        let versionByte: UInt8 = isTestnet ? 0xef : 0x80
        
        // Combine version byte + private key
        var data = Data([versionByte])
        data.append(privateKey)
        
        // Calculate checksum (double SHA256)
        let hash1 = sha256(data)
        let hash2 = sha256(hash1)
        let checksum = hash2.prefix(4)
        
        // Append checksum
        data.append(checksum)
        
        // Encode to Base58
        return encodeBase58(data)
    }
    
    /// Encode data to Base58
    private static func encodeBase58(_ data: Data) -> String {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        
        if data.isEmpty { return "" }
        
        // Count leading zeros
        let zeroCount = data.prefix(while: { $0 == 0 }).count
        
        // Convert data to big integer
        let num = data.reduce(into: [UInt8]()) { result, byte in
            var carry = UInt(byte)
            for i in 0..<result.count {
                carry += UInt(result[i]) << 8
                result[i] = UInt8(carry % 58)
                carry /= 58
            }
            while carry > 0 {
                result.append(UInt8(carry % 58))
                carry /= 58
            }
        }
        
        // Convert to string
        var encoded = ""
        for digit in num.reversed() {
            encoded.append(alphabet[alphabet.index(alphabet.startIndex, offsetBy: Int(digit))])
        }
        
        // Add '1' for each leading zero byte
        encoded = String(repeating: "1", count: zeroCount) + encoded
        
        return encoded
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
