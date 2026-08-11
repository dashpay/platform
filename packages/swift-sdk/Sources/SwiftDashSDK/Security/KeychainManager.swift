import Foundation
import Security
import os.log

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

    /// Shared singleton instance with default service name.
    ///
    /// `nonisolated` so the singleton can be reached from off-actor
    /// contexts — e.g. the Rust persister callback path that writes
    /// via `storePrivateKeyNonisolated`. Safe because
    /// `KeychainManager` is `Sendable` (all state is `let` +
    /// thread-safe Security-framework calls).
    public nonisolated static let shared = KeychainManager()

    /// The service name used for keychain entries
    public let serviceName: String

    /// Optional access group for sharing keys between apps
    public let accessGroup: String?

    /// Initialize with the unified keychain service name shared
    /// with `WalletStorage` (`org.dashfoundation.wallet`). All app
    /// key material — identity private keys, special masternode
    /// keys, per-wallet mnemonics — ends up under this single
    /// service so the keychain explorer and any cross-item queries
    /// see one namespace.
    ///
    /// `nonisolated` so the `shared` singleton — also `nonisolated` —
    /// can be constructed lazily from any isolation domain. The
    /// initializer only writes the two `let` fields; no actor-isolated
    /// state is touched.
    public nonisolated init() {
        self.serviceName = WalletStorage.keychainService
        self.accessGroup = nil
    }

    /// Initialize with custom service name and optional access group
    /// - Parameters:
    ///   - serviceName: The service name for keychain entries (e.g., "com.myapp.keys")
    ///   - accessGroup: Optional access group for sharing keys between apps
    public nonisolated init(serviceName: String, accessGroup: String? = nil) {
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
        // Delegate to the nonisolated implementation so both the
        // main-actor path and off-actor callers (e.g. Rust-side
        // persister callbacks) share identical Keychain semantics.
        return storePrivateKeyNonisolated(keyData, identityId: identityId, keyIndex: keyIndex)
    }

    /// Off-actor variant of [`storePrivateKey`].
    ///
    /// The underlying Security-framework APIs (`SecItemAdd`,
    /// `SecItemDelete`) are thread-safe, and this class's state
    /// (`serviceName` + `accessGroup`) is immutable (`let`), so the
    /// write can run from any isolation domain. This is the entry
    /// point the FFI persister callback (`persistIdentityKeys`
    /// in `PlatformWalletPersistenceHandler`) calls when Rust
    /// forwards a `Clear` private key — the callback runs on the
    /// Rust persister thread, not on the main actor, so the
    /// `@MainActor`-pinned `storePrivateKey` isn't reachable.
    ///
    /// Prefer the `@MainActor` wrapper when you're already in a
    /// main-actor context; call this directly from nonisolated
    /// contexts (background queues, detached tasks, C callbacks).
    @discardableResult
    public nonisolated func storePrivateKeyNonisolated(
        _ keyData: Data,
        identityId: Data,
        keyIndex: Int32
    ) -> String? {
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

    /// Delete every `privkey_<identityHex>_*` keychain row for `identityId`.
    public nonisolated func deleteAllPrivateKeys(for identityId: Data) throws {
        try deleteItems(accountPrefixes: ["privkey_\(identityId.toHexString())_"])
    }

    /// Delete every per-identity keychain row — both `privkey_*` and
    /// `specialkey_*` schemes — for `identityId`.
    public nonisolated func deleteAllKeychainItems(forIdentityId identityId: Data) throws {
        let identityHex = identityId.toHexString()
        try deleteItems(accountPrefixes: [
            "privkey_\(identityHex)_",
            "specialkey_\(identityHex)_"
        ])
    }

    private nonisolated func deleteItems(accountPrefixes: [String]) throws {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true
        ]

        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let searchStatus = SecItemCopyMatching(query as CFDictionary, &result)
        if searchStatus == errSecItemNotFound {
            return
        }
        guard searchStatus == errSecSuccess, let items = result as? [[String: Any]] else {
            throw KeychainError.retrieveFailed(searchStatus)
        }

        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                  accountPrefixes.contains(where: { account.hasPrefix($0) })
            else {
                continue
            }
            try deleteGenericPassword(account: account)
        }
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

    /// Retrieve data from the keychain by identifier.
    ///
    /// `nonisolated` so the FFI signer trampoline (which runs from
    /// any Tokio worker, off the main actor) can call it directly.
    /// `SecItemCopyMatching` is thread-safe and this type's state is
    /// `let` — same rationale as `storePrivateKeyNonisolated`.
    /// - Parameter identifier: The identifier for the stored data
    /// - Returns: The stored data, or nil if not found
    public nonisolated func retrieveKeyData(identifier: String) -> Data? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: identifier,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
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

    private nonisolated func deleteGenericPassword(account: String) throws {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: account
        ]

        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status)
        }
    }

    /// Nonisolated because the result only depends on the arguments
    /// — no access to actor-isolated state — and the function is
    /// shared between the `@MainActor` wrapper methods and the
    /// off-actor `storePrivateKeyNonisolated` path.
    private nonisolated func generateKeyIdentifier(identityId: Data, keyIndex: Int32) -> String {
        return "privkey_\(identityId.toHexString())_\(keyIndex)"
    }

    private nonisolated func generateSpecialKeyIdentifier(identityId: Data, keyType: SpecialKeyType) -> String {
        return "specialkey_\(identityId.toHexString())_\(keyType.rawValue)"
    }
}

// MARK: - Identity-key storage (derivation-path keyed)

extension KeychainManager {
    /// Compute the 20-byte RIPEMD160(SHA256(pubkey)) hash160 of the
    /// public-key bytes, returned as a lowercase hex string. Used to
    /// stamp `IdentityPrivateKeyMetadata.publicKeyHash` on identity
    /// private-key Keychain rows so the diagnostic explorer can show
    /// it without re-deriving.
    ///
    /// Backed by the Rust `platform_wallet_hash160` FFI helper (the
    /// only RIPEMD-160 implementation available without adding a new
    /// dependency on the iOS side — neither CryptoKit nor
    /// CommonCrypto exposes one).
    ///
    /// Returns `""` on empty input or FFI failure (matches the
    /// pre-change "no hash" sentinel — callers don't have to special-
    /// case the empty data path).
    public nonisolated static func computePublicKeyHashHex(_ publicKey: Data) -> String {
        guard !publicKey.isEmpty else { return "" }
        var out = [UInt8](repeating: 0, count: 20)
        let rc: Int32 = publicKey.withUnsafeBytes { raw -> Int32 in
            guard let base = raw.bindMemory(to: UInt8.self).baseAddress else { return -1 }
            return out.withUnsafeMutableBufferPointer { buf -> Int32 in
                guard let outBase = buf.baseAddress else { return -1 }
                return platform_wallet_hash160(base, UInt(publicKey.count), outBase)
            }
        }
        guard rc == 0 else { return "" }
        return Data(out).toHexString()
    }
}

/// Module-level alias so cross-module callers can reference the
/// metadata type without the (apparently brittle in some Xcode
/// toolchains) `KeychainManager.IdentityPrivateKeyMetadata` nested
/// path. Both names point at the same struct; pick whichever reads
/// better at the call site.
public typealias IdentityPrivateKeyMetadata = KeychainManager.IdentityPrivateKeyMetadata

extension KeychainManager {
    /// Metadata stamped into each identity-private-key keychain item
    /// as a JSON blob on `kSecAttrGeneric`. Everything here is
    /// non-secret — pub-key bytes + hash + DPP key descriptor +
    /// derivation breadcrumbs — so the diagnostic keychain explorer
    /// can pretty-print it.
    ///
    /// `keyType` / `purpose` / `securityLevel` use the same DPP
    /// `repr(u8)` discriminants every other FFI surface speaks
    /// (`IdentityPubkeyFFI`, `IdentityKeyEntryFFI`,
    /// `IdentityKeyRestoreFFI`):
    /// - `keyType`: 0 = ECDSA_SECP256K1, etc.
    /// - `purpose`: 0 = AUTHENTICATION, etc.
    /// - `securityLevel`: 0 = MASTER, 1 = CRITICAL, 2 = HIGH,
    ///   3 = MEDIUM.
    ///
    /// **Codable migration**: the three new descriptor fields plus
    /// `publicKeyHash` are all optional-decoded (default 0 / empty
    /// string) so existing rows written before this struct grew don't
    /// fail to decode — the explorer just shows the older shape.
    /// New writes always populate everything.
    public struct IdentityPrivateKeyMetadata: Codable, Sendable {
        public let identityId: String     // base58
        public let keyId: UInt32
        public let walletId: String       // hex
        public let identityIndex: UInt32
        public let keyIndex: UInt32
        public let derivationPath: String
        public let publicKey: String      // hex (compressed, 33 bytes typically)
        public let publicKeyHash: String  // hex (20 bytes, RIPEMD160(SHA256))
        public let keyType: UInt8         // DPP KeyType discriminant
        public let purpose: UInt8         // DPP Purpose discriminant
        public let securityLevel: UInt8   // DPP SecurityLevel discriminant

