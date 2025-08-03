import Foundation
import CryptoKit

/// Test key generator for demo purposes only
/// DO NOT USE IN PRODUCTION - This generates deterministic keys which are insecure
struct TestKeyGenerator {
    
    /// Generate a deterministic private key from identity ID (FOR DEMO ONLY)
    static func generateTestPrivateKey(identityId: Data, keyIndex: UInt32, purpose: UInt8) -> Data {
        // Create deterministic seed from identity ID, key index, and purpose
        var seedData = Data()
        seedData.append(identityId)
        seedData.append(contentsOf: withUnsafeBytes(of: keyIndex) { Data($0) })
        seedData.append(purpose)
        
        // Use SHA256 to generate a 32-byte private key
        let hash = SHA256.hash(data: seedData)
        return Data(hash)
    }
    
    /// Generate test private keys for an identity
    static func generateTestPrivateKeys(identityId: Data) -> [String: Data] {
        var keys: [String: Data] = [:]
        
        // Generate keys for different purposes
        // Key 0: Master key (not used in state transitions)
        keys["0"] = generateTestPrivateKey(identityId: identityId, keyIndex: 0, purpose: 0)
        
        // Key 1: Authentication key (HIGH security)
        keys["1"] = generateTestPrivateKey(identityId: identityId, keyIndex: 1, purpose: 0)
        
        // Key 2: Transfer key (CRITICAL security)
        keys["2"] = generateTestPrivateKey(identityId: identityId, keyIndex: 2, purpose: 2)
        
        return keys
    }
    
    /// Get private key for a specific key ID
    static func getPrivateKey(identityId: Data, keyId: UInt32) -> Data? {
        let keys = generateTestPrivateKeys(identityId: identityId)
        return keys[String(keyId)]
    }
}