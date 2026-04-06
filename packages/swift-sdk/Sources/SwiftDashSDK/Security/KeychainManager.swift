import Foundation
import Security

// MARK: - Supporting Types

/// Types of special keys (voting, owner, payout) for masternode operations
public enum SpecialKeyType: String, Sendable {
    case voting = "voting"
    case owner = "owner"
    case payout = "payout"
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
