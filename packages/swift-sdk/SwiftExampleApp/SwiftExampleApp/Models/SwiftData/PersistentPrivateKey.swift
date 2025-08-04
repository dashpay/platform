import Foundation
import SwiftData

/// SwiftData model for persisting private key references
/// The actual key data is stored securely in the iOS Keychain
@Model
final class PersistentPrivateKey {
    @Attribute(.unique) var id: String // identityId_keyIndex
    var identityId: Data
    var keyIndex: Int32
    var keychainIdentifier: String // Reference to the key in keychain
    var createdAt: Date
    var lastAccessed: Date?
    
    init(identityId: Data, keyIndex: Int32, keychainIdentifier: String) {
        self.id = "\(identityId.toHexString())_\(keyIndex)"
        self.identityId = identityId
        self.keyIndex = keyIndex
        self.keychainIdentifier = keychainIdentifier
        self.createdAt = Date()
        self.lastAccessed = nil
    }
    
    /// Retrieve the actual key data from keychain
    func getKeyData() -> Data? {
        lastAccessed = Date()
        return KeychainManager.shared.retrievePrivateKey(identityId: identityId, keyIndex: keyIndex)
    }
    
    /// Check if the key still exists in keychain
    var isAvailable: Bool {
        KeychainManager.shared.hasPrivateKey(identityId: identityId, keyIndex: keyIndex)
    }
}