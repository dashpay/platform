import Foundation
import LocalAuthentication
import Security

// MARK: - Wallet Storage

/// Keychain-backed storage for wallet secrets.
///
/// The type used to carry three legacy storage modes — a
/// PIN-encrypted seed at `wallet.seed`, a single-wallet mnemonic at
/// `wallet.mnemonic`, and a PIN hash at `wallet.pin`. None of those
/// had live callers, the PIN flow was never surfaced in the UI, and
/// mixing single-wallet and per-wallet storage in the same type
/// muddied the data model. They've all been removed; the
/// [`cleanupLegacyItems`](x-source-tag://cleanupLegacyItems) helper
/// wipes any residue from prior installs.
///
/// Current responsibilities:
///
/// * Per-wallet mnemonic storage at
///   `wallet.mnemonic.<64-char-hex-walletId>`.
/// * Enumeration of stored wallet ids (used by the orphan-mnemonic
///   recovery flow in `ContentView`).
/// * Biometric-protected seed stash at `wallet.biometric` — not yet
///   wired to a caller but kept because it's a different category
///   (hardware-protected rather than a legacy PIN construct).
public class WalletStorage {
    /// Unified keychain service name for the app. Everything the
    /// SDK writes — per-wallet mnemonics (here), identity private
    /// keys, special keys (in `KeychainManager`) — is filed under
    /// this single service so the keychain explorer + future
    /// cross-item queries see one namespace. Legacy services are
    /// scrubbed on launch by `cleanupLegacyItems`.
    public static let keychainService = "org.dashfoundation.wallet"

    /// Per-instance alias that lets the rest of this file stay
    /// short; the static constant is the one external callers
    /// should reach for.
    private var keychainService: String { Self.keychainService }
    /// Base account string used to build per-wallet mnemonic
    /// accounts via `perWalletMnemonicAccount(for:)`. The legacy
    /// single-mnemonic row at the bare `"wallet.mnemonic"` account
    /// is no longer stored — see `cleanupLegacyItems`.
    private let mnemonicKeychainAccount = "wallet.mnemonic"
    private let biometricKeychainAccount = "wallet.biometric"

    public init() {}

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

    // MARK: - Legacy Cleanup

    /// Best-effort scrub of keychain residue from prior app
    /// architectures. Two kinds of cleanup:
    ///
    /// 1. **Whole-service wipe** of the legacy namespaces we've now
    ///    consolidated under `keychainService` (currently
    ///    `org.dashfoundation.wallet`): `org.dash.wallet` (old
    ///    WalletStorage service), `com.dash.sdk.keys` (old SDK
    ///    `KeychainManager` default), and
    ///    `com.dash.swiftexampleapp.keys` (old app-level
    ///    `KeychainManager` wrapper). Everything filed under those
    ///    services is deleted — on this dev example app it's
    ///    acceptable to lose key material rather than carry
    ///    migration code for every shape.
    /// 2. **Legacy account wipe** of the three pre-per-wallet items
    ///    (`wallet.seed` / `wallet.mnemonic` / `wallet.pin`) under
    ///    the **current** service, in case they ever leaked into
    ///    the new namespace.
    ///
    /// Safe to call on a fresh install — each `SecItemDelete`
    /// returns `errSecItemNotFound` and the method ignores every
    /// status. Per-wallet mnemonic rows under the current service
    /// (`wallet.mnemonic.<hex-walletId>`) are unaffected because
    /// the per-account deletions match on the full account string,
    /// not a prefix.
    ///
    /// Called once per launch from `SwiftExampleAppApp.bootstrap`.
    ///
    /// - Tag: cleanupLegacyItems
    public static func cleanupLegacyItems() {
        // (1) Whole-service wipes for renamed / retired services.
        let legacyServices = [
            "org.dash.wallet",
            "com.dash.sdk.keys",
            "com.dash.swiftexampleapp.keys",
        ]
        for service in legacyServices {
            let query: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: service,
            ]
            _ = SecItemDelete(query as CFDictionary)
        }

        // (2) Targeted legacy-account wipes under the current
        // consolidated service. Defensive — if anything ever wrote
        // these accounts into the new namespace we clear them here.
        let legacyAccounts = ["wallet.seed", "wallet.mnemonic", "wallet.pin"]
        for account in legacyAccounts {
            let query: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: keychainService,
                kSecAttrAccount as String: account,
            ]
            _ = SecItemDelete(query as CFDictionary)
        }
    }
}

// MARK: - Wallet Storage Errors

public enum WalletStorageError: LocalizedError {
    case keychainError(OSStatus)
    case mnemonicNotFound
    case biometricSetupFailed
    case biometricAuthenticationFailed

    public var errorDescription: String? {
        switch self {
        case .keychainError(let status):
            return "Keychain error: \(status)"
        case .mnemonicNotFound:
            return "Mnemonic not found"
        case .biometricSetupFailed:
            return "Failed to setup biometric protection"
        case .biometricAuthenticationFailed:
            return "Biometric authentication failed"
        }
    }
}
