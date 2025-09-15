import Foundation
import LocalAuthentication
import Security
import CryptoKit

// MARK: - Wallet Storage

public class WalletStorage {
    private let keychainService = "org.dash.wallet"
    private let seedKeychainAccount = "wallet.seed"
    private let pinKeychainAccount = "wallet.pin"
    private let biometricKeychainAccount = "wallet.biometric"
    
    // MARK: - Seed Storage
    
    public func storeSeed(_ seed: Data, pin: String) throws -> Data {
        // Derive encryption key from PIN
        let salt = generateSalt()
        let key = try deriveKey(from: pin, salt: salt)
        
        // Encrypt seed
        let encryptedSeed = try encryptData(seed, with: key)
        
        // Store salt with encrypted seed
        var storedData = Data()
        storedData.append(salt)
        storedData.append(encryptedSeed)
        
        // Store in keychain with biometric protection if available
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount,
            kSecValueData as String: storedData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        
        // Delete existing if any
        SecItemDelete(query as CFDictionary)
        
        // Add new
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        
        // Store PIN hash separately for verification
        try storePINHash(pin)
        
        return storedData
    }
    
    public func retrieveSeed(pin: String) throws -> Data {
        // Verify PIN first
        guard try verifyPIN(pin) else {
            throw WalletStorageError.invalidPIN
        }
        
        // Retrieve encrypted seed from keychain
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount,
            kSecReturnData as String: true
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess,
              let storedData = result as? Data,
              storedData.count > 32 else {
            throw WalletStorageError.seedNotFound
        }
        
        // Extract salt and encrypted seed
        let salt = storedData.prefix(32)
        let encryptedSeed = storedData.suffix(from: 32)
        
        // Derive key from PIN
        let key = try deriveKey(from: pin, salt: Data(salt))
        
        // Decrypt seed
        return try decryptData(encryptedSeed, with: key)
    }
    
    public func deleteSeed() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount
        ]
        
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw WalletStorageError.keychainError(status)
        }
    }
    
    // MARK: - PIN Management
    
    private func storePINHash(_ pin: String) throws {
        let pinData = Data(pin.utf8)
        let hash = SHA256.hash(data: pinData)
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: pinKeychainAccount,
            kSecValueData as String: Data(hash),
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        
        SecItemDelete(query as CFDictionary)
        
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
    }
    
    private func verifyPIN(_ pin: String) throws -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: pinKeychainAccount,
            kSecReturnData as String: true
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess,
              let storedHash = result as? Data else {
            return false
        }
        
        let pinData = Data(pin.utf8)
        let hash = SHA256.hash(data: pinData)
        
        return Data(hash) == storedHash
    }
    
    // MARK: - Biometric Protection
    
    public func enableBiometricProtection(for seed: Data) throws {
        // Create access control with biometric authentication
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(
            nil,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .biometryCurrentSet,
            &error
        ) else {
            throw WalletStorageError.biometricSetupFailed
        }
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: biometricKeychainAccount,
            kSecValueData as String: seed,
            kSecAttrAccessControl as String: access
        ]
        
        SecItemDelete(query as CFDictionary)
        
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
    }
    
    public func retrieveSeedWithBiometric() throws -> Data {
        let context = LAContext()
        context.localizedReason = "Authenticate to access your wallet"
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: biometricKeychainAccount,
            kSecReturnData as String: true,
            kSecUseAuthenticationContext as String: context
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess,
              let seed = result as? Data else {
            throw WalletStorageError.biometricAuthenticationFailed
        }
        
        return seed
    }
    
    // MARK: - Encryption Helpers
    
    private func generateSalt() -> Data {
        var salt = Data(count: 32)
        _ = salt.withUnsafeMutableBytes { bytes in
            SecRandomCopyBytes(kSecRandomDefault, 32, bytes.baseAddress!)
        }
        return salt
    }
    
    private func deriveKey(from pin: String, salt: Data) throws -> SymmetricKey {
        let pinData = Data(pin.utf8)
        
        // Use PBKDF2 for key derivation
        var derivedKey = Data(count: 32)
        let result = derivedKey.withUnsafeMutableBytes { derivedKeyBytes in
            salt.withUnsafeBytes { saltBytes in
                pinData.withUnsafeBytes { pinBytes in
                    CCKeyDerivationPBKDF(
                        CCPBKDFAlgorithm(kCCPBKDF2),
                        pinBytes.baseAddress!.assumingMemoryBound(to: Int8.self),
                        pinData.count,
                        saltBytes.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        salt.count,
                        CCPseudoRandomAlgorithm(kCCPRFHmacAlgSHA256),
                        10000, // iterations
                        derivedKeyBytes.baseAddress!.assumingMemoryBound(to: UInt8.self),
                        32
                    )
                }
            }
        }
        
        guard result == kCCSuccess else {
            throw WalletStorageError.keyDerivationFailed
        }
        
        return SymmetricKey(data: derivedKey)
    }
    
    private func encryptData(_ data: Data, with key: SymmetricKey) throws -> Data {
        let sealed = try AES.GCM.seal(data, using: key)
        guard let combined = sealed.combined else {
            throw WalletStorageError.encryptionFailed
        }
        return combined
    }
    
    private func decryptData(_ data: Data, with key: SymmetricKey) throws -> Data {
        let box = try AES.GCM.SealedBox(combined: data)
        return try AES.GCM.open(box, using: key)
    }
}

// MARK: - Wallet Storage Errors

public enum WalletStorageError: LocalizedError {
    case keychainError(OSStatus)
    case seedNotFound
    case invalidPIN
    case biometricSetupFailed
    case biometricAuthenticationFailed
    case keyDerivationFailed
    case encryptionFailed
    case decryptionFailed
    
    public var errorDescription: String? {
        switch self {
        case .keychainError(let status):
            return "Keychain error: \(status)"
        case .seedNotFound:
            return "Wallet seed not found"
        case .invalidPIN:
            return "Invalid PIN"
        case .biometricSetupFailed:
            return "Failed to setup biometric protection"
        case .biometricAuthenticationFailed:
            return "Biometric authentication failed"
        case .keyDerivationFailed:
            return "Failed to derive encryption key"
        case .encryptionFailed:
            return "Failed to encrypt data"
        case .decryptionFailed:
            return "Failed to decrypt data"
        }
    }
}

// MARK: - CommonCrypto Import

import CommonCrypto

extension WalletStorage {
    // Bridge for CommonCrypto since it's not available in Swift
    private func CCKeyDerivationPBKDF(
        _ algorithm: CCPBKDFAlgorithm,
        _ password: UnsafePointer<Int8>,
        _ passwordLen: Int,
        _ salt: UnsafePointer<UInt8>,
        _ saltLen: Int,
        _ prf: CCPseudoRandomAlgorithm,
        _ rounds: UInt32,
        _ derivedKey: UnsafeMutablePointer<UInt8>,
        _ derivedKeyLen: Int
    ) -> Int32 {
        return CCCryptorStatus(
            CommonCrypto.CCKeyDerivationPBKDF(
                algorithm,
                password,
                passwordLen,
                salt,
                saltLen,
                prf,
                rounds,
                derivedKey,
                derivedKeyLen
            )
        )
    }
}
