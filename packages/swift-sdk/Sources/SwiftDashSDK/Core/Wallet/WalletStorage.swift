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
/// * Per-wallet user-facing metadata (display name + free-form
///   description) at `wallet.metadata.<64-char-hex-walletId>`,
///   carried as a JSON-encoded `WalletKeychainMetadata` blob so the
///   orphan-mnemonic recovery flow can repopulate the SwiftData row
///   with the original name/description after a reinstall.
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
    public static let keychainService =
        ProcessInfo.processInfo.environment["DASH_KEYCHAIN_SERVICE"]
        ?? "org.dashfoundation.wallet"

    /// Per-instance alias that lets the rest of this file stay
    /// short; the static constant is the one external callers
    /// should reach for.
    private var keychainService: String { Self.keychainService }
    /// Base account string used to build per-wallet mnemonic
    /// accounts via `perWalletMnemonicAccount(for:)`. The legacy
    /// single-mnemonic row at the bare `"wallet.mnemonic"` account
    /// is no longer stored — see `cleanupLegacyItems`.
    private let mnemonicKeychainAccount = "wallet.mnemonic"
    /// Base account string used to build per-wallet metadata
    /// accounts via `perWalletMetadataAccount(for:)`. Read by the
    /// orphan-mnemonic recovery flow so reinstalls can restore the
    /// user-facing wallet name and description from the keychain
    /// even though SwiftData was wiped.
    public static let metadataAccountPrefix = "wallet.metadata"
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

    /// Retrieve the mnemonic UTF-8 bytes keyed by wallet id.
    ///
    /// Returning raw bytes lets security-sensitive call sites avoid
    /// materializing a Swift `String` unless they truly need one.
    public func retrieveMnemonicUTF8Bytes(for walletId: Data) throws -> Data {
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
        guard let data = result as? Data, !data.isEmpty else {
            throw WalletStorageError.mnemonicNotFound
        }
        return data
    }

    /// Retrieve a mnemonic keyed by wallet id.
    public func retrieveMnemonic(for walletId: Data) throws -> String {
        let data = try retrieveMnemonicUTF8Bytes(for: walletId)
        guard let mnemonic = String(data: data, encoding: .utf8), !mnemonic.isEmpty else {
            throw WalletStorageError.mnemonicNotFound
        }
        return mnemonic
    }

    /// Cheap existence check used by signer preflight paths.
    ///
    /// Unlike `retrieveMnemonic(...)`, this does not materialize the
    /// mnemonic bytes into Swift heap objects.
    public func hasMnemonic(for walletId: Data) -> Bool {
        let account = perWalletMnemonicAccount(for: walletId)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        return status == errSecSuccess
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

    // MARK: - Per-Wallet Metadata Storage
    //
    // User-facing display strings (name + description) carried in
    // the keychain so reinstalls / orphan-mnemonic recovery can
    // repopulate the corresponding `PersistentWallet` row. The blob
    // is intentionally tiny — only fields the user typed should
    // live here, not derived/cached state like sync heights.

    private func perWalletMetadataAccount(for walletId: Data) -> String {
        let hex = walletId.map { String(format: "%02x", $0) }.joined()
        return "\(Self.metadataAccountPrefix).\(hex)"
    }

    /// Write (or replace) the metadata blob for `walletId`. Uses the
    /// delete-then-add pattern matching `storeMnemonic` so the
    /// `kSecAttrAccessible` value is rewritten on every save.
    public func setMetadata(_ metadata: WalletKeychainMetadata, for walletId: Data) throws {
        let data = try JSONEncoder().encode(metadata)
        let account = perWalletMetadataAccount(for: walletId)

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

    /// Read back the metadata blob for `walletId`. Returns `nil` on
    /// `errSecItemNotFound` so the orphan-recovery flow can
    /// distinguish "no metadata stored" from a hard keychain error.
    public func metadata(for walletId: Data) throws -> WalletKeychainMetadata? {
        let account = perWalletMetadataAccount(for: walletId)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
        guard let data = result as? Data, !data.isEmpty else {
            return nil
        }
        // Decode failures here mean a corrupted blob — treat as
        // "no metadata available" rather than a hard error so the
        // wallet can still be recovered. The caller logs and falls
        // back to the placeholder name.
        return try? JSONDecoder().decode(WalletKeychainMetadata.self, from: data)
    }

    /// Delete the metadata blob keyed by `walletId`. Idempotent.
    public func deleteMetadata(for walletId: Data) throws {
        let account = perWalletMetadataAccount(for: walletId)
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
    /// status. Per-wallet rows under the current service
    /// (`wallet.mnemonic.<hex-walletId>` and the matching
    /// `wallet.metadata.<hex-walletId>` blobs written by
    /// `setMetadata`) are unaffected because the per-account
    /// deletions match on the full account string, not a prefix.
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

// MARK: - Wallet Keychain Metadata

/// User-facing / user-intent wallet metadata persisted in the
/// keychain so it can be carried across SwiftData wipes (orphan
/// recovery, app reinstalls). Intentionally minimal — only fields
/// the user explicitly chose (name, description, networks) plus
/// values the user can't recompute cheaply (`birthHeight`, which
/// the SPV tip at creation locks in for the lifetime of the
/// wallet). Derived / cached state like `syncedHeight` belongs in
/// SwiftData, not here.
public struct WalletKeychainMetadata: Codable, Equatable {
    /// Display name the user assigned to the wallet, if any.
    public var name: String?
    /// Optional free-form description the user typed alongside the
    /// name. Currently no UI to set this — the field is wired
    /// through so future write paths can populate it without a
    /// schema migration.
    public var walletDescription: String?
    /// Networks the user explicitly enabled on this wallet, as
    /// stable string codes (`Network.networkName` —
    /// `"mainnet"` / `"testnet"` / `"devnet"` / `"regtest"`).
    /// Strings rather than raw `UInt32`s so a future enum addition
    /// doesn't crash old clients reading new blobs — unknown codes
    /// just get filtered out at decode time. `nil` on rows written
    /// before this field landed; recovery falls back to its old
    /// testnet default.
    public var networks: [String]?
    /// SPV chain tip at the moment the wallet was originally
    /// created. The first SPV scan starts from this height instead
    /// of genesis, so preserving it across a reinstall avoids
    /// re-scanning years of irrelevant history. `nil` for blobs
    /// written before this field landed (or for imported wallets
    /// where we don't yet capture a genesis-distance estimate);
    /// callers fall back to the live SPV tip in that case.
    public var birthHeight: UInt32?

    public init(
        name: String? = nil,
        walletDescription: String? = nil,
        networks: [String]? = nil,
        birthHeight: UInt32? = nil
    ) {
        self.name = name
        self.walletDescription = walletDescription
        self.networks = networks
        self.birthHeight = birthHeight
    }

    /// Decoded `networks` array as `Network` values, dropping any
    /// strings that don't match a known case so unknown future
    /// codes don't blow up recovery on an older client.
    public var resolvedNetworks: [Network] {
        guard let networks else { return [] }
        return networks.compactMap { code in
            switch code.lowercased() {
            case "mainnet": return .mainnet
            case "testnet": return .testnet
            case "devnet": return .devnet
            case "regtest": return .regtest
            default: return nil
            }
        }
    }

    /// JSON keys are stable and short — `description` collides with
    /// `CustomStringConvertible.description` on the Swift side but
    /// is the natural name on disk, so we do the rename in the
    /// `CodingKeys`. `birthHeight` is camel-cased to match the JS /
    /// SwiftData side, both of which already use that spelling.
    private enum CodingKeys: String, CodingKey {
        case name
        case walletDescription = "description"
        case networks
        case birthHeight
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
