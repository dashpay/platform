import Foundation
import SwiftData

// MARK: - Marshalling a fetched identity's keys

extension PersistentPublicKey {
    /// Marshal the public keys of an identity fetched from Platform
    /// (`SDK.identityGet`, which returns the `serde_json` form of the DPP
    /// `Identity`) into unsaved `PersistentPublicKey` rows, or return `nil`
    /// when the key list cannot be read completely.
    ///
    /// `nil` means "do not apply": `PersistentIdentity
    /// .mergeFetchedPublicKeys(_:)` writes each fetched row over the stored
    /// row with its key id, so a key read only in part (a defaulted purpose,
    /// bounds it could not decode) must never reach it, and neither may a
    /// list whose entries could not all be read. The result is `nil` when:
    /// - `publicKeys` is missing, or is neither an array of key objects nor
    ///   an object keyed by the key id (the two shapes the response has
    ///   used);
    /// - any entry is not an object, or lacks or holds a malformed `id`,
    ///   `purpose`, `securityLevel`, `type`, `readOnly` or `data`; the id
    ///   must fit the protocol's `UInt32` (and, in the keyed shape, agree
    ///   with the key it is under), and `data` must be base64;
    /// - a purpose, security level or key type is a raw value the Swift
    ///   `KeyPurpose`, `SecurityLevel` or `KeyType` does not know;
    /// - `contractBounds` is absent (DPP always writes it) or holds a value
    ///   none of the shapes below decodes;
    /// - `disabledAt`, `totalBudget` or `expiresAt` holds a value
    ///   `UInt64(jsonValue:)` cannot read. These are optional protocol
    ///   `u64`s: absent or `null` is "none", and DPP writes one above
    ///   2^53 - 1 as a decimal string;
    /// - two entries carry the same key id.
    ///
    /// An empty list is a complete list and gives `[]`.
    ///
    /// Contract bounds are written to the three bounds columns:
    /// - `"contractBounds": null` is an unbounded key: kind 0, no id.
    /// - `{"$type": "singleContract", "id": <base58>}`: kind 1.
    /// - `{"$type": "documentType", "id": <base58>, "documentTypeName": ...}`:
    ///   kind 2 with the (non-empty) document type name.
    /// - `{"$type": "contractGroup", "id": <base58>}`: kind 3, the id is the
    ///   contract group's.
    ///
    /// In each object shape the id must decode from base58 to 32 bytes.
    ///
    /// Every row's `identityId` is `identityId`'s base58 string, the form
    /// the platform-wallet persister writes. The rows are not inserted into
    /// any `ModelContext`; they are ordered by key id.
    public static func fromFetchedIdentity(
        _ identityJSON: [String: Any],
        identityId: Data
    ) -> [PersistentPublicKey]? {
        let identityIdBase58 = identityId.toBase58String()
        var rows: [PersistentPublicKey] = []
        if let keysById = identityJSON["publicKeys"] as? [String: Any] {
            for (keyIdText, value) in keysById {
                guard let keyObject = value as? [String: Any],
                      let keyId = UInt32(keyIdText),
                      let row = fromFetchedKey(keyObject, identityId: identityIdBase58),
                      row.keyId == Int32(bitPattern: keyId) else {
                    return nil
                }
                rows.append(row)
            }
        } else if let entries = identityJSON["publicKeys"] as? [Any] {
            for entry in entries {
                guard let keyObject = entry as? [String: Any],
                      let row = fromFetchedKey(keyObject, identityId: identityIdBase58) else {
                    return nil
                }
                rows.append(row)
            }
        } else {
            return nil
        }
        guard Set(rows.map(\.keyId)).count == rows.count else {
            return nil
        }
        return rows.sorted { $0.keyId < $1.keyId }
    }

