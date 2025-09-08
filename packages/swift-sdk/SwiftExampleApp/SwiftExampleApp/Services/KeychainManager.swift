import Foundation
import Security

/// Manages secure storage of private keys in the iOS Keychain
final class KeychainManager {
    static let shared = KeychainManager()
    
    private let serviceName = "com.dash.swiftexampleapp.keys"
    private let accessGroup: String? = nil // Set this if you need app group sharing
    
    private init() {}
    
    // MARK: - Private Key Storage
    
    /// Store a private key in the keychain
    /// - Parameters:
    ///   - keyData: The private key data
    ///   - identityId: The identity ID
    ///   - keyIndex: The key index
    /// - Returns: A unique identifier for the stored key
    @discardableResult
    func storePrivateKey(_ keyData: Data, identityId: Data, keyIndex: Int32) -> String? {
        let keyIdentifier = generateKeyIdentifier(identityId: identityId, keyIndex: keyIndex)
        
        // Create the query
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: keyIdentifier,
            kSecValueData as String: keyData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrSynchronizable as String: false // Never sync private keys to iCloud
        ]
        
        // Add metadata
        var metadata: [String: Any] = [
            "identityId": identityId.toHexString(),
            "keyIndex": keyIndex,
            "createdAt": Date().timeIntervalSince1970
        ]
        
        if let metadataData = try? JSONSerialization.data(withJSONObject: metadata) {
            query[kSecAttrGeneric as String] = metadataData
        }
        
        // Add access group if specified
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        // Delete any existing item first
        SecItemDelete(query as CFDictionary)
        
        // Add the new item
        let status = SecItemAdd(query as CFDictionary, nil)
        
        if status == errSecSuccess {
            return keyIdentifier
        } else {
            print("Failed to store private key: \(status)")
            return nil
        }
    }
    
    /// Retrieve a private key from the keychain
    func retrievePrivateKey(identityId: Data, keyIndex: Int32) -> Data? {
        let keyIdentifier = generateKeyIdentifier(identityId: identityId, keyIndex: keyIndex)
        print("🔐 KeychainManager: Retrieving key with identifier: \(keyIdentifier)")
        
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: keyIdentifier,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        if status == errSecSuccess {
            let data = result as? Data
            print("🔐 KeychainManager: Retrieved key data: \(data != nil ? "\(data!.count) bytes" : "nil")")
            return data
        } else {
            print("🔐 KeychainManager: Failed to retrieve private key: \(status)")
            return nil
        }
    }
    
    /// Delete a private key from the keychain
    func deletePrivateKey(identityId: Data, keyIndex: Int32) -> Bool {
        let keyIdentifier = generateKeyIdentifier(identityId: identityId, keyIndex: keyIndex)
        
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: keyIdentifier
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound
    }
    
    /// Delete all private keys for an identity
    func deleteAllPrivateKeys(for identityId: Data) -> Bool {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        // First, find all keys for this identity
        var result: AnyObject?
        let searchStatus = SecItemCopyMatching(query as CFDictionary, &result)
        
        if searchStatus == errSecSuccess,
           let items = result as? [[String: Any]] {
            // Filter items for this identity and delete them
            for item in items {
                if let account = item[kSecAttrAccount as String] as? String,
                   account.hasPrefix("privkey_\(identityId.toHexString())_") {
                    var deleteQuery: [String: Any] = [
                        kSecClass as String: kSecClassGenericPassword,
                        kSecAttrService as String: serviceName,
                        kSecAttrAccount as String: account
                    ]
                    
                    if let accessGroup = accessGroup {
                        deleteQuery[kSecAttrAccessGroup as String] = accessGroup
                    }
                    
                    SecItemDelete(deleteQuery as CFDictionary)
                }
            }
        }
        
        return true
    }
    
    // MARK: - Special Keys (Voting, Owner, Payout)
    
    func storeSpecialKey(_ keyData: Data, identityId: Data, keyType: SpecialKeyType) -> String? {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return storeKeyData(keyData, identifier: keyIdentifier)
    }
    
    func retrieveSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Data? {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return retrieveKeyData(identifier: keyIdentifier)
    }
    
    func deleteSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Bool {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return deleteKeyData(identifier: keyIdentifier)
    }
    
    // MARK: - Private Helpers
    
    private func generateKeyIdentifier(identityId: Data, keyIndex: Int32) -> String {
        return "privkey_\(identityId.toHexString())_\(keyIndex)"
    }
    
    private func generateSpecialKeyIdentifier(identityId: Data, keyType: SpecialKeyType) -> String {
        return "specialkey_\(identityId.toHexString())_\(keyType.rawValue)"
    }
    
    private func storeKeyData(_ keyData: Data, identifier: String) -> String? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: identifier,
            kSecValueData as String: keyData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrSynchronizable as String: false
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        SecItemDelete(query as CFDictionary)
        
        let status = SecItemAdd(query as CFDictionary, nil)
        return status == errSecSuccess ? identifier : nil
    }
    
    private func retrieveKeyData(identifier: String) -> Data? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: identifier,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        return status == errSecSuccess ? result as? Data : nil
    }
    
    private func deleteKeyData(identifier: String) -> Bool {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: identifier
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        let status = SecItemDelete(query as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound
    }
    
    // MARK: - Key Existence Check
    
    func hasPrivateKey(identityId: Data, keyIndex: Int32) -> Bool {
        let keyIdentifier = generateKeyIdentifier(identityId: identityId, keyIndex: keyIndex)
        
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: keyIdentifier,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        let status = SecItemCopyMatching(query as CFDictionary, nil)
        return status == errSecSuccess
    }
    
    func hasSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Bool {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: keyIdentifier,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        
        let status = SecItemCopyMatching(query as CFDictionary, nil)
        return status == errSecSuccess
    }
}

// MARK: - Supporting Types

enum SpecialKeyType: String {
    case voting = "voting"
    case owner = "owner"
    case payout = "payout"
}

// MARK: - Error Handling

enum KeychainError: LocalizedError {
    case storeFailed(OSStatus)
    case retrieveFailed(OSStatus)
    case deleteFailed(OSStatus)
    case invalidData
    
    var errorDescription: String? {
        switch self {
        case .storeFailed(let status):
            return "Failed to store key in keychain: \(status)"
        case .retrieveFailed(let status):
            return "Failed to retrieve key from keychain: \(status)"
        case .deleteFailed(let status):
            return "Failed to delete key from keychain: \(status)"
        case .invalidData:
            return "Invalid key data"
        }
    }
}