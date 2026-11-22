import SwiftData
import XCTest

@testable import SwiftDashSDK

/// The key usage limits protocol version 14 added (`totalBudget`, the credits
/// a key may take from its identity over its whole lifetime, and `expiresAt`,
/// the block time in milliseconds from which it can no longer sign) have to
/// survive every hop they make on the client: the persister round that first
/// writes them, a later round that RAISES them (what an
/// `IdentityKeyLimitsUpdate` emits), and the `IdentityPublicKey` projection the
/// rest of the SDK reads a stored key through.
@MainActor
final class IdentityKeyLimitsPersistenceTests: XCTestCase {

    private let walletId = Data(repeating: 0xC1, count: 32)
    private let identityId = Data(repeating: 0xC2, count: 32)
    private let publicKeyData = Data(repeating: 0x02, count: 33)

    private func snapshot(
        keyId: UInt32 = 3,
        totalBudget: UInt64?,
        expiresAt: UInt64?
    ) -> PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot {
        .init(
            identityId: identityId,
            keyId: keyId,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue,
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            readOnly: false,
            disabledAt: nil,
            totalBudget: totalBudget,
            expiresAt: expiresAt,
            publicKeyData: publicKeyData,
            publicKeyHash: Data(repeating: 0x03, count: 20),
            walletId: nil,
            derivationIndices: nil,
            contractBounds: nil
        )
    }

    private func persist(
        _ handler: PlatformWalletPersistenceHandler,
        _ entry: PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot
    ) {
        handler.beginChangeset(walletId: walletId)
        XCTAssertNoThrow(try handler.persistIdentityKeys(walletId: walletId, upserts: [entry], removed: []))
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    private func storedKey(_ container: ModelContainer) throws -> PersistentPublicKey {
        let context = ModelContext(container)
        let rows = try context.fetch(FetchDescriptor<PersistentPublicKey>())
        XCTAssertEqual(rows.count, 1, "one key row per (identity, key id)")
        return try XCTUnwrap(rows.first)
    }

    /// A key registered with both limits persists both, in the units the
    /// protocol uses: credits, and block time in milliseconds.
    func testPersistWritesBothUsageLimits() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        persist(handler, snapshot(totalBudget: 100_000, expiresAt: 1_800_000_000_000))

        let row = try storedKey(container)
        XCTAssertEqual(row.totalBudgetCredits, 100_000)
        XCTAssertEqual(row.expiresAtMillis, 1_800_000_000_000)
        XCTAssertTrue(row.hasLimits)
    }

    /// A key with neither limit is a version 0 key: both columns stay nil, so
    /// nothing downstream mistakes it for a key that may not spend.
    func testPersistLeavesAVersionZeroKeyUnlimited() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        persist(handler, snapshot(totalBudget: nil, expiresAt: nil))

