import Foundation

/// Test signer implementation for the example app
/// In a real app, this would integrate with iOS Keychain or biometric authentication
class TestSigner: Signer {
    private var privateKeys: [String: Data] = [:]
    
    init() {
        // Initialize with some test private keys for demo purposes
        // In a real app, these would be securely stored and retrieved
        privateKeys["11111111111111111111111111111111"] = Data(repeating: 0x01, count: 32)
        privateKeys["22222222222222222222222222222222"] = Data(repeating: 0x02, count: 32)
        privateKeys["33333333333333333333333333333333"] = Data(repeating: 0x03, count: 32)
    }
    
    func sign(identityPublicKey: Data, data: Data) -> Data? {
        // In a real implementation, this would:
        // 1. Find the identity by its public key
        // 2. Retrieve the corresponding private key from secure storage
        // 3. Sign the data using the private key
        // 4. Return the signature
        
        // For demo purposes, we'll create a mock signature
        // based on the public key and data
        var signature = Data()
        signature.append(contentsOf: "SIGNATURE:".utf8)
        signature.append(identityPublicKey.prefix(32))
        signature.append(data.prefix(32))
        
        // Ensure signature is at least 64 bytes (typical for ECDSA)
        while signature.count < 64 {
            signature.append(0)
        }
        
        return signature
    }
    
    func canSign(identityPublicKey: Data) -> Bool {
        // In a real implementation, check if we have the private key
        // corresponding to this public key
        // For demo purposes, return true for known test identities
        return true
    }
    
    func addPrivateKey(_ key: Data, forIdentity identityId: String) {
        privateKeys[identityId] = key
    }
    
    func removePrivateKey(forIdentity identityId: String) {
        privateKeys.removeValue(forKey: identityId)
    }
}