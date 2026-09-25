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
/// * A non-secret per-wallet presence marker at
///   `wallet.present.<64-char-hex-walletId>` under the sibling
///   service `<keychainService>.presence`, readable after the first
///   unlock so a locked device can still answer "does a wallet
///   exist?" — see [`walletPresence()`](x-source-tag://walletPresence).
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

    /// Keychain service for the per-wallet presence markers:
    /// `<keychainService>.presence`.
    ///
    /// A sibling service rather than `keychainService` itself on purpose.
    /// The markers are `AfterFirstUnlock` while every other per-wallet item
    /// is `WhenUnlocked`, and a service-wide `SecItemCopyMatching` over a
    /// mix of the two behaves badly on a locked device whichever way
    /// securityd resolves it: fail the whole query and the marker is
    /// unreadable exactly when it is needed; drop the locked rows and
    /// `listWalletIdsWithMnemonic()` would return an empty inventory
    /// instead of an error. Keeping each service homogeneous keeps both
    /// answers honest. Derived from `keychainService` so the
    /// `DASH_KEYCHAIN_SERVICE` override applies to both.
    public static let presenceKeychainService = "\(keychainService).presence"
    /// Base account string used to build per-wallet presence-marker
    /// accounts via `perWalletPresenceMarkerAccount(for:)`.
    public static let presenceMarkerAccountPrefix = "wallet.present"

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
    ///
    /// Also maintains the wallet's presence marker (see `walletPresence()`)
    /// under the invariant *marker ⇒ a mnemonic was stored*. The steps run
    /// in the one order that keeps it across an interruption at any
    /// point: the marker comes off before the old mnemonic is deleted,
    /// and goes back on only after the new mnemonic is in place. A
    /// process killed in between leaves at most a mnemonic without a
    /// marker — the one direction of drift the presence read repairs —
    /// never a marker without a mnemonic. For the same reason a failed
    /// trailing marker write is not an error here: the mnemonic *is*
    /// stored, and the next `walletPresence()` call on an unlocked
    /// device backfills the marker.
    public func storeMnemonic(_ mnemonic: String, for walletId: Data) throws {
        Self.mutationLock.lock()
        defer { Self.mutationLock.unlock() }

        try deletePresenceMarker(for: walletId)
        try deleteMnemonicItem(for: walletId)
        try addMnemonicItem(Data(mnemonic.utf8), for: walletId)
        try? storePresenceMarker(for: walletId)
    }

    /// Raw delete of the mnemonic item. Idempotent. Overridable so tests
    /// can observe the order of the keychain steps without a keychain.
    func deleteMnemonicItem(for walletId: Data) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: perWalletMnemonicAccount(for: walletId)
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Raw add of the mnemonic item; the caller has deleted any previous
    /// one so the `kSecAttrAccessible` value is rewritten on every save.
    func addMnemonicItem(_ data: Data, for walletId: Data) throws {
        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: perWalletMnemonicAccount(for: walletId),
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

    /// Three-way answer to "can this wallet's mnemonic be read right now?".
    ///
    /// [`hasMnemonic(for:)`] collapses the last two cases into `false`, which
    /// is fine where the question is "is this watch-only?" but wrong wherever
    /// the caller has to decide between giving up and trying again later.
    public enum MnemonicAvailability: Sendable, Equatable {
        /// The item exists and its attributes were readable.
        case present
        /// The Keychain answered definitively that there is no such item —
        /// a genuine watch-only wallet.
        case absent
        /// The lookup failed for another reason: the device is locked, access
        /// was denied, the daemon was unavailable. Says nothing about whether
        /// a mnemonic exists, so callers should retry rather than conclude.
        case unavailable(OSStatus)
    }

    /// Whether the wallet's mnemonic is readable, keeping "no such item" apart
    /// from "could not tell". Attribute-only; no secret is materialized.
    public func mnemonicAvailability(for walletId: Data) -> MnemonicAvailability {
        let account = perWalletMnemonicAccount(for: walletId)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: account,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: true
        ]
        var result: AnyObject?
        switch SecItemCopyMatching(query as CFDictionary, &result) {
        case errSecSuccess: return .present
        case errSecItemNotFound: return .absent
        case let status: return .unavailable(status)
        }
    }

    /// Cheap existence check used by signer preflight paths.
    ///
    /// Unlike `retrieveMnemonic(...)`, this does not materialize the
    /// mnemonic bytes into Swift heap objects.
    ///
    /// Answers `false` both for "no such item" and for "could not tell",
    /// which is what a preflight wants. A caller that has to choose between
    /// giving up and retrying needs `mnemonicAvailability(for:)` instead.
    public func hasMnemonic(for walletId: Data) -> Bool {
        mnemonicAvailability(for: walletId) == .present
    }

    /// Attribute-only identity stamp of the wallet's mnemonic Keychain
    /// item, or `nil` when no item exists (or attributes are unreadable).
    ///
    /// Built from the item's creation + modification dates, which change
    /// whenever the item is written: `storeMnemonic` is delete-then-add
    /// (fresh dates every call) and any in-place `SecItemUpdate` bumps
    /// the modification date. The seed-binding verification cache binds
    /// its marker to this stamp so a rewritten or re-created mnemonic
    /// item invalidates the cached verification and forces a full
    /// re-verify — without this, a marker verified against an older
    /// mnemonic would keep passing after the item changed.
    ///
    /// Like `hasMnemonic`, this queries attributes only — the secret is
    /// never materialized.
    public func mnemonicKeychainStamp(for walletId: Data) -> String? {
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
        guard status == errSecSuccess,
              let attrs = result as? [String: Any],
              let modified = attrs[kSecAttrModificationDate as String] as? Date else {
            return nil
        }
        let created = attrs[kSecAttrCreationDate as String] as? Date ?? modified
        // Millisecond precision; both dates so delete-then-add and in-place
        // update are each guaranteed to change the stamp.
        return "c\(Int64(created.timeIntervalSince1970 * 1000))"
            + "-m\(Int64(modified.timeIntervalSince1970 * 1000))"
    }

    /// Delete a mnemonic keyed by wallet id. Idempotent.
    ///
    /// The mnemonic goes first and the presence marker second, under the
    /// same lock `walletPresence()` takes: a presence read cannot slip
    /// between the two steps and backfill a marker for a mnemonic that is
    /// about to disappear, and an interruption between them leaves a
    /// marker whose mnemonic is gone only until the next unlocked
    /// presence read reconciles it away.
    public func deleteMnemonic(for walletId: Data) throws {
        Self.mutationLock.lock()
        defer { Self.mutationLock.unlock() }

        try deleteMnemonicItem(for: walletId)
        try deletePresenceMarker(for: walletId)
    }

    /// Enumerate all wallet ids with a stored mnemonic.
    ///
    /// Reads every `kSecClassGenericPassword` entry under
    /// `keychainService`, keeps those whose account starts with
    /// `wallet.mnemonic.` followed by a 64-character lowercase hex
    /// string, and decodes the suffix back into 32-byte wallet ids.
    /// Returns an empty array if the keychain has none. Throws while the
    /// device is locked (the items are `WhenUnlocked`); see
    /// `walletPresence()` for the question that has to be answered then.
    public func listWalletIdsWithMnemonic() throws -> [Data] {
        try listWalletIds(service: keychainService, accountPrefix: mnemonicKeychainAccount)
    }

    /// Wallet ids encoded in the accounts of every generic-password item
    /// under `service` whose account is `<accountPrefix>.<64-hex>`.
    /// `errSecItemNotFound` is an empty list; any other failure is thrown
    /// as `keychainError` so "nothing there" and "could not look" stay
    /// distinguishable.
    private func listWalletIds(service: String, accountPrefix: String) throws -> [Data] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
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

        let prefix = "\(accountPrefix)."
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

    // MARK: - Wallet Presence Marker
    //
    // The mnemonic and metadata items are `WhenUnlockedThisDeviceOnly`, so
    // a process launched while the device is locked (a background app
    // refresh, a silent push) cannot even enumerate them: the inventory
    // read fails and a naive caller concludes "no wallet" and shows the
    // setup screen. The marker is the durable, non-secret answer to that
    // one question.
    //
    // Invariant: marker ⇒ a mnemonic was stored for that id. The marker
    // is maintained only inside `storeMnemonic` / `deleteMnemonic`, in an
    // order that keeps the invariant across an interruption at any step
    // (mnemonic on before marker on; mnemonic off before marker off), and
    // the three entry points that touch both items share one process-wide
    // lock so a presence read cannot interleave with a write or delete
    // (`PlatformWalletManager.deleteWallet` uses its own `WalletStorage`
    // instance; the lock is static for that reason). Drift is therefore
    // only ever a mnemonic *without* a marker — a wallet stored before the
    // marker existed, a trailing marker write that failed — and every
    // presence read made while the inventory is readable repairs it, in
    // both directions: it rewrites the marker set to match the inventory.
    //
    // Security trade-off: `AfterFirstUnlockThisDeviceOnly` means anyone
    // who can query this app's keychain after the first unlock since
    // boot learns that a wallet exists here, and its 32-byte wallet id.
    // The id is a hash, not key material; it identifies nothing outside
    // this app and unlocks nothing. Like the mnemonic it stays in the
    // keychain across a reinstall (which is why this is not a
    // `UserDefaults` flag) and never syncs to iCloud.

    /// Serialises `storeMnemonic`, `deleteMnemonic` and `walletPresence()`
    /// across every `WalletStorage` instance in the process. Held only
    /// around keychain calls; never taken by the item-level helpers.
    private static let mutationLock = NSLock()

    private func perWalletPresenceMarkerAccount(for walletId: Data) -> String {
        let hex = walletId.map { String(format: "%02x", $0) }.joined()
        return "\(Self.presenceMarkerAccountPrefix).\(hex)"
    }

    /// Write (or rewrite) the presence marker for `walletId`. A lifecycle
    /// step of `storeMnemonic` and of the reconciliation in
    /// `walletPresence()`, not an SDK operation: a marker written on its
    /// own would assert a wallet that may not exist. Delete-then-add like
    /// the other writers so the accessibility class is re-applied on
    /// every write. The payload is the wallet id itself — no secret.
    func storePresenceMarker(for walletId: Data) throws {
        let account = perWalletPresenceMarkerAccount(for: walletId)

        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.presenceKeychainService,
            kSecAttrAccount as String: account
        ]
        let deleteStatus = SecItemDelete(deleteQuery as CFDictionary)
        guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
            throw WalletStorageError.keychainError(deleteStatus)
        }

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.presenceKeychainService,
            kSecAttrAccount as String: account,
            kSecValueData as String: walletId,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Delete the presence marker for `walletId`. Idempotent. A lifecycle
    /// step of `storeMnemonic`, `deleteMnemonic` and the reconciliation in
    /// `walletPresence()`; not an SDK operation.
    func deletePresenceMarker(for walletId: Data) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.presenceKeychainService,
            kSecAttrAccount as String: perWalletPresenceMarkerAccount(for: walletId)
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw WalletStorageError.keychainError(status)
        }
    }

    /// Wallet ids that carry a presence marker. Readable once the device
    /// has been unlocked since boot, locked or not. Empty means "no
    /// marker", which is *not* "no wallet" — see `walletPresence()`.
    /// Throws `keychainError` before the first unlock or on any other
    /// lookup failure.
    public func markedWalletIds() throws -> [Data] {
        try listWalletIds(
            service: Self.presenceKeychainService,
            accountPrefix: Self.presenceMarkerAccountPrefix
        )
    }

    /// Three-way answer to "does this app hold at least one wallet?".
    public enum WalletPresence: Sendable, Equatable {
        /// The mnemonic inventory was readable and non-empty, or it was
        /// unreadable and at least one wallet id is marked.
        case present
        /// The mnemonic inventory was readable and empty. The only verdict
        /// on which a setup screen may be shown.
        case absent
        /// Could not tell: the marker set was unreadable (before the first
        /// unlock, `errSecInteractionNotAllowed`, …) or it was empty and
        /// the inventory was unreadable (a locked device with wallets
        /// stored before the marker existed). Says nothing about whether
        /// a wallet exists; decide later, not now.
        case unknown(OSStatus)
    }

    /// Whether at least one wallet exists, answerable on a locked device.
    ///
    /// The mnemonic inventory is the source of truth whenever it can be
    /// read; the marker set stands in for it only while it cannot:
    ///
    /// 1. `markedWalletIds()` unreadable → `.unknown(status)`.
    /// 2. `listWalletIdsWithMnemonic()` readable → verdict from it
    ///    (`.present` / `.absent`), and the marker set is reconciled to
    ///    it: a marker is written for every inventoried id without one
    ///    (wallets stored before the marker existed backfill themselves
    ///    on the first unlocked read) and any marker without a mnemonic
    ///    is removed. Best effort; a failed write or delete changes
    ///    nothing about the verdict.
    /// 3. Inventory unreadable → marker set non-empty is `.present`,
    ///    empty is `.unknown(status)`.
    ///
    /// Invariant for callers: marker present ⇒ a mnemonic was stored for
    /// that id; a missing marker on its own never means "no wallet" — only
    /// a readable, empty inventory does.
    ///
    /// - Tag: walletPresence
    public func walletPresence() -> WalletPresence {
        Self.mutationLock.lock()
        defer { Self.mutationLock.unlock() }

        let marked: [Data]
        do {
            marked = try markedWalletIds()
        } catch {
            return .unknown(Self.status(of: error))
        }

        let inventory: [Data]
        do {
            inventory = try listWalletIdsWithMnemonic()
        } catch {
            return marked.isEmpty ? .unknown(Self.status(of: error)) : .present
        }

        let markedSet = Set(marked)
        let inventorySet = Set(inventory)
        for walletId in inventory where !markedSet.contains(walletId) {
            try? storePresenceMarker(for: walletId)
        }
        for walletId in marked where !inventorySet.contains(walletId) {
            try? deletePresenceMarker(for: walletId)
        }
        return inventory.isEmpty ? .absent : .present
    }

    /// The `OSStatus` behind a `WalletStorageError.keychainError`; anything
    /// else (an override that throws its own error) maps to
    /// `errSecInternalError` so `unknown` still carries *a* status.
    private static func status(of error: Error) -> OSStatus {
        if case WalletStorageError.keychainError(let status) = error {
            return status
        }
        return errSecInternalError
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
