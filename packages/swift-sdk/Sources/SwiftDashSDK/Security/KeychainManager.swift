import Foundation
import Security

// MARK: - Supporting Types

/// Types of special keys (voting, owner, payout) for masternode operations
public enum SpecialKeyType: String, Sendable {
    case voting = "voting"
    case owner = "owner"
    case payout = "payout"
}

/// Errors that can occur during keychain operations
public enum KeychainError: LocalizedError, Sendable {
    case storeFailed(OSStatus)
    case retrieveFailed(OSStatus)
    case deleteFailed(OSStatus)
    case invalidData

    public var errorDescription: String? {
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

// MARK: - KeychainManager

/// Manages secure storage of private keys in the iOS Keychain.
///
/// This class provides a secure way to store, retrieve, and delete private keys
/// associated with Dash identities. Keys are stored with strong security settings:
/// - Only accessible when device is unlocked
/// - Never synchronized to iCloud
/// - Optionally shared via app groups
///
/// Example usage:
/// ```swift
/// let manager = KeychainManager.shared
///
/// // Store a private key
/// let keyId = manager.storePrivateKey(privateKeyData, identityId: identityData, keyIndex: 0)
///
/// // Retrieve a private key
/// if let privateKey = manager.retrievePrivateKey(identityId: identityData, keyIndex: 0) {
///     // Use the private key
/// }
/// ```
@MainActor
public final class KeychainManager: Sendable {

    /// Shared singleton instance with default service name
    public static let shared = KeychainManager()

    /// The service name used for keychain entries
    public let serviceName: String

    /// Optional access group for sharing keys between apps
    public let accessGroup: String?

    /// Initialize with default service name "com.dash.sdk.keys"
    public init() {
        self.serviceName = "com.dash.sdk.keys"
        self.accessGroup = nil
    }

    /// Initialize with custom service name and optional access group
    /// - Parameters:
    ///   - serviceName: The service name for keychain entries (e.g., "com.myapp.keys")
    ///   - accessGroup: Optional access group for sharing keys between apps
    public init(serviceName: String, accessGroup: String? = nil) {
        self.serviceName = serviceName
        self.accessGroup = accessGroup
    }

    // MARK: - Private Key Storage

    /// Store a private key in the keychain
    /// - Parameters:
    ///   - keyData: The private key data
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyIndex: The key index within the identity
    /// - Returns: A unique identifier for the stored key, or nil if storage failed
    @discardableResult
    public func storePrivateKey(_ keyData: Data, identityId: Data, keyIndex: Int32) -> String? {
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
        let metadata: [String: Any] = [
            "identityId": identityId.map { String(format: "%02x", $0) }.joined(),
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
            print("KeychainManager: Failed to store private key: \(status)")
            return nil
        }
    }

    /// Retrieve a private key from the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyIndex: The key index within the identity
    /// - Returns: The private key data, or nil if not found
    public func retrievePrivateKey(identityId: Data, keyIndex: Int32) -> Data? {
        let keyIdentifier = generateKeyIdentifier(identityId: identityId, keyIndex: keyIndex)

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
            return result as? Data
        } else {
            return nil
        }
    }

    /// Delete a private key from the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyIndex: The key index within the identity
    /// - Returns: true if deletion succeeded or key didn't exist
    @discardableResult
    public func deletePrivateKey(identityId: Data, keyIndex: Int32) -> Bool {
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
    /// - Parameter identityId: The identity ID (32 bytes)
    /// - Returns: true if deletion completed (even if no keys existed)
    @discardableResult
    public func deleteAllPrivateKeys(for identityId: Data) -> Bool {
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

        let identityHex = identityId.map { String(format: "%02x", $0) }.joined()

        if searchStatus == errSecSuccess,
           let items = result as? [[String: Any]] {
            // Filter items for this identity and delete them
            for item in items {
                if let account = item[kSecAttrAccount as String] as? String,
                   account.hasPrefix("privkey_\(identityHex)_") {
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

    /// Store a special key (voting, owner, or payout) in the keychain
    /// - Parameters:
    ///   - keyData: The key data
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyType: The type of special key
    /// - Returns: A unique identifier for the stored key, or nil if storage failed
    @discardableResult
    public func storeSpecialKey(_ keyData: Data, identityId: Data, keyType: SpecialKeyType) -> String? {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return storeKeyData(keyData, identifier: keyIdentifier)
    }

    /// Retrieve a special key from the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyType: The type of special key
    /// - Returns: The key data, or nil if not found
    public func retrieveSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Data? {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return retrieveKeyData(identifier: keyIdentifier)
    }

    /// Delete a special key from the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyType: The type of special key
    /// - Returns: true if deletion succeeded or key didn't exist
    @discardableResult
    public func deleteSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Bool {
        let keyIdentifier = generateSpecialKeyIdentifier(identityId: identityId, keyType: keyType)
        return deleteKeyData(identifier: keyIdentifier)
    }

    // MARK: - Key Existence Check

    /// Check if a private key exists in the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyIndex: The key index within the identity
    /// - Returns: true if the key exists
    public func hasPrivateKey(identityId: Data, keyIndex: Int32) -> Bool {
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

    /// Check if a special key exists in the keychain
    /// - Parameters:
    ///   - identityId: The identity ID (32 bytes)
    ///   - keyType: The type of special key
    /// - Returns: true if the key exists
    public func hasSpecialKey(identityId: Data, keyType: SpecialKeyType) -> Bool {
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

    // MARK: - Generic Key Storage

    /// Store arbitrary data in the keychain with a custom identifier
    /// - Parameters:
    ///   - keyData: The data to store
    ///   - identifier: A unique identifier for the stored data
    /// - Returns: The identifier if storage succeeded, or nil if it failed
    @discardableResult
    public func storeKeyData(_ keyData: Data, identifier: String) -> String? {
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

    /// Retrieve data from the keychain by identifier
    /// - Parameter identifier: The identifier for the stored data
    /// - Returns: The stored data, or nil if not found
    public func retrieveKeyData(identifier: String) -> Data? {
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

    /// Delete data from the keychain by identifier
    /// - Parameter identifier: The identifier for the stored data
    /// - Returns: true if deletion succeeded or data didn't exist
    @discardableResult
    public func deleteKeyData(identifier: String) -> Bool {
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

    // MARK: - Private Helpers

    private func generateKeyIdentifier(identityId: Data, keyIndex: Int32) -> String {
        let identityHex = identityId.map { String(format: "%02x", $0) }.joined()
        return "privkey_\(identityHex)_\(keyIndex)"
    }

    private func generateSpecialKeyIdentifier(identityId: Data, keyType: SpecialKeyType) -> String {
        let identityHex = identityId.map { String(format: "%02x", $0) }.joined()
        return "specialkey_\(identityHex)_\(keyType.rawValue)"
    }
}
