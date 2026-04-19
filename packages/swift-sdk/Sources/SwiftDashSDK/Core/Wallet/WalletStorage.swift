import Foundation
import LocalAuthentication
import Security
import CryptoKit

// MARK: - Wallet Storage

public class WalletStorage {
    private let keychainService = "org.dash.wallet"
    private let seedKeychainAccount = "wallet.seed"
    private let mnemonicKeychainAccount = "wallet.mnemonic"
    private let pinKeychainAccount = "wallet.pin"
    private let biometricKeychainAccount = "wallet.biometric"

    public init() {}

    // MARK: - Seed Storage

    /// Store the wallet seed encrypted with the user's PIN.
    ///
    /// The seed is encrypted using AES-GCM with a key derived from the PIN
    /// via PBKDF2 (10,000 iterations, SHA-256). A random 32-byte salt is
    /// prepended to the ciphertext. The PIN hash is stored separately for
    /// verification on retrieval.
    ///
    /// - Parameters:
    ///   - seed: The raw 64-byte BIP39 seed.
    ///   - pin: The user's plaintext PIN used to derive the encryption key.
    /// - Returns: The stored data (salt + ciphertext) for round-trip verification.
    public func storeSeed(_ seed: Data, pin: String) throws -> Data {
        let salt = generateSalt()
        let key = try deriveKey(from: pin, salt: salt)
        let encryptedSeed = try encryptData(seed, with: key)

        var storedData = Data()
        storedData.append(salt)
        storedData.append(encryptedSeed)

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount
        ]

        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount,
            kSecValueData as String: storedData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }

        try storePINHash(pin)

        return storedData
    }

    /// Retrieve and decrypt the wallet seed using the user's PIN.
    ///
    /// Verifies the PIN against the stored hash before attempting decryption.
    ///
    /// - Parameter pin: The user's plaintext PIN.
    /// - Returns: The decrypted 64-byte BIP39 seed.
    public func retrieveSeed(pin: String) throws -> Data {
        guard try verifyPIN(pin) else {
            throw WalletStorageError.invalidPIN
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: seedKeychainAccount,
            kSecReturnData as String: true
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        if status == errSecItemNotFound {
            throw WalletStorageError.seedNotFound
        }
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        guard let storedData = result as? Data,
              storedData.count > 32 else {
            throw WalletStorageError.seedNotFound
        }

        let salt = storedData.prefix(32)
        let encryptedSeed = storedData.suffix(from: 32)
        let key = try deriveKey(from: pin, salt: Data(salt))
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

    // MARK: - Mnemonic Storage

    /// Store the wallet mnemonic (BIP39 recovery phrase) in the keychain.
    ///
    /// Plain keychain storage with no PIN encryption — iOS keychain hardware
    /// protection (`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`) is the
    /// security boundary.
    ///
    /// - Parameter mnemonic: The space-separated BIP39 mnemonic phrase.
    public func storeMnemonic(_ mnemonic: String) throws {
        let data = Data(mnemonic.utf8)

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: mnemonicKeychainAccount
        ]

        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: mnemonicKeychainAccount,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Retrieve the wallet mnemonic from the keychain.
    ///
    /// - Returns: The space-separated BIP39 mnemonic phrase.
    /// - Throws: `WalletStorageError.mnemonicNotFound` if no mnemonic is stored,
    ///   or `WalletStorageError.keychainError` for other keychain failures.
    public func retrieveMnemonic() throws -> String {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: mnemonicKeychainAccount,
            kSecReturnData as String: true
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        if status == errSecItemNotFound {
            throw WalletStorageError.mnemonicNotFound
        }
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        guard let data = result as? Data,
              let mnemonic = String(data: data, encoding: .utf8),
              !mnemonic.isEmpty else {
            throw WalletStorageError.mnemonicNotFound
        }

        return mnemonic
    }

    /// Delete the wallet mnemonic from the keychain.
    ///
    /// Idempotent — succeeds silently if no mnemonic is stored.
    public func deleteMnemonic() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: mnemonicKeychainAccount
        ]

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw WalletStorageError.keychainError(status)
        }
    }

    // MARK: - Per-Wallet Mnemonic Storage
    //
    // Multi-wallet variant keyed by the 32-byte walletId. Stores each
    // mnemonic at account `wallet.mnemonic.<hex-walletId>` so any
    // number of wallets can coexist.

    private func perWalletMnemonicAccount(for walletId: Data) -> String {
        let hex = walletId.map { String(format: "%02x", $0) }.joined()
        return "\(mnemonicKeychainAccount).\(hex)"
    }

    /// Store a mnemonic keyed by wallet id.
    public func storeMnemonic(_ mnemonic: String, for walletId: Data) throws {
        let data = Data(mnemonic.utf8)
        let account = perWalletMnemonicAccount(for: walletId)

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account
        ]
        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Retrieve a mnemonic keyed by wallet id.
    public func retrieveMnemonic(for walletId: Data) throws -> String {
        let account = perWalletMnemonicAccount(for: walletId)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        if status == errSecItemNotFound {
            throw WalletStorageError.mnemonicNotFound
        }
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        guard let data = result as? Data,
              let mnemonic = String(data: data, encoding: .utf8),
              !mnemonic.isEmpty else {
            throw WalletStorageError.mnemonicNotFound
        }
        return mnemonic
    }

    /// Delete a mnemonic keyed by wallet id. Idempotent.
    public func deleteMnemonic(for walletId: Data) throws {
        let account = perWalletMnemonicAccount(for: walletId)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Enumerate all wallet ids with a stored mnemonic.
    ///
    /// Reads every `kSecClassGenericPassword` entry under
    /// `org.dash.wallet`, keeps those whose account starts with
    /// `wallet.mnemonic.` followed by a 64-character lowercase hex
    /// string, and decodes the suffix back into 32-byte wallet ids.
    /// Returns an empty array if the keychain has none.
    public func listWalletIdsWithMnemonic() throws -> [Data] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return []
        }
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        guard let items = result as? [[String: Any]] else { return [] }

        let prefix = "\(mnemonicKeychainAccount)."
        var walletIds: [Data] = []
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                  account.hasPrefix(prefix) else { continue }
            let hex = String(account.dropFirst(prefix.count))
            guard hex.count == 64, let walletId = Self.dataFromHex(hex) else { continue }
            walletIds.append(walletId)
        }
        return walletIds
    }

    /// Decode a lowercase hex string into bytes. Returns nil on any
    /// non-hex character or an odd length.
    private static func dataFromHex(_ hex: String) -> Data? {
        guard hex.count % 2 == 0 else { return nil }
        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            guard let byte = UInt8(hex[index..<next], radix: 16) else { return nil }
            data.append(byte)
            index = next
        }
        return data
    }

    // MARK: - PIN Management

    private func storePINHash(_ pin: String) throws {
        let pinData = Data(pin.utf8)
        let hash = SHA256.hash(data: pinData)

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: pinKeychainAccount
        ]

        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: pinKeychainAccount,
            kSecValueData as String: Data(hash),
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
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
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(
            nil,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .biometryCurrentSet,
            &error
        ) else {
            throw WalletStorageError.biometricSetupFailed
        }

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: biometricKeychainAccount
        ]

        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: biometricKeychainAccount,
            kSecValueData as String: seed,
            kSecAttrAccessControl as String: access
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
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
    case mnemonicNotFound

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
        case .mnemonicNotFound:
            return "Mnemonic not found"
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