    /// One key object of a fetched identity as an unsaved row, or `nil`
    /// when any field cannot be read (see `fromFetchedIdentity`).
    private static func fromFetchedKey(
        _ keyObject: [String: Any],
        identityId: String
    ) -> PersistentPublicKey? {
        guard let keyId = jsonUInt32(keyObject["id"]),
              let purpose = jsonUInt8(keyObject["purpose"]).flatMap(KeyPurpose.init(rawValue:)),
              let securityLevel = jsonUInt8(keyObject["securityLevel"]).flatMap(SecurityLevel.init(rawValue:)),
              let keyType = jsonUInt8(keyObject["type"]).flatMap(KeyType.init(rawValue:)),
              let readOnly = jsonBool(keyObject["readOnly"]),
              let dataText = keyObject["data"] as? String,
              let data = Data(base64Encoded: dataText),
              let bounds = FetchedContractBounds(keyObject: keyObject),
              let disabledAt = optionalUInt64(keyObject["disabledAt"]),
              let totalBudget = optionalUInt64(keyObject["totalBudget"]),
              let expiresAt = optionalUInt64(keyObject["expiresAt"]) else {
            return nil
        }
        return PersistentPublicKey(
            // The row stores the protocol `UInt32` key id as its bit pattern.
            keyId: Int32(bitPattern: keyId),
            purpose: purpose,
            securityLevel: securityLevel,
            keyType: keyType,
            publicKeyData: data,
            readOnly: readOnly,
            disabledAt: disabledAt.map { Int64(bitPattern: $0) },
            contractBounds: bounds.ids,
            contractBoundsDocumentTypeName: bounds.documentTypeName,
            totalBudget: totalBudget.map { Int64(bitPattern: $0) },
            expiresAt: expiresAt.map { Int64(bitPattern: $0) },
            contractBoundsKind: bounds.kind,
            identityId: identityId
        )
    }

    /// A JSON number (not a boolean or a string) that is a non-negative
    /// integer fitting `UInt32`.
    private static func jsonUInt32(_ value: Any?) -> UInt32? {
        guard value is NSNumber, let parsed = UInt64(jsonValue: value) else {
            return nil
        }
        return UInt32(exactly: parsed)
    }

    /// A JSON number that is a non-negative integer fitting `UInt8`, the
    /// raw value type of `KeyPurpose`, `SecurityLevel` and `KeyType`.
    private static func jsonUInt8(_ value: Any?) -> UInt8? {
        jsonUInt32(value).flatMap { UInt8(exactly: $0) }
    }

    /// A JSON boolean. A number is not read as one.
    private static func jsonBool(_ value: Any?) -> Bool? {
        guard let number = value as? NSNumber,
              CFGetTypeID(number) == CFBooleanGetTypeID() else {
            return nil
        }
        return number.boolValue
    }

    /// An optional protocol `u64`: `.some(nil)` when the field is absent or
    /// `null`, `.some(value)` when `UInt64(jsonValue:)` reads it, and `nil`
    /// when it holds anything else.
    private static func optionalUInt64(_ value: Any?) -> UInt64?? {
        guard let value, !(value is NSNull) else {
            return .some(nil)
        }
        guard let parsed = UInt64(jsonValue: value) else {
            return nil
        }
        return .some(parsed)
    }
}

/// The `contractBounds` field of one fetched key, projected onto the three
/// bounds columns of `PersistentPublicKey`. The initializer fails when the
/// field is absent or no known shape decodes it (see
/// `PersistentPublicKey.fromFetchedIdentity`).
private struct FetchedContractBounds {
    let ids: [Data]?
    let documentTypeName: String?
    let kind: Int

    init?(keyObject: [String: Any]) {
        guard let value = keyObject["contractBounds"] else {
            return nil
        }
        if value is NSNull {
            ids = nil
            documentTypeName = nil
            kind = 0
            return
        }
        guard let object = value as? [String: Any],
              let type = object["$type"] as? String,
              let idText = object["id"] as? String,
              let id = Data.identifier(fromBase58: idText),
              id.count == 32 else {
            return nil
        }
        ids = [id]
        switch type {
        case "singleContract":
            documentTypeName = nil
            kind = 1
        case "documentType":
            guard let name = object["documentTypeName"] as? String, !name.isEmpty else {
                return nil
            }
            documentTypeName = name
            kind = 2
        case "contractGroup":
            documentTypeName = nil
            kind = 3
        default:
            return nil
        }
    }
}

// MARK: - Merging fetched keys into an identity

