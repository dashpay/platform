import Foundation
import DashSDKFFI
import SwiftDashSDK

/// Helper for validating private keys against public keys
enum KeyValidation {
    /// Validate that a private key matches a public key
    static func validatePrivateKeyForPublicKey(
        privateKeyHex: String,
        publicKeyHex: String,
        keyType: KeyType,
        isTestnet: Bool = true
    ) -> Bool {
        // Convert key type to FFI representation
        let ffiKeyType: UInt8
        switch keyType {
        case .ecdsaSecp256k1:
            ffiKeyType = 0
        case .bls12_381:
            ffiKeyType = 1
        case .ecdsaHash160:
            ffiKeyType = 2
        case .bip13ScriptHash:
            ffiKeyType = 3
        case .eddsa25519Hash160:
            ffiKeyType = 4
        }
        
        let result = privateKeyHex.withCString { privateKeyCStr in
            publicKeyHex.withCString { publicKeyCStr in
                dash_sdk_validate_private_key_for_public_key(privateKeyCStr, publicKeyCStr, ffiKeyType, isTestnet)
            }
        }
        
        // Check for errors
        if result.error != nil {
            let error = result.error!.pointee
            defer {
                dash_sdk_error_free(result.error)
            }
            print("Validation error: \(error.message != nil ? String(cString: error.message!) : "Unknown")")
            return false
        }
        
        guard result.data != nil else {
            print("No validation result data")
            return false
        }
        
        // The result is a string "true" or "false"
        let resultStr = String(cString: result.data.assumingMemoryBound(to: CChar.self))
        
        // Free the result data
        dash_sdk_string_free(result.data.assumingMemoryBound(to: CChar.self))
        
        return resultStr == "true"
    }
    
    /// Match a private key to its corresponding public key in a list of public keys
    /// Returns the matching public key or nil if no match found
    static func matchPrivateKeyToPublicKeys(
        privateKeyData: Data,
        publicKeys: [IdentityPublicKey],
        isTestnet: Bool = true
    ) -> IdentityPublicKey? {
        let privateKeyHex = privateKeyData.toHexString()
        
        for publicKey in publicKeys {
            let publicKeyHex = publicKey.data.toHexString()
            
            if validatePrivateKeyForPublicKey(
                privateKeyHex: privateKeyHex,
                publicKeyHex: publicKeyHex,
                keyType: publicKey.keyType,
                isTestnet: isTestnet
            ) {
                return publicKey
            }
        }
        
        return nil
    }
}