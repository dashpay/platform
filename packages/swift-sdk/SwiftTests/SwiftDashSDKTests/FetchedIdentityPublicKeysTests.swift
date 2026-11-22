import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `PersistentPublicKey.fromFetchedIdentity(_:identityId:)`
/// and `PersistentIdentity.mergeFetchedPublicKeys(_:)`, the pair the example
/// app's identity load and key refresh write an identity's key rows with.
///
/// The refresh used to rebuild every key row from the fetch with no contract
/// bounds and carry over only the Keychain identifier, so a wallet-derived
/// key lost the wallet id and derivation path it signs with, and a bounded
/// key came back unbounded. These tests pin that an unchanged key now keeps
/// its row and its local metadata, that changed or dropped keys leave no row
/// behind for the platform-wallet persister to match, and that the bounds
/// the fetch carries land in the right columns.
@MainActor
final class FetchedIdentityPublicKeysTests: XCTestCase {

    private let identityId = Data(repeating: 0x35, count: 32)
    private let contractId = Data(repeating: 0x6B, count: 32)
    private let groupId = Data(repeating: 0x5C, count: 32)
    private let walletId = Data(repeating: 0x07, count: 32)
    private let keychainIdentifier = "privkey_test_1"
    private let derivationPath = "m/9'/1'/5'/0'/0'/0'/1'"

    private func publicKeyData(_ seed: UInt8) -> Data {
        Data([0x02] + [UInt8](repeating: seed, count: 32))
    }

    // MARK: - JSON fixtures

    /// The `contractBounds` field of one key object.
    private enum Bounds {
        /// The field is not in the object at all.
        case absent
        /// `"contractBounds": null`, how DPP writes an unbounded key.
        case null
        /// An internally tagged bounds object.
        case object([String: Any])
    }

    private func keyObject(
        id: Int,
        data: Data,
        bounds: Bounds,
        extra: [String: Any] = [:]
    ) -> [String: Any] {
        var object: [String: Any] = [
            "$formatVersion": "0",
            "id": id,
            "purpose": Int(KeyPurpose.authentication.rawValue),
            "securityLevel": Int(SecurityLevel.high.rawValue),
            "type": Int(KeyType.ecdsaSecp256k1.rawValue),
            "readOnly": false,
            "data": data.base64EncodedString(),
            "disabledAt": NSNull(),
        ]
        switch bounds {
        case .absent:
            break
        case .null:
            object["contractBounds"] = NSNull()
        case .object(let value):
            object["contractBounds"] = value
        }
        for (key, value) in extra {
            object[key] = value
        }
        return object
    }

    private var documentTypeBounds: Bounds {
        .object([
            "$type": "documentType",
            "id": contractId.toBase58String(),
            "documentTypeName": "contactRequest",
        ])
    }

    private var groupBounds: Bounds {
        .object(["$type": "contractGroup", "id": groupId.toBase58String()])
    }

    /// An `identityGet` response, put through `JSONSerialization` so the
    /// values have the types the real response parses into.
    private func identityJSON(
        _ keys: [[String: Any]],
        keyedById: Bool = false
    ) throws -> [String: Any] {
        let publicKeys: Any
        if keyedById {
            var byId: [String: Any] = [:]
            for key in keys {
                byId[String(describing: key["id"] ?? "")] = key
            }
            publicKeys = byId
        } else {
            publicKeys = keys
        }
        let object: [String: Any] = [
            "$formatVersion": "0",
            "id": identityId.toBase58String(),
            "balance": 1_000,
            "revision": 1,
            "publicKeys": publicKeys,
        ]
        let data = try JSONSerialization.data(withJSONObject: object)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    // MARK: - Store helpers

    private func makeStore() throws -> (ModelContainer, ModelContext, PersistentIdentity) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        context.insert(identity)
        try context.save()
        return (container, context, identity)
    }

    private func allKeyRows(_ context: ModelContext) throws -> [PersistentPublicKey] {
        try context.fetch(FetchDescriptor<PersistentPublicKey>())
    }