extension PersistentIdentity {
    /// Bring this identity's public-key rows in line with `fetched`, the
    /// rows `PersistentPublicKey.fromFetchedIdentity(_:identityId:)` built
    /// from a Platform fetch of the same identity. Call it only with a list
    /// that parser returned (not `nil`).
    ///
    /// The rules follow from what the protocol allows. Drive never removes
    /// an identity key: its identity update methods (rs-drive
    /// `drive/identity/update/methods`) add keys, disable or re-enable them
    /// and change a key's budget or expiry, but have no removal. An identity
    /// update also refuses a new key under an id the identity already has,
    /// so the type and bytes under an existing key id never change. The SDK also accepts a proof one block behind the latest, so
    /// a response can predate a key that was just added.
    ///
    /// - A stored row with the same key id, key type and public key bytes as
    ///   a fetched key is updated in place: purpose, security level,
    ///   read-only, `disabledAt`, `totalBudget`, `expiresAt` and the three
    ///   contract bounds columns are copied from the fetched row, and
    ///   `identityId` is rewritten to this identity's base58 string (which
    ///   moves a row written with the legacy hex form onto the form the
    ///   platform-wallet persister writes). `privateKeyKeychainIdentifier`,
    ///   `walletId`, `identityDerivationPath`, `createdAt` and
    ///   `lastAccessed` are kept, so the key can still sign after a refresh.
    /// - A stored row that no fetched key shares a key id with is KEPT
    ///   unchanged and still linked. Since keys are never removed, its
    ///   absence means the response predates the key, and dropping the row
    ///   would lose its signing metadata: a later fetch would recreate it
    ///   without its Keychain identifier, wallet id or derivation path.
    /// - A stored row with the same key id as a fetched key but a different
    ///   key type or bytes (which the protocol does not produce, so the row
    ///   is not a key this identity has) is replaced: it is unlinked and
    ///   deleted from this identity's `modelContext`, if it has one, and the
    ///   fetched row is added carrying none of its local metadata. Any
    ///   other stored row with that key id, beyond the one a match updated,
    ///   is removed the same way.
    /// - A fetched key with no stored row under its id is added as a new row
    ///   with this identity's base58 id.
    ///
    /// A replaced row is deleted, not only unlinked. The persister first
    /// takes the row linked to the identity with that key id and, only when
    /// there is none, falls back to a `(keyId, base58 identityId)` query and
    /// links what that finds; a stale unlinked row with the same key id could
    /// be found by that fallback and re-linked, and cold restore, which reads
    /// the identity's linked rows, would then restore the wrong key.
    ///
    /// Keychain items are never touched here. Does not save; the caller
    /// saves the context.
    public func mergeFetchedPublicKeys(_ fetched: [PersistentPublicKey]) {
        let identityIdBase58 = identityIdBase58
        let existing = publicKeys
        var matched = Set<ObjectIdentifier>()
        var replaced: [PersistentPublicKey] = []
        var additions: [PersistentPublicKey] = []
        for fresh in fetched {
            let sameId = existing.filter {
                $0.keyId == fresh.keyId && !matched.contains(ObjectIdentifier($0))
            }
            let current = sameId.first {
                $0.keyType == fresh.keyType && $0.publicKeyData == fresh.publicKeyData
            }
            replaced.append(contentsOf: sameId.filter { $0 !== current })
            guard let current else {
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
            // Written column by column: the `contractBounds` setter would
            // reset the document type name and the kind.
            current.contractBoundsData = fresh.contractBoundsData
            current.contractBoundsDocumentTypeName = fresh.contractBoundsDocumentTypeName
            current.contractBoundsKind = fresh.contractBoundsKind
            current.identityId = identityIdBase58
        }

        // A row an earlier fetched key matched is never replaced by a later
        // one (only possible when `fetched` repeats a key id).
        let staleIds = Set(replaced.map { ObjectIdentifier($0) }).subtracting(matched)
        let stale = existing.filter { staleIds.contains(ObjectIdentifier($0)) }
        if !stale.isEmpty {
            publicKeys.removeAll { staleIds.contains(ObjectIdentifier($0)) }
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
