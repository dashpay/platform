import Foundation
import SwiftData
import SwiftDashSDK

// Re-export SDK type for backward compatibility
public typealias PersistentPublicKey = SwiftDashSDK.PersistentPublicKey

// App-specific extensions that depend on KeychainManager
extension SwiftDashSDK.PersistentPublicKey {
    /// Check if this public key has an associated private key available in keychain
    @MainActor
    var hasPrivateKey: Bool {
        privateKeyKeychainIdentifier != nil && isPrivateKeyAvailable
    }

    /// Check if the private key is still available in keychain
    @MainActor
    var isPrivateKeyAvailable: Bool {
        guard privateKeyKeychainIdentifier != nil else { return false }
        return KeychainManager.shared.hasPrivateKey(identityId: Data.identifier(fromBase58: identityId) ?? Data(), keyIndex: keyId)
    }

    /// Retrieve the private key data from keychain
    @MainActor
    func getPrivateKeyData() -> Data? {
        guard let identityData = Data.identifier(fromBase58: identityId) else { return nil }
        return KeychainManager.shared.retrievePrivateKey(identityId: identityData, keyIndex: keyId)
    }

    /// Store a private key for this public key
    @MainActor
    func setPrivateKey(_ privateKeyData: Data) {
        guard let identityData = Data.identifier(fromBase58: identityId) else { return }
        if let keychainId = KeychainManager.shared.storePrivateKey(privateKeyData, identityId: identityData, keyIndex: keyId) {
            self.privateKeyKeychainIdentifier = keychainId
        }
    }

    /// Remove the private key from keychain
    @MainActor
    func removePrivateKey() {
        guard let identityData = Data.identifier(fromBase58: identityId) else { return }
        _ = KeychainManager.shared.deletePrivateKey(identityId: identityData, keyIndex: keyId)
        self.privateKeyKeychainIdentifier = nil
    }
}