    /// The key row the platform-wallet persister would find: it fetches by
    /// key id and the identity's base58 string and takes the first result.
    private func persisterRows(_ context: ModelContext, keyId: Int32) throws -> [PersistentPublicKey] {
        let identityIdText = identityId.toBase58String()
        return try context.fetch(FetchDescriptor<PersistentPublicKey>(
            predicate: #Predicate {
                $0.keyId == keyId && $0.identityId == identityIdText
            }
        ))
    }

    private func setLocalMetadata(_ row: PersistentPublicKey) {
        row.privateKeyKeychainIdentifier = keychainIdentifier
        row.walletId = walletId
        row.identityDerivationPath = derivationPath
        row.lastAccessed = Date(timeIntervalSince1970: 1_700_000_000)
    }

    // MARK: - Import then refresh

    /// The bug this pair fixes: load the identity, let the wallet attach its
    /// signing breadcrumb, then refresh. The unchanged keys keep their rows,
    /// their Keychain identifier, wallet id and derivation path, and their
    /// bounds, including a contract group bound (kind 3) the DPP-layer
    /// `ContractBounds` cannot express.
    func testRefreshKeepsTheLocalMetadataAndBoundsOfUnchangedKeys() throws {
        let (_, context, identity) = try makeStore()
        let response = try identityJSON([
            keyObject(id: 0, data: publicKeyData(0xA0), bounds: .null),
            keyObject(id: 1, data: publicKeyData(0xA1), bounds: documentTypeBounds),
            keyObject(id: 2, data: publicKeyData(0xA2), bounds: groupBounds),
        ])

        identity.mergeFetchedPublicKeys(
            PersistentPublicKey.fromFetchedIdentity(response, identityId: identityId))
        try context.save()

        let imported = identity.publicKeys.sorted { $0.keyId < $1.keyId }
        XCTAssertEqual(imported.map(\.keyId), [0, 1, 2])
        for row in imported.dropFirst() {
            setLocalMetadata(row)
        }
        try context.save()
        let importedIds = imported.map(\.persistentModelID)
        let createdAt = imported.map(\.createdAt)

        identity.mergeFetchedPublicKeys(
            PersistentPublicKey.fromFetchedIdentity(response, identityId: identityId))
        try context.save()

        let refreshed = identity.publicKeys.sorted { $0.keyId < $1.keyId }
        XCTAssertEqual(refreshed.map(\.persistentModelID), importedIds,
                       "unchanged keys must keep their rows")
        XCTAssertEqual(refreshed.map(\.createdAt), createdAt)
        XCTAssertEqual(try allKeyRows(context).count, 3, "a refresh must not add rows")

        let documentTypeKey = refreshed[1]
        XCTAssertEqual(documentTypeKey.privateKeyKeychainIdentifier, keychainIdentifier)
        XCTAssertEqual(documentTypeKey.walletId, walletId)
        XCTAssertEqual(documentTypeKey.identityDerivationPath, derivationPath)
        XCTAssertEqual(documentTypeKey.lastAccessed, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertEqual(documentTypeKey.contractBoundsKind, 2)
        XCTAssertEqual(documentTypeKey.contractBounds, [contractId])
        XCTAssertEqual(documentTypeKey.contractBoundsDocumentTypeName, "contactRequest")

        let groupKey = refreshed[2]
        XCTAssertEqual(groupKey.privateKeyKeychainIdentifier, keychainIdentifier)
        XCTAssertEqual(groupKey.walletId, walletId)
        XCTAssertEqual(groupKey.identityDerivationPath, derivationPath)
        XCTAssertEqual(groupKey.contractBoundsKind, 3)
        XCTAssertEqual(groupKey.contractBounds, [groupId])
        XCTAssertNil(groupKey.contractBoundsDocumentTypeName)

        XCTAssertEqual(refreshed[0].contractBoundsKind, 0)
        XCTAssertNil(refreshed[0].contractBoundsData)
        for keyId in Int32(0)...2 {
            XCTAssertEqual(try persisterRows(context, keyId: keyId).count, 1,
                           "the persister must find exactly one row for key \(keyId)")
        }
    }

    /// A key whose bytes changed under the same id is a different key: its
    /// old row leaves the store, and the new row carries none of the old
    /// key's local metadata.
    func testRefreshReplacesAKeyWhoseBytesChanged() throws {
        let (_, context, identity) = try makeStore()
        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: documentTypeBounds)]),
            identityId: identityId))
        try context.save()
        let oldRow = try XCTUnwrap(identity.publicKeys.first)
        setLocalMetadata(oldRow)
        try context.save()
        let oldRowId = oldRow.persistentModelID

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xB1), bounds: .null)]),
            identityId: identityId))
        try context.save()

        let rows = try allKeyRows(context)
        XCTAssertEqual(rows.count, 1, "the old row must be deleted, not only unlinked")
        let newRow = try XCTUnwrap(rows.first)
        XCTAssertNotEqual(newRow.persistentModelID, oldRowId)
        XCTAssertEqual(identity.publicKeys.map(\.persistentModelID), [newRow.persistentModelID])
        XCTAssertEqual(newRow.keyId, 1)
        XCTAssertEqual(newRow.publicKeyData, publicKeyData(0xB1))
        XCTAssertEqual(newRow.identityId, identityId.toBase58String())
        XCTAssertNil(newRow.privateKeyKeychainIdentifier)
        XCTAssertNil(newRow.walletId)
        XCTAssertNil(newRow.identityDerivationPath)
        XCTAssertNil(newRow.lastAccessed)
        XCTAssertEqual(newRow.contractBoundsKind, 0)
        XCTAssertEqual(try persisterRows(context, keyId: 1).map(\.persistentModelID),
                       [newRow.persistentModelID])
    }

    /// A key Platform no longer returns is taken off the identity and
    /// deleted from the store, so no orphan with its key id is left for the
    /// persister to update.
    func testRefreshDeletesAKeyNoLongerFetched() throws {
        let (_, context, identity) = try makeStore()
        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([
                keyObject(id: 0, data: publicKeyData(0xA0), bounds: .null),
                keyObject(id: 1, data: publicKeyData(0xA1), bounds: .null),
            ]),
            identityId: identityId))
        try context.save()
        let kept = try XCTUnwrap(identity.publicKeys.first { $0.keyId == 0 })
        setLocalMetadata(try XCTUnwrap(identity.publicKeys.first { $0.keyId == 1 }))
        try context.save()

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 0, data: publicKeyData(0xA0), bounds: .null)]),
            identityId: identityId))
        try context.save()

        XCTAssertEqual(identity.publicKeys.map(\.persistentModelID), [kept.persistentModelID])
        XCTAssertEqual(try allKeyRows(context).map(\.keyId), [0])
        XCTAssertTrue(try persisterRows(context, keyId: 1).isEmpty)
    }

    /// A row written by the old load path carries the identity id as HEX,
    /// which the persister's base58 lookup cannot see. The merge matches it
    /// by key id, type and bytes anyway, keeps its metadata and rewrites the
    /// id to base58.
    func testMergeRewritesALegacyHexIdentityIdToBase58() throws {
        let (_, context, identity) = try makeStore()
        let legacy = PersistentPublicKey(
            keyId: 1,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData(0xA1),
            contractBounds: [contractId],
            contractBoundsDocumentTypeName: "contactRequest",
            contractBoundsKind: 2,
            identityId: identityId.toHexString())
        context.insert(legacy)
        identity.addPublicKey(legacy)
        setLocalMetadata(legacy)
        try context.save()
        XCTAssertTrue(try persisterRows(context, keyId: 1).isEmpty)

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: documentTypeBounds)]),
            identityId: identityId))
        try context.save()

        XCTAssertEqual(try allKeyRows(context).map(\.persistentModelID), [legacy.persistentModelID])
        XCTAssertEqual(legacy.identityId, identityId.toBase58String())
        XCTAssertEqual(legacy.privateKeyKeychainIdentifier, keychainIdentifier)
        XCTAssertEqual(legacy.walletId, walletId)
        XCTAssertEqual(legacy.identityDerivationPath, derivationPath)
        XCTAssertEqual(try persisterRows(context, keyId: 1).map(\.persistentModelID),
                       [legacy.persistentModelID])
    }

    // MARK: - Parser

    /// Each bounds shape lands in the three columns the platform-wallet
    /// persister writes for the same variant; `null` is unbounded (kind 0)
    /// and a missing or unreadable field is unknown (kind `nil`).
    func testParserDecodesEveryContractBoundsShape() throws {
        let response = try identityJSON([
            keyObject(id: 0, data: publicKeyData(0xA0), bounds: .null),
            keyObject(id: 1, data: publicKeyData(0xA1), bounds: .object([
                "$type": "singleContract", "id": contractId.toBase58String(),
            ])),
            keyObject(id: 2, data: publicKeyData(0xA2), bounds: documentTypeBounds),
            keyObject(id: 3, data: publicKeyData(0xA3), bounds: groupBounds),
            keyObject(id: 4, data: publicKeyData(0xA4), bounds: .absent),
            keyObject(id: 5, data: publicKeyData(0xA5), bounds: .object([
                "$type": "somethingNew", "id": contractId.toBase58String(),
            ])),
            keyObject(id: 6, data: publicKeyData(0xA6), bounds: .object([
                "$type": "singleContract", "id": "not base58 0OIl",
            ])),
            keyObject(id: 7, data: publicKeyData(0xA7), bounds: .object([
                "$type": "documentType", "id": contractId.toBase58String(),
            ])),
        ])

        let rows = PersistentPublicKey.fromFetchedIdentity(response, identityId: identityId)
        XCTAssertEqual(rows.map(\.keyId), [0, 1, 2, 3, 4, 5, 6, 7])
        let columns = rows.map { ($0.contractBoundsKind, $0.contractBounds, $0.contractBoundsDocumentTypeName) }

        XCTAssertEqual(columns[0].0, 0)
        XCTAssertNil(columns[0].1)
        XCTAssertNil(columns[0].2)

        XCTAssertEqual(columns[1].0, 1)
        XCTAssertEqual(columns[1].1, [contractId])
        XCTAssertNil(columns[1].2)

        XCTAssertEqual(columns[2].0, 2)
        XCTAssertEqual(columns[2].1, [contractId])
        XCTAssertEqual(columns[2].2, "contactRequest")

        XCTAssertEqual(columns[3].0, 3)
        XCTAssertEqual(columns[3].1, [groupId])
        XCTAssertNil(columns[3].2)

        // Absent, an unknown `$type`, an id that is not base58 and a
        // document type bound with no name are all unknown.
        for index in 4...7 {
            XCTAssertNil(rows[index].contractBoundsKind, "key \(index)")
            XCTAssertNil(rows[index].contractBoundsData, "key \(index)")
            XCTAssertNil(rows[index].contractBoundsDocumentTypeName, "key \(index)")
        }
    }

    /// The other key fields, the object-keyed-by-id `publicKeys` shape, the
    /// base58 identity id, and that parsing inserts nothing.
    func testParserReadsBothShapesAndInsertsNothing() throws {
        let (_, context, _) = try makeStore()
        let keys = [
            keyObject(id: 3, data: publicKeyData(0xA3), bounds: .null, extra: [
                "purpose": Int(KeyPurpose.transfer.rawValue),
                "securityLevel": Int(SecurityLevel.critical.rawValue),
                "readOnly": true,
                "disabledAt": 1_700_000_000_000,
                "totalBudget": "9007199254740992",
                "expiresAt": 1_800_000_000_000,
            ]),
            keyObject(id: 0, data: publicKeyData(0xA0), bounds: .null),
            // Skipped: `data` is not base64.
            keyObject(id: 9, data: Data(), bounds: .null, extra: ["data": "%%%"]),
        ]

        for keyedById in [false, true] {
            let rows = PersistentPublicKey.fromFetchedIdentity(
                try identityJSON(keys, keyedById: keyedById), identityId: identityId)
            XCTAssertEqual(rows.map(\.keyId), [0, 3], "keyedById: \(keyedById)")
            let key = rows[1]
            XCTAssertEqual(key.purposeEnum, .transfer)
            XCTAssertEqual(key.securityLevelEnum, .critical)
            XCTAssertEqual(key.keyTypeEnum, .ecdsaSecp256k1)
            XCTAssertTrue(key.readOnly)
            XCTAssertEqual(key.disabledAt, 1_700_000_000_000)
            XCTAssertEqual(key.totalBudgetCredits, 9_007_199_254_740_992)
            XCTAssertEqual(key.expiresAtMillis, 1_800_000_000_000)
            XCTAssertEqual(key.publicKeyData, publicKeyData(0xA3))
            for row in rows {
                XCTAssertEqual(row.identityId, identityId.toBase58String())
                XCTAssertNil(row.modelContext)
                XCTAssertNil(row.identity)
            }
        }
        XCTAssertTrue(try allKeyRows(context).isEmpty)
        XCTAssertTrue(PersistentPublicKey.fromFetchedIdentity([:], identityId: identityId).isEmpty)
    }

    /// A fetch that does not say what a key's bounds are (kind `nil`)
    /// leaves the stored bounds alone; one that says `null` clears them.
    func testMergeKeepsStoredBoundsOnlyWhenTheFetchedBoundsAreUnknown() throws {
        let (_, context, identity) = try makeStore()
        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: groupBounds)]),
            identityId: identityId))
        try context.save()
        let row = try XCTUnwrap(identity.publicKeys.first)

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: .absent)]),
            identityId: identityId))
        try context.save()
        XCTAssertEqual(identity.publicKeys.map(\.persistentModelID), [row.persistentModelID])
        XCTAssertEqual(row.contractBoundsKind, 3)
        XCTAssertEqual(row.contractBounds, [groupId])

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: documentTypeBounds)]),
            identityId: identityId))
        try context.save()
        XCTAssertEqual(row.contractBoundsKind, 2)
        XCTAssertEqual(row.contractBounds, [contractId])
        XCTAssertEqual(row.contractBoundsDocumentTypeName, "contactRequest")

        identity.mergeFetchedPublicKeys(PersistentPublicKey.fromFetchedIdentity(
            try identityJSON([keyObject(id: 1, data: publicKeyData(0xA1), bounds: .null)]),
            identityId: identityId))
        try context.save()
        XCTAssertEqual(row.contractBoundsKind, 0)
        XCTAssertNil(row.contractBoundsData)
        XCTAssertNil(row.contractBoundsDocumentTypeName)
    }
}