        public init(
            identityId: String,
            keyId: UInt32,
            walletId: String,
            identityIndex: UInt32,
            keyIndex: UInt32,
            derivationPath: String,
            publicKey: String,
            publicKeyHash: String,
            keyType: UInt8,
            purpose: UInt8,
            securityLevel: UInt8
        ) {
            self.identityId = identityId
            self.keyId = keyId
            self.walletId = walletId
            self.identityIndex = identityIndex
            self.keyIndex = keyIndex
            self.derivationPath = derivationPath
            self.publicKey = publicKey
            self.publicKeyHash = publicKeyHash
            self.keyType = keyType
            self.purpose = purpose
            self.securityLevel = securityLevel
        }

        // Backward-compatible decoding: rows written before the
        // descriptor + hash fields existed should still load (the
        // explorer just won't have those breadcrumbs to render).
        // Default `publicKeyHash` to "" and the three discriminants
        // to 0 (the canonical "first" enum value in each DPP enum,
        // which happens to be MASTER / AUTHENTICATION /
        // ECDSA_SECP256K1 — same as the convention used by the
        // registration path for the master key, so it's a sane "I
        // don't actually know" fallback).
        enum CodingKeys: String, CodingKey {
            case identityId, keyId, walletId, identityIndex, keyIndex,
                 derivationPath, publicKey, publicKeyHash,
                 keyType, purpose, securityLevel
        }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            self.identityId = try c.decode(String.self, forKey: .identityId)
            self.keyId = try c.decode(UInt32.self, forKey: .keyId)
            self.walletId = try c.decode(String.self, forKey: .walletId)
            self.identityIndex = try c.decode(UInt32.self, forKey: .identityIndex)
            self.keyIndex = try c.decode(UInt32.self, forKey: .keyIndex)
            self.derivationPath = try c.decode(String.self, forKey: .derivationPath)
            self.publicKey = try c.decode(String.self, forKey: .publicKey)
            self.publicKeyHash = try c.decodeIfPresent(String.self, forKey: .publicKeyHash) ?? ""
            self.keyType = try c.decodeIfPresent(UInt8.self, forKey: .keyType) ?? 0
            self.purpose = try c.decodeIfPresent(UInt8.self, forKey: .purpose) ?? 0
            self.securityLevel = try c.decodeIfPresent(UInt8.self, forKey: .securityLevel) ?? 0
        }
    }

    /// Store a 32-byte ECDSA private key derived from a wallet
    /// mnemonic at `derivationPath`. The keychain item's account is
    /// namespaced as `identity_privkey.<derivationPath>` so the
    /// explorer can categorise it under "Identity Private Keys"
    /// without scanning metadata, and the `metadata` encodes into
    /// `kSecAttrGeneric` as JSON so every ancillary field — identity
    /// id, key id, owning wallet id, pubkey + hash, indices — is
    /// discoverable from a single keychain read.
    ///
    /// Returns the stored account string (or `nil` on Security-
    /// framework failure). Idempotent per derivation path: an
    /// existing row is deleted + replaced.
    ///
    /// `nonisolated` because this is called from the Rust persister
    /// callback which runs off the main actor; the underlying
    /// SecItem APIs are thread-safe and this type's state is `let`.
    @discardableResult
    public nonisolated func storeIdentityPrivateKey(
        _ privateKey: Data,
        derivationPath: String,
        metadata: IdentityPrivateKeyMetadata
    ) -> String? {
        // Account name MUST be unique per (wallet, derivationPath).
        // The earlier scheme `"identity_privkey.\(derivationPath)"`
        // collided whenever two wallets had an identity at the same
        // `identity_index` — both wallets derive identity keys under
        // `m/9'/<coin>'/5'/0'/<idIdx>'/<purpose>'/<keyId>'`, so the
        // path alone isn't unique across wallets. The collision
        // caused later writes to overwrite earlier ones in Keychain,
        // leaving prior `PersistentPublicKey` rows pointing at an
        // account that now holds another wallet's private bytes —
        // the signing trampoline would happily return them and
        // produce signatures that wouldn't verify.
        //
        // Including the wallet id (hex) in the account name keeps
        // every wallet's identity-key set independent. Reads via
        // `retrieveIdentityPrivateKey(publicKeyHex:)` still work
        // unmodified — that path scans every `identity_privkey.*`
        // item and matches on the metadata's `publicKey` hex, so the
        // account-name shape doesn't matter for lookup. The direct
        // `retrieveKeyData(identifier:)` path uses whatever string
        // we return here, which the persister stores on the
        // `PersistentPublicKey.privateKeyKeychainIdentifier` column;
        // on the next persister upsert the row gets the new account
        // string, while sessions in between fall back to the
        // metadata-scan path automatically.
        let account = "identity_privkey.\(metadata.walletId).\(derivationPath)"

        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: account,
            kSecValueData as String: privateKey,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            kSecAttrSynchronizable as String: false,
            kSecAttrLabel as String: metadata.publicKey.lowercased(),
        ]

        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        // Encode metadata as JSON. `kSecAttrGeneric` accepts
        // arbitrary bytes; the keychain explorer already pretty-
        // prints JSON payloads on the detail view.
        if let metadataData = try? JSONEncoder().encode(metadata) {
            query[kSecAttrGeneric as String] = metadataData
        }

        // Delete-then-add pattern lets the row settle into the new
        // `kSecValueData` + `kSecAttrGeneric` without fighting
        // SecItemUpdate's attribute-preservation quirks.
        //
        // The DELETE query must be ATTRIBUTE-ONLY: Apple Keychain
        // treats `kSecValueData` and `kSecAttrGeneric` (the value
        // payloads) as *value filters* on delete queries — including
        // them would only delete rows whose data ALREADY equals the
        // new bytes, which by definition is never true on an update.
        // Without this fix, every prior failed attempt left a stale
        // row that we could neither delete (data mismatch) nor add
        // (uniqueness conflict on service+account → errSecDuplicateItem).
        var deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: account,
        ]
        if let accessGroup = accessGroup {
            deleteQuery[kSecAttrAccessGroup as String] = accessGroup
        }
        SecItemDelete(deleteQuery as CFDictionary)

        let status = SecItemAdd(query as CFDictionary, nil)
        return status == errSecSuccess ? account : nil
    }

    /// Look up a stored identity private-key row by its public-key
    /// bytes (hex-encoded — matches the `publicKey` field of
    /// [`IdentityPrivateKeyMetadata`]).
    ///
    /// This is the lookup the FFI signer trampoline
    /// (`KeychainSigner`) uses when no `PersistentPublicKey` row is
    /// available yet — for example mid-identity-registration, where
    /// the keys have been pre-derived + persisted to the Keychain by
    /// Swift but the SwiftData row is only inserted by the Rust
    /// persister callback once registration completes.
    ///
    /// Scans every `identity_privkey.*` keychain item, parses the
    /// metadata blob on `kSecAttrGeneric`, and returns the row whose
    /// `publicKey` matches `publicKeyHex`. `nil` if nothing matches.
    ///
    /// `nonisolated` because Security-framework calls are thread-safe
    /// and this type's state is `let` — safe to call from the FFI
    /// trampoline on any Tokio worker thread.
    public nonisolated func retrieveIdentityPrivateKey(publicKeyHex: String) -> Data? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrLabel as String: publicKeyHex.lowercased(),
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnData as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess else { return nil }
        return result as? Data
    }

    /// Existence check for the wallet-derived identity-private-key
    /// scheme (`identity_privkey.<derivationPath>` items keyed by
    /// public-key hex in metadata). Mirrors
    /// `retrieveIdentityPrivateKey(publicKeyHex:)` but skips
    /// `kSecReturnData` so the secret bytes never enter Swift
    /// memory — only the metadata blob is loaded and matched.
    /// Useful for UI diagnostics ("do we hold the private bytes
    /// for this public key?") that should not surface the key
    /// material itself.
    public nonisolated func hasIdentityPrivateKey(publicKeyHex: String) -> Bool {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            return false
        }

        let decoder = JSONDecoder()
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                account.hasPrefix("identity_privkey.")
            else {
                continue
            }
            guard let metadataData = item[kSecAttrGeneric as String] as? Data,
                let metadata = try? decoder.decode(IdentityPrivateKeyMetadata.self, from: metadataData)
            else {
                continue
            }
            if metadata.publicKey.caseInsensitiveCompare(publicKeyHex) == .orderedSame {
                return true
            }
        }
        return false
    }

    /// Return the keychain account string of the identity-private-key
    /// item whose metadata `publicKey` matches `publicKeyHex`, or `nil`
    /// if none is stored. Like `hasIdentityPrivateKey(publicKeyHex:)`
    /// this skips `kSecReturnData`, so the secret bytes never enter Swift
    /// memory — only attributes are read. The returned account is the
    /// identifier `retrieveKeyData(identifier:)` consumes, so a caller
    /// can adopt it onto `PersistentPublicKey.privateKeyKeychainIdentifier`
    /// when a key was materialized by another path (e.g. registration's
    /// own keychain write) without re-deriving anything.
    public nonisolated func identityPrivateKeyAccount(publicKeyHex: String) -> String? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            return nil
        }

        let decoder = JSONDecoder()
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                account.hasPrefix("identity_privkey.")
            else {
                continue
            }
            guard let metadataData = item[kSecAttrGeneric as String] as? Data,
                let metadata = try? decoder.decode(IdentityPrivateKeyMetadata.self, from: metadataData)
            else {
                continue
            }
            if metadata.publicKey.caseInsensitiveCompare(publicKeyHex) == .orderedSame {
                return account
            }
        }
        return nil
    }

    /// Decode the metadata blob (`kSecAttrGeneric`) of every
    /// `identity_privkey.*` keychain item — no secret bytes loaded. The
    /// breadcrumb backfill uses this to populate
    /// `PersistentPublicKey.{walletId, identityDerivationPath}` for keys
    /// materialized before those columns existed; sourcing it from the
    /// Keychain (not SwiftData) means it heals even if the SwiftData store
    /// was rebuilt.
    public nonisolated func allIdentityPrivateKeyMetadata() -> [IdentityPrivateKeyMetadata] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            return []
        }

        let decoder = JSONDecoder()
        var out: [IdentityPrivateKeyMetadata] = []
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                account.hasPrefix("identity_privkey.")
            else {
                continue
            }
            guard let metadataData = item[kSecAttrGeneric as String] as? Data,
                let metadata = try? decoder.decode(
                    IdentityPrivateKeyMetadata.self, from: metadataData)
            else {
                continue
            }
            out.append(metadata)
        }
        return out
    }

    /// Delete the identity private-key row for the
    /// `(walletId, derivationPath)` pair — symmetric with
    /// `storeIdentityPrivateKey` (which writes under the
    /// `identity_privkey.<walletId>.<path>` account scheme).
    ///
    /// Idempotent; returns true on success or "not found". A
    /// guarded sweep of the legacy (`identity_privkey.<path>` —
    /// no walletId) account is also included so callers don't end
    /// up with a half-migrated state where the new-format row is
    /// gone but a legacy row at the same path lingers.
    ///
    /// SAFETY: the legacy account format collided across wallets
    /// (the very bug we are fixing by namespacing the new path),
    /// so deleting a `identity_privkey.<path>` row unconditionally
    /// could wipe a DIFFERENT wallet's secret if the two wallets
    /// happened to derive an identity at the same DIP-9 path. The
    /// sweep therefore looks up the legacy row, decodes its
    /// metadata blob, and only deletes when
    /// `metadata.walletId == walletIdHex`. Pathological in
    /// practice but cheap to defend against.
    @discardableResult
    public nonisolated func deleteIdentityPrivateKey(
        walletId: Data,
        derivationPath: String
    ) -> Bool {
        let walletIdHex = walletId.toHexString()
        let newAccount = "identity_privkey.\(walletIdHex).\(derivationPath)"
        var newQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: newAccount,
        ]
        if let accessGroup = accessGroup {
            newQuery[kSecAttrAccessGroup as String] = accessGroup
        }
        let newStatus = SecItemDelete(newQuery as CFDictionary)
        var ok = newStatus == errSecSuccess || newStatus == errSecItemNotFound

        if !deleteLegacyKeychainEntryIfOwnedByWallet(
            walletIdHex: walletIdHex,
            derivationPath: derivationPath
        ) {
            // Legacy sweep had a non-recoverable error (something
            // other than "not found" / "different wallet's row").
            // The new-format delete already happened, so the
            // caller's primary intent landed — surface the legacy
            // failure via the return value so anyone treating it
            // as a strict success can react.
            ok = false
        }
        return ok
    }

    /// Forget every materialized copy of ONE identity key across both
    /// storage schemes, independent of SwiftData breadcrumbs:
    /// - the legacy `(identityId, keyIndex)` item;
    /// - the wallet-derived `identity_privkey.*` item — via the
    ///   caller's breadcrumb when it has one, and ALWAYS via a
    ///   metadata public-key sweep too, because pre-migration rows
    ///   carry `nil` breadcrumb columns until the backfill runs while
    ///   their secret is still revealable by public-key lookup.
    ///
    /// Returns `true` only when nothing remains retrievable under
    /// either scheme afterward (verified with `hasIdentityPrivateKey`
    /// + `hasPrivateKey`) — a caller must NOT clear its stored key
    /// reference on `false`, or the secret would outlive every handle
    /// to it. Note this forgets the MATERIALIZED secret; a
    /// wallet-derivable key can still be re-derived from the seed on
    /// demand.
    @discardableResult
    public func forgetIdentityKeyMaterial(
        identityId: Data,
        keyIndex: Int32,
        publicKeyHex: String,
        breadcrumbWalletId: Data? = nil,
        derivationPath: String? = nil
    ) -> Bool {
        // An unresolved public key would make the derived-scheme
        // sweep a vacuous no-op that still "verifies" — refuse
        // instead so the caller keeps its reference.
        guard !publicKeyHex.isEmpty else { return false }
        var ok = deletePrivateKey(identityId: identityId, keyIndex: keyIndex)
        if let breadcrumbWalletId, let derivationPath {
            ok = deleteIdentityPrivateKey(
                walletId: breadcrumbWalletId,
                derivationPath: derivationPath
            ) && ok
        }
        // Sweep every derived-scheme row EITHER selector can reach —
        // metadata public-key match (the account lookup's selector)
        // or kSecAttrLabel match (retrieval's selector, which never
        // decodes metadata) — with STRICT error handling: a lookup
        // that fails for any reason other than not-found (Keychain
        // locked, inaccessible, misconfigured) fails the forget.
        // "Couldn't look" must never read as "absent". Bounded
        // re-enumeration: each pass deletes everything it saw, so the
        // next pass must come back empty; the bound is defensive.
        var passes = 0
        while true {
            guard let accounts = identityPrivateKeyAccountsForForget(
                matching: publicKeyHex
            ) else { return false }
            if accounts.isEmpty { break }
            passes += 1
            if passes > 4 { return false }
            for account in accounts {
                do {
                    try deleteGenericPassword(account: account)
                } catch {
                    return false
                }
            }
        }
        // Legacy scheme verified by a strict exact-account probe —
        // only a confirmed not-found counts as absent.
        guard legacyPrivateKeyConfirmedAbsent(
            identityId: identityId,
            keyIndex: keyIndex
        ) == true else { return false }
        return ok
    }

    /// Every service item reachable by EITHER derived-scheme selector
    /// for `publicKeyHex`: a decoded-metadata `publicKey` match (the
    /// account lookup's selector) or a `kSecAttrLabel` match
    /// (retrieval's selector — served even when the metadata blob is
    /// missing or corrupt). Attributes-only; the secret bytes never
    /// enter Swift memory. Returns `[]` when the Keychain confirms
    /// nothing matches, and `nil` on any other error so callers fail
    /// closed instead of mistaking "couldn't look" for "absent".
    private nonisolated func identityPrivateKeyAccountsForForget(
        matching publicKeyHex: String
    ) -> [String]? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return [] }
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            return nil
        }
        let target = publicKeyHex.lowercased()
        let decoder = JSONDecoder()
        var accounts: [String] = []
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String else {
                continue
            }
            let labelMatches =
                (item[kSecAttrLabel as String] as? String)?.lowercased() == target
            var metadataMatches = false
            if account.hasPrefix("identity_privkey."),
               let blob = item[kSecAttrGeneric as String] as? Data,
               let metadata = try? decoder.decode(
                   IdentityPrivateKeyMetadata.self, from: blob
               ),
               metadata.publicKey.caseInsensitiveCompare(publicKeyHex) == .orderedSame {
                metadataMatches = true
            }
            if labelMatches || metadataMatches {
                accounts.append(account)
            }
        }
        return accounts
    }

    /// Strict tri-state presence probe for one identity key's
    /// materialized scalar across BOTH storage schemes: `true` = at
    /// least one item confirmed present, `false` = both schemes
    /// confirmed absent, `nil` = a lookup failed (locked/inaccessible
    /// Keychain) — callers must preserve their current state on
    /// `nil`, never conclude. Attributes-only.
    public nonisolated func identityScalarConfirmedPresent(
        identityId: Data,
        keyIndex: Int32,
        publicKeyHex: String?
    ) -> Bool? {
        guard let legacyAbsent = legacyPrivateKeyConfirmedAbsent(
            identityId: identityId, keyIndex: keyIndex
        ) else { return nil }
        if !legacyAbsent { return true }
        guard let publicKeyHex, !publicKeyHex.isEmpty else { return false }
        guard let derived = identityPrivateKeyAccountsForForget(
            matching: publicKeyHex
        ) else { return nil }
        return derived.isEmpty ? false : true
    }

    /// Strict tri-state presence probe for a voting/owner/payout
    /// special key. Same contract as
    /// `identityScalarConfirmedPresent`: `nil` means "couldn't look".
    public nonisolated func specialKeyConfirmedPresent(
        identityId: Data,
        keyType: SpecialKeyType
    ) -> Bool? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: generateSpecialKeyIdentifier(
                identityId: identityId, keyType: keyType
            ),
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecSuccess { return true }
        if status == errSecItemNotFound { return false }
        return nil
    }

    /// Strict absence probe for the legacy `(identityId, keyIndex)`
    /// item: `true` = the Keychain confirmed not-found, `false` = the
    /// item exists, `nil` = the lookup itself failed (locked or
    /// inaccessible Keychain) — callers must treat `nil` as "cannot
    /// verify", never as absence.
    private nonisolated func legacyPrivateKeyConfirmedAbsent(
        identityId: Data,
        keyIndex: Int32
    ) -> Bool? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: generateKeyIdentifier(
                identityId: identityId, keyIndex: keyIndex
            ),
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return true }
        if status == errSecSuccess { return false }
        return nil
    }

    /// Best-effort delete of the legacy (no-walletId) keychain
    /// entry for `derivationPath`, gated on its decoded
    /// `IdentityPrivateKeyMetadata.walletId` matching `walletIdHex`.
    /// No-op when the row doesn't exist or belongs to a different
    /// wallet. Returns `false` only on a genuine Keychain error
    /// (NOT for "not found" or "different wallet's data") so the
    /// caller can distinguish "nothing to do" from "tried + failed".
    ///
    /// Shared by both `deleteIdentityPrivateKey` (per-key delete)
    /// and `WalletKeyHealthSheet.cleanupLegacyKeychainEntry`
    /// (post-rederive cleanup) so both call sites apply the same
    /// safety guard.
    public nonisolated func deleteLegacyKeychainEntryIfOwnedByWallet(
        walletIdHex: String,
        derivationPath: String
    ) -> Bool {
        let legacyAccount = "identity_privkey.\(derivationPath)"
        var lookupQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: legacyAccount,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        if let accessGroup = accessGroup {
            lookupQuery[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let lookupStatus = SecItemCopyMatching(lookupQuery as CFDictionary, &result)
        if lookupStatus == errSecItemNotFound {
            return true
        }
        guard lookupStatus == errSecSuccess, let attrs = result as? [String: Any] else {
            return false
        }
        // Row exists but metadata is malformed / missing. Be
        // conservative: refuse to delete data we can't prove we
        // own. Surface via `os_log` so the per-key legacy path
        // is symmetric with the wholesale sweeps
        // (`deleteAllIdentityPrivateKeys(forIdentityIdBase58:)`
        // and `forWalletId:`) — all three "skip, can't verify
        // ownership" branches land in Console.app uniformly.
        // Treat as success: we successfully decided not to delete.
        guard let metadataData = attrs[kSecAttrGeneric as String] as? Data else {
            Self.log.error(
                "Skipping legacy identity_privkey row at account \(legacyAccount, privacy: .public) — missing kSecAttrGeneric metadata blob; private bytes remain"
            )
            return true
        }
        let metadata: IdentityPrivateKeyMetadata
        do {
            metadata = try JSONDecoder().decode(
                IdentityPrivateKeyMetadata.self,
                from: metadataData
            )
        } catch {
            Self.log.error(
                "Skipping legacy identity_privkey row at account \(legacyAccount, privacy: .public) — metadata JSON decode failed (\(String(describing: error), privacy: .public)); private bytes remain"
            )
            return true
        }
        guard metadata.walletId.caseInsensitiveCompare(walletIdHex) == .orderedSame else {
            return true
        }

        var deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: legacyAccount,
        ]
        if let accessGroup = accessGroup {
            deleteQuery[kSecAttrAccessGroup as String] = accessGroup
        }
        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        return deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound
    }

    /// Delete every `identity_privkey.*` keychain row whose
    /// `IdentityPrivateKeyMetadata.identityId` matches
    /// `identityIdBase58` — the base58 identity id Swift uses
    /// throughout the persistence layer. Used by the key-health
    /// sheet's "Delete orphan identity" action so cascading the
    /// SwiftData row doesn't leave its keys' private bytes behind
    /// in Keychain.
    ///
    /// Scans the metadata blob (`kSecAttrGeneric`) on every
    /// matching keychain item — handles both new-format
    /// (`identity_privkey.<walletId>.<path>`) and legacy-format
    /// (`identity_privkey.<path>`) accounts uniformly. Idempotent;
    /// no-op when nothing matches.
    public nonisolated func deleteAllIdentityPrivateKeys(forIdentityIdBase58 identityIdBase58: String) throws {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return
        }
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            throw KeychainError.retrieveFailed(status)
        }

        let decoder = JSONDecoder()
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                  account.hasPrefix("identity_privkey.")
            else {
                continue
            }
            // Surface undecodable-metadata rows via os_log rather
            // than silently skipping them. The caller has asked
            // for a wholesale identity-scoped wipe, so the safest
            // action is still to NOT delete a row we can't prove
            // we own — but the developer needs to know that an
            // `identity_privkey.*` row exists whose metadata we
            // can't read (likely a partial / aborted write, a
            // legacy pre-metadata row, or a future schema rename),
            // because the user is being told the orphan was
            // cleaned up. Visible in Console.app under the
            // `dashpay.SwiftDashSDK` subsystem.
            guard let metadataData = item[kSecAttrGeneric as String] as? Data else {
                Self.log.error(
                    "Skipping identity_privkey row at account \(account, privacy: .public) — missing kSecAttrGeneric metadata blob; private bytes remain"
                )
                continue
            }
            let metadata: IdentityPrivateKeyMetadata
            do {
                metadata = try decoder.decode(IdentityPrivateKeyMetadata.self, from: metadataData)
            } catch {
                Self.log.error(
                    "Skipping identity_privkey row at account \(account, privacy: .public) — metadata JSON decode failed (\(String(describing: error), privacy: .public)); private bytes remain"
                )
                continue
            }
            guard metadata.identityId == identityIdBase58 else {
                continue
            }
            try deleteGenericPassword(account: account)
        }
    }

    /// Subsystem-tagged logger for Keychain operations. Surfaces
    /// silently-skipped rows from the wholesale sweeps so the
    /// developer/user is aware that some `identity_privkey.*`
    /// rows were not touched despite the cleanup running. Filter
    /// in Console.app with `subsystem:dashpay.SwiftDashSDK
    /// category:Keychain`.
    ///
    /// `nonisolated` so the keychain helpers (which are
    /// `nonisolated` themselves — they run on the FFI signer
    /// trampoline's worker thread) can reach the logger without
    /// hopping to the main actor. `Logger` is already `Sendable`,
    /// so plain `nonisolated` (not `nonisolated(unsafe)`) is the
    /// right escape hatch.
    fileprivate nonisolated static let log = Logger(subsystem: "dashpay.SwiftDashSDK", category: "Keychain")

    /// Delete every `identity_privkey.<derivationPath>` keychain row whose
    /// `IdentityPrivateKeyMetadata.walletId` matches `walletId`.
    public nonisolated func deleteAllIdentityPrivateKeys(forWalletId walletId: Data) throws {
        let walletIdHex = walletId.toHexString()
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        if let accessGroup = accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return
        }
        guard status == errSecSuccess, let items = result as? [[String: Any]] else {
            throw KeychainError.retrieveFailed(status)
        }

        let decoder = JSONDecoder()
        for item in items {
            guard let account = item[kSecAttrAccount as String] as? String,
                  account.hasPrefix("identity_privkey.")
            else {
                continue
            }
            // Same logging-on-skip rationale as
            // `deleteAllIdentityPrivateKeys(forIdentityIdBase58:)`
            // above — see that helper's doc comment for why we
            // refuse to delete rows we can't prove we own.
            guard let metadataData = item[kSecAttrGeneric as String] as? Data else {
                Self.log.error(
                    "Skipping identity_privkey row at account \(account, privacy: .public) — missing kSecAttrGeneric metadata blob; private bytes remain"
                )
                continue
            }
            let metadata: IdentityPrivateKeyMetadata
            do {
                metadata = try decoder.decode(IdentityPrivateKeyMetadata.self, from: metadataData)
            } catch {
                Self.log.error(
                    "Skipping identity_privkey row at account \(account, privacy: .public) — metadata JSON decode failed (\(String(describing: error), privacy: .public)); private bytes remain"
                )
                continue
            }
            guard metadata.walletId.caseInsensitiveCompare(walletIdHex) == .orderedSame else {
                continue
            }

            try deleteGenericPassword(account: account)
        }
    }
}

// MARK: - Platform-address private-key storage — REMOVED
//
// Platform-address private keys are NOT persisted. They are pure
// derivation outputs of `(mnemonic, derivation-path)` and exist only
// for the duration of a single signing call. The FFI signer
// trampoline (see `KeychainSigner.swift`'s `key_type == 0xFF` branch)
// pulls the mnemonic from Keychain via `WalletStorage.retrieveMnemonic`
// and the derivation path from `PersistentPlatformAddress`, then
// hands both to `dash_sdk_sign_with_mnemonic_and_path` which derives,
// signs, and zeroes the buffers before returning.
//
// No `platform_address_privkey.*` Keychain account convention exists
// any more.
