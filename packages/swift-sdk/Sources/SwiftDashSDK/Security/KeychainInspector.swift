import Foundation
import Security

// MARK: - KeychainInspector

/// Read-only metadata enumerator for
/// `kSecClassGenericPassword` items under a given `kSecAttrService`.
///
/// Produced to back diagnostic UIs (the example app's "Keychain
/// Explorer") that want to see *what* the app stores, without
/// pulling the stored values into memory. Every query sets
/// `kSecReturnData = false` and never calls `SecItemCopyMatching`
/// with the data flag set, so key material stays encrypted in the
/// keychain throughout the lifetime of the returned summaries.
///
/// Both of this SDK's keychain call sites —
/// [`KeychainManager`](x-source-tag://KeychainManager) (identity
/// private keys + special keys) and `WalletStorage` (seeds,
/// mnemonics, PIN material) — store items with `kSecClass ==
/// kSecClassGenericPassword`, which is the only class this
/// inspector supports.
public struct KeychainInspector: Sendable {
    public init() {}

    /// Enumerate every generic-password item filed under `service`.
    ///
    /// Returns `[]` on `errSecItemNotFound` or any other SecItem
    /// failure — this is a diagnostic surface, not a correctness
    /// path. Items are returned sorted ascending by account name
    /// (case-insensitive) so the caller can render them stably.
    public func listItems(service: String) -> [KeychainItemSummary] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
            // Explicitly false — never pull secret data into memory
            // through this inspector.
            kSecReturnData as String: false,
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            // `errSecItemNotFound` is the "empty service" case and is
            // expected; everything else is also silenced because the
            // UI can't usefully act on it.
            return []
        }

        guard let items = result as? [[String: Any]] else {
            return []
        }

        return items
            .map { attrs in KeychainItemSummary(service: service, attributes: attrs) }
            .sorted { lhs, rhs in
                lhs.account.localizedCaseInsensitiveCompare(rhs.account) == .orderedAscending
            }
    }
}

// MARK: - KeychainItemSummary

/// Metadata-only summary of a single keychain item. Intentionally
/// omits `kSecValueData` — callers that need the bytes must query
/// directly through `KeychainManager` / `WalletStorage` with their
/// authentication context.
public struct KeychainItemSummary: Identifiable, Hashable, Sendable {
    /// Stable id across diffs in SwiftUI lists — service + account
    /// uniquely identifies a generic-password row.
    public var id: String { "\(service)/\(account)" }

    /// `kSecAttrService` the item was filed under.
    public let service: String
    /// `kSecAttrAccount` — the key-identifier string. Follows the
    /// app's naming conventions (`privkey_…`, `specialkey_…`,
    /// `wallet.mnemonic.<hex>`, etc.).
    public let account: String
    /// `kSecAttrCreationDate`.
    public let createdAt: Date?
    /// `kSecAttrModificationDate`.
    public let modifiedAt: Date?
    /// `kSecAttrAccessible` string (e.g.
    /// `"dk"` for `WhenUnlockedThisDeviceOnly`). The constants are
    /// 4-byte CoreFoundation codes; the raw string is surfaced here
    /// unchanged so the detail view can render it verbatim.
    public let accessibleLevel: String?
    /// `kSecAttrSynchronizable` — true if iCloud-synced.
    public let synchronizable: Bool
    /// `kSecAttrLabel` / `kSecAttrDescription` / `kSecAttrComment`.
    /// Optional user-facing metadata Apple surfaces in Keychain
    /// Access; often nil for SDK-stored items.
    public let label: String?
    public let itemDescription: String?
    public let comment: String?
    /// `kSecAttrCreator` — a fourcc if the caller set one.
    public let creator: String?
    /// Decoded `kSecAttrGeneric`. `KeychainManager.storePrivateKey`
    /// stores a JSON dictionary there (`identityId`, `keyIndex`,
    /// `createdAt`); this is the pretty-printed form for display.
    public let genericMetadata: String?
    /// `kSecAttrGeneric` raw byte count, when present. Exposes
    /// nothing about the underlying value.
    public let genericDataBytes: Int

    init(service: String, attributes: [String: Any]) {
        self.service = service
        self.account = (attributes[kSecAttrAccount as String] as? String) ?? "(unnamed)"
        self.createdAt = attributes[kSecAttrCreationDate as String] as? Date
        self.modifiedAt = attributes[kSecAttrModificationDate as String] as? Date
        self.accessibleLevel = attributes[kSecAttrAccessible as String] as? String
        self.synchronizable = (attributes[kSecAttrSynchronizable as String] as? Bool) ?? false
        self.label = attributes[kSecAttrLabel as String] as? String
        self.itemDescription = attributes[kSecAttrDescription as String] as? String
        self.comment = attributes[kSecAttrComment as String] as? String

        // `kSecAttrCreator` is delivered as a `CFNumber` (OSType /
        // FourCharCode). Render it as a 4-char ASCII string when the
        // bits land in the printable ASCII range; fall back to hex.
        if let raw = attributes[kSecAttrCreator as String] as? UInt32 {
            self.creator = Self.formatFourCharCode(raw)
        } else {
            self.creator = nil
        }

        if let generic = attributes[kSecAttrGeneric as String] as? Data {
            self.genericDataBytes = generic.count
            self.genericMetadata = Self.describeGenericPayload(generic)
        } else {
            self.genericDataBytes = 0
            self.genericMetadata = nil
        }
    }

    /// Interpret a `kSecAttrGeneric` byte blob as either
    /// pretty-printed JSON (the convention `KeychainManager` follows)
    /// or a UTF-8 string. Falls back to a hex preview for opaque
    /// blobs so the detail view still has something to show.
    private static func describeGenericPayload(_ data: Data) -> String? {
        if let obj = try? JSONSerialization.jsonObject(with: data),
            let prettyData = try? JSONSerialization.data(
                withJSONObject: obj,
                options: [.prettyPrinted, .sortedKeys]
            ),
            let pretty = String(data: prettyData, encoding: .utf8)
        {
            return pretty
        }
        if let str = String(data: data, encoding: .utf8),
            !str.isEmpty,
            str.allSatisfy({ !$0.isNewline || $0 == "\n" })
        {
            return str
        }
        let preview = data.prefix(32)
            .map { String(format: "%02x", $0) }
            .joined()
        return preview.isEmpty ? nil : "hex: \(preview)\(data.count > 32 ? "…" : "")"
    }

    private static func formatFourCharCode(_ raw: UInt32) -> String {
        let bytes: [UInt8] = [
            UInt8((raw >> 24) & 0xff),
            UInt8((raw >> 16) & 0xff),
            UInt8((raw >> 8) & 0xff),
            UInt8(raw & 0xff),
        ]
        if bytes.allSatisfy({ $0 >= 0x20 && $0 < 0x7f }) {
            return String(bytes: bytes, encoding: .ascii) ?? String(format: "0x%08x", raw)
        }
        return String(format: "0x%08x", raw)
    }
}