        let row = try storedKey(container)
        XCTAssertNil(row.totalBudget)
        XCTAssertNil(row.expiresAt)
        XCTAssertFalse(row.hasLimits)
    }

    /// What a key limits update emits: the same key id upserted again with a
    /// raised budget and a later expiry. The row must follow, not keep the
    /// values it was first written with: a wallet that showed the old budget
    /// after raising it would refuse work Platform now allows.
    func testUpsertOfTheSameKeyRaisesTheStoredLimits() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        persist(handler, snapshot(totalBudget: 100_000, expiresAt: 1_800_000_000_000))
        persist(handler, snapshot(totalBudget: 350_000, expiresAt: 1_900_000_000_000))

        let row = try storedKey(container)
        XCTAssertEqual(row.totalBudgetCredits, 350_000)
        XCTAssertEqual(row.expiresAtMillis, 1_900_000_000_000)
    }

    /// Load Identity can leave a held identity with its key attached under the
    /// hex identity id and an unlinked Base58-tagged copy of the same key id.
    /// A raise must update the attached row, the one the key screens and cold
    /// restore read, not the orphan.
    func testRaiseUpdatesTheAttachedRowWhateverIdentityStringItCarries() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        seed.insert(identity)
        let attached = PersistentPublicKey(
            keyId: 3, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: publicKeyData,
            totalBudget: 100_000, expiresAt: 1_800_000_000_000,
            identityId: identityId.toHexString())
        seed.insert(attached)
        attached.identity = identity
        identity.addPublicKey(attached)
        let orphan = PersistentPublicKey(
            keyId: 3, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: publicKeyData,
            totalBudget: 100_000, expiresAt: 1_800_000_000_000,
            identityId: identityId.toBase58String())
        seed.insert(orphan)
        try seed.save()

        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        persist(handler, snapshot(totalBudget: 350_000, expiresAt: 1_900_000_000_000))

        let context = ModelContext(container)
        let identityIdData = identityId
        let stored = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == identityIdData })).first)
        XCTAssertEqual(stored.publicKeys.count, 1)
        let row = try XCTUnwrap(stored.publicKeys.first)
        XCTAssertEqual(row.totalBudgetCredits, 350_000)
        XCTAssertEqual(row.expiresAtMillis, 1_900_000_000_000)
        XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentPublicKey>()).count, 2,
                       "no third row, and the orphan is left alone")
    }

    /// A reload can unlink a key row and leave its identity without one. A later
    /// upsert of the key updates that row and attaches it to the identity, so
    /// the key screens and cold restore see it, without adding a second row.
    func testUpsertAttachesAnUnlinkedRowWhenItsIdentityHasNone() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        seed.insert(PersistentIdentity(identityId: identityId, network: .testnet))
        seed.insert(PersistentPublicKey(
            keyId: 3, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: publicKeyData,
            totalBudget: 100_000, identityId: identityId.toBase58String()))
        try seed.save()

        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        persist(handler, snapshot(totalBudget: 350_000, expiresAt: nil))

        let context = ModelContext(container)
        let identityIdData = identityId
        let stored = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == identityIdData })).first)
        XCTAssertEqual(stored.publicKeys.count, 1)
        XCTAssertEqual(stored.publicKeys.first?.totalBudgetCredits, 350_000)
        XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentPublicKey>()).count, 1)
    }

    /// A key lookup that fails rejects the round: the registered C callback
    /// reports the failure, so Rust ends the round unsuccessfully, and nothing
    /// it staged, including a sibling key written before the failure, is
    /// committed.
    func testKeyLookupFailureRejectsTheWholeRound() throws {
        final class FailingSecondKeyLookup: ModelFetching, @unchecked Sendable {
            enum Failure: Error { case read }
            private let lock = NSLock()
            private var keyLookups = 0
            func fetch<T: PersistentModel>(_ descriptor: FetchDescriptor<T>, in context: ModelContext) throws -> [T] {
                if T.self == PersistentPublicKey.self {
                    let count = lock.withLock { () -> Int in
                        keyLookups += 1
                        return keyLookups
                    }
                    if count == 2 { throw Failure.read }
                }
                return try context.fetch(descriptor)
            }
        }
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet, modelFetcher: FailingSecondKeyLookup())
        let callbacks = handler.makeCallbacks()
        defer { Unmanaged<PlatformWalletPersistenceHandler>.fromOpaque(callbacks.context!).release() }
        let persistKeys = try XCTUnwrap(callbacks.on_persist_identity_keys_fn)

        var publicKeyBytes = [UInt8](publicKeyData)
        let rc: Int32 = publicKeyBytes.withUnsafeMutableBufferPointer { publicKeyBuffer in
            func entry(keyId: UInt32, totalBudget: UInt64) -> IdentityKeyEntryFFI {
                var row = IdentityKeyEntryFFI()
                withUnsafeMutableBytes(of: &row.identity_id) { $0.copyBytes(from: identityId) }
                row.key_id = keyId
                row.purpose = KeyPurpose.authentication.rawValue
                row.security_level = SecurityLevel.high.rawValue
                row.key_type = KeyType.ecdsaSecp256k1.rawValue
                row.public_key_data_ptr = publicKeyBuffer.baseAddress
                row.public_key_data_len = UInt(publicKeyBuffer.count)
                row.total_budget_is_some = true
                row.total_budget = totalBudget
                return row
            }
            let rows = [entry(keyId: 3, totalBudget: 100_000), entry(keyId: 4, totalBudget: 200_000)]
            handler.beginChangeset(walletId: walletId)
            return walletId.withUnsafeBytes { walletIdBytes in
                rows.withUnsafeBufferPointer { rowsBuffer in
                    persistKeys(callbacks.context, walletIdBytes.bindMemory(to: UInt8.self).baseAddress,
                                rowsBuffer.baseAddress, UInt(rowsBuffer.count), nil, 0)
                }
            }
        }
        XCTAssertNotEqual(rc, 0, "the callback reports the failed lookup")
        // Rust ends the round by the callback's result.
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: rc == 0))

        XCTAssertTrue(try ModelContext(container).fetch(FetchDescriptor<PersistentPublicKey>()).isEmpty)
    }

    /// Rust is the source of truth on every round, so a key re-emitted without
    /// limits clears the columns rather than leaving stale ones behind.
    func testUpsertWithoutLimitsClearsTheStoredOnes() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)

        persist(handler, snapshot(totalBudget: 100_000, expiresAt: 1_800_000_000_000))
        persist(handler, snapshot(totalBudget: nil, expiresAt: nil))

        let row = try storedKey(container)
        XCTAssertNil(row.totalBudget)
        XCTAssertNil(row.expiresAt)
    }

    /// The projection the rest of the SDK reads a stored key through, both
    /// ways: a limited key must not come back unlimited from either direction.
    func testIdentityPublicKeyProjectionRoundTripsTheLimits() throws {
        let key = IdentityPublicKey(
            id: 3,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            readOnly: false,
            data: publicKeyData,
            totalBudget: 100_000,
            expiresAt: 1_800_000_000_000
        )
        XCTAssertTrue(key.hasLimits)
        XCTAssertTrue(key.isExpired(at: TimestampMillis(1_800_000_000_000)))
        XCTAssertFalse(key.isExpired(at: TimestampMillis(1_799_999_999_999)))

        let row = try XCTUnwrap(PersistentPublicKey.from(key, identityId: "identity"))
        XCTAssertEqual(row.totalBudgetCredits, 100_000)
        XCTAssertEqual(row.expiresAtMillis, 1_800_000_000_000)

        let projected = try XCTUnwrap(row.toIdentityPublicKey())
        XCTAssertEqual(projected.totalBudget, 100_000)
        XCTAssertEqual(projected.expiresAt, 1_800_000_000_000)

        // And a key with neither limit never claims one.
        let unlimited = IdentityPublicKey(
            id: 4,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            readOnly: false,
            data: publicKeyData
        )
        XCTAssertFalse(unlimited.hasLimits)
        XCTAssertFalse(unlimited.isExpired(at: TimestampMillis.max))
        let unlimitedRow = try XCTUnwrap(
            PersistentPublicKey.from(unlimited, identityId: "identity"))
        XCTAssertNil(unlimitedRow.totalBudget)
        XCTAssertNil(unlimitedRow.expiresAt)
        XCTAssertNil(try XCTUnwrap(unlimitedRow.toIdentityPublicKey()).totalBudget)
    }

    /// `IdentityPublicKey` still round-trips through its synthesized `Codable`
    /// with the two limits on it, and a payload carrying the
    /// `"$formatVersion"` tag a version 1 key is written with still decodes:
    /// the model does not declare that key, and an undeclared key must stay
    /// ignored rather than failing the decode.
    func testCodableCarriesTheLimitsAndIgnoresTheFormatVersionTag() throws {
        let key = IdentityPublicKey(
            id: 3,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            readOnly: false,
            data: publicKeyData,
            totalBudget: 100_000,
            expiresAt: 1_800_000_000_000
        )
        let encoded = try JSONEncoder().encode(key)
        XCTAssertEqual(try JSONDecoder().decode(IdentityPublicKey.self, from: encoded), key)

        var object = try XCTUnwrap(
            JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        XCTAssertEqual(object["totalBudget"] as? UInt64, 100_000)
        XCTAssertEqual(object["expiresAt"] as? UInt64, 1_800_000_000_000)
        object["$formatVersion"] = "1"
        let tagged = try JSONSerialization.data(withJSONObject: object)
        XCTAssertEqual(try JSONDecoder().decode(IdentityPublicKey.self, from: tagged), key)

        // A version 0 key writes neither field, and decodes to no limits.
        let unlimited = IdentityPublicKey(
            id: 4,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            readOnly: false,
            data: publicKeyData
        )
        let unlimitedObject = try XCTUnwrap(
            JSONSerialization.jsonObject(with: try JSONEncoder().encode(unlimited))
                as? [String: Any])
        var withTag = unlimitedObject
        withTag["$formatVersion"] = "0"
        let decoded = try JSONDecoder().decode(
            IdentityPublicKey.self,
            from: try JSONSerialization.data(withJSONObject: withTag))
        XCTAssertNil(decoded.totalBudget)
        XCTAssertNil(decoded.expiresAt)
        XCTAssertFalse(decoded.hasLimits)
    }
}
