import Foundation
import SwiftData

// MARK: - Marshalling a fetched identity's keys

extension PersistentPublicKey {
    /// Marshal the public keys of an identity fetched from Platform
    /// (`SDK.identityGet`, which returns the `serde_json` form of the DPP
    /// `Identity`) into unsaved `PersistentPublicKey` rows.
    ///
    /// `publicKeys` is read in either shape the response has used: an
    /// array of key objects, or an object keyed by the key id. A key object
    /// that lacks `id`, `purpose`, `securityLevel`, `type` or a base64
    /// `data` is skipped. `disabledAt`, `totalBudget` and `expiresAt` are
    /// protocol `u64`s, read with `UInt64(jsonValue:)` because DPP writes
    /// one above 2^53 - 1 as a decimal string.
    ///
    /// Contract bounds are written to the three bounds columns:
    /// - `"contractBounds": null` is an unbounded key: kind 0, no id.
    /// - `{"$type": "singleContract", "id": <base58>}`: kind 1.
    /// - `{"$type": "documentType", "id": <base58>, "documentTypeName": ...}`:
    ///   kind 2 with the document type name.
    /// - `{"$type": "contractGroup", "id": <base58>}`: kind 3, the id is the
    ///   contract group's.
    ///
    /// When the field is missing, or holds a value none of these shapes
    /// decodes (an unknown `$type`, an id that is not 32 bytes of base58),
    /// the bounds are UNKNOWN: `contractBoundsKind` and
    /// `contractBoundsData` stay `nil`. `PersistentIdentity
    /// .mergeFetchedPublicKeys(_:)` reads that as "keep what is stored"
    /// rather than as "unbounded".
    ///
    /// Every row's `identityId` is `identityId`'s base58 string, the form
    /// the platform-wallet persister matches rows by. The rows are not
    /// inserted into any `ModelContext`; they are ordered by key id.
    public static func fromFetchedIdentity(
        _ identityJSON: [String: Any],
        identityId: Data
    ) -> [PersistentPublicKey] {
        let identityIdBase58 = identityId.toBase58String()
        var rows: [PersistentPublicKey] = []
        if let keysById = identityJSON["publicKeys"] as? [String: Any] {
            for (keyIdText, value) in keysById {
                guard let keyObject = value as? [String: Any],
                      let keyId = Int(keyIdText) ?? keyObject["id"] as? Int,
                      let row = fromFetchedKey(keyObject, keyId: keyId, identityId: identityIdBase58) else {
                    continue
                }
                rows.append(row)
            }
        } else if let keyObjects = identityJSON["publicKeys"] as? [[String: Any]] {
            for keyObject in keyObjects {
                guard let keyId = keyObject["id"] as? Int,
                      let row = fromFetchedKey(keyObject, keyId: keyId, identityId: identityIdBase58) else {
                    continue
                }
                rows.append(row)
            }
        }
        return rows.sorted { $0.keyId < $1.keyId }
    }

    /// One key object of a fetched identity as an unsaved row, or `nil`
    /// when a required field is missing or malformed.
    private static func fromFetchedKey(
        _ keyObject: [String: Any],
        keyId: Int,
        identityId: String
    ) -> PersistentPublicKey? {
        guard let rowKeyId = Int32(exactly: keyId),
              let purpose = keyObject["purpose"] as? Int,
              let securityLevel = keyObject["securityLevel"] as? Int,
              let keyType = keyObject["type"] as? Int,
              let dataText = keyObject["data"] as? String,
              let data = Data(base64Encoded: dataText) else {
            return nil
        }
        let bounds = FetchedContractBounds(keyObject: keyObject)
        return PersistentPublicKey(
            keyId: rowKeyId,
            purpose: UInt8(exactly: purpose).flatMap(KeyPurpose.init(rawValue:)) ?? .authentication,
            securityLevel: UInt8(exactly: securityLevel).flatMap(SecurityLevel.init(rawValue:)) ?? .high,
            keyType: UInt8(exactly: keyType).flatMap(KeyType.init(rawValue:)) ?? .ecdsaSecp256k1,
            publicKeyData: data,
            readOnly: keyObject["readOnly"] as? Bool ?? false,
            disabledAt: UInt64(jsonValue: keyObject["disabledAt"]).map { Int64(bitPattern: $0) },
            contractBounds: bounds.ids,
            contractBoundsDocumentTypeName: bounds.documentTypeName,
            totalBudget: UInt64(jsonValue: keyObject["totalBudget"]).map { Int64(bitPattern: $0) },
            expiresAt: UInt64(jsonValue: keyObject["expiresAt"]).map { Int64(bitPattern: $0) },
            contractBoundsKind: bounds.kind,
            identityId: identityId
        )
    }
}

/// The `contractBounds` field of one fetched key, projected onto the three
/// bounds columns of `PersistentPublicKey`. `kind == nil` means the bounds
/// could not be read (see `PersistentPublicKey.fromFetchedIdentity`).
private struct FetchedContractBounds {
    let ids: [Data]?
    let documentTypeName: String?
    let kind: Int?

    static let unknown = FetchedContractBounds(ids: nil, documentTypeName: nil, kind: nil)

    init(ids: [Data]?, documentTypeName: String?, kind: Int?) {
        self.ids = ids
        self.documentTypeName = documentTypeName
        self.kind = kind
    }

    init(keyObject: [String: Any]) {
        guard let value = keyObject["contractBounds"] else {
            self = .unknown
            return
        }
        if value is NSNull {
            self.init(ids: nil, documentTypeName: nil, kind: 0)
            return
        }
        guard let object = value as? [String: Any],
              let type = object["$type"] as? String,
              let idText = object["id"] as? String,
              let id = Data.identifier(fromBase58: idText),
              id.count == 32 else {
            self = .unknown
            return
        }
        switch type {
        case "singleContract":
            self.init(ids: [id], documentTypeName: nil, kind: 1)
        case "documentType":
            guard let name = object["documentTypeName"] as? String, !name.isEmpty else {
                self = .unknown
                return
            }
            self.init(ids: [id], documentTypeName: name, kind: 2)
        case "contractGroup":
            self.init(ids: [id], documentTypeName: nil, kind: 3)
        default:
            self = .unknown
        }
    }
}

// MARK: - Merging fetched keys into an identity

extension PersistentIdentity {
    /// Bring this identity's public-key rows in line with `fetched`, the
    /// rows `PersistentPublicKey.fromFetchedIdentity(_:identityId:)` built
    /// from a Platform fetch of the same identity.
    ///
    /// A fetched key MATCHES an existing row with the same key id, key type
    /// and public key bytes; each existing row matches at most one fetched
    /// key. On a match the existing row is updated in place and the fetched
    /// row is dropped:
    /// - purpose, security level, read-only, `disabledAt`, `totalBudget`
    ///   and `expiresAt` are copied from the fetched row;
    /// - the three contract bounds columns are copied only when the fetched
    ///   row's `contractBoundsKind` is not `nil`. A `nil` kind means the
    ///   fetch did not say what the bounds are, so the stored bounds stay;
    /// - `identityId` is rewritten to this identity's base58 string, which
    ///   also moves a row written with the legacy hex form onto the form the
    ///   platform-wallet persister matches by;
    /// - `privateKeyKeychainIdentifier`, `walletId`,
    ///   `identityDerivationPath`, `createdAt` and `lastAccessed` are kept,
    ///   so the key can still be signed with after a refresh.
    ///
    /// A fetched key with no match is appended as a new row (with this
    /// identity's base58 id), carrying none of that local metadata: a key
    /// whose bytes changed under the same id is a different key.
    ///
    /// An existing row no fetched key matched is removed from `publicKeys`
    /// AND deleted from this identity's `modelContext`, if it has one. Only
    /// unlinking it would leave an orphan row with the same key id and
    /// identity id in the store, and the persister, which finds a key row by
    /// that pair and takes the first result, could then update the orphan
    /// instead of the live row. Keychain items are never touched here.
    ///
    /// Does not save; the caller saves the context.
    public func mergeFetchedPublicKeys(_ fetched: [PersistentPublicKey]) {
        let identityIdBase58 = identityIdBase58
        let existing = publicKeys
        var matched = Set<ObjectIdentifier>()
        var additions: [PersistentPublicKey] = []
        for fresh in fetched {
            guard let current = existing.first(where: {
                !matched.contains(ObjectIdentifier($0))
                    && $0.keyId == fresh.keyId
                    && $0.keyType == fresh.keyType
                    && $0.publicKeyData == fresh.publicKeyData
            }) else {
                fresh.identityId = identityIdBase58
                additions.append(fresh)
                continue
            }
            matched.insert(ObjectIdentifier(current))
            current.purpose = fresh.purpose
            current.securityLevel = fresh.securityLevel
            current.readOnly = fresh.readOnly
            current.disabledAt = fresh.disabledAt
            current.totalBudget = fresh.totalBudget
            current.expiresAt = fresh.expiresAt
            if fresh.contractBoundsKind != nil {
                // Written column by column: the `contractBounds` setter
                // would reset the document type name and the kind.
                current.contractBoundsData = fresh.contractBoundsData
                current.contractBoundsDocumentTypeName = fresh.contractBoundsDocumentTypeName
                current.contractBoundsKind = fresh.contractBoundsKind
            }
            current.identityId = identityIdBase58
        }

        let stale = existing.filter { !matched.contains(ObjectIdentifier($0)) }
        if !stale.isEmpty {
            publicKeys.removeAll { !matched.contains(ObjectIdentifier($0)) }
            if let context = modelContext {
                for row in stale {
                    context.delete(row)
                }
            }
        }
        for addition in additions {
            addPublicKey(addition)
        }
        lastUpdated = Date()
    }
}
