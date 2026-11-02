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
        handler.persistIdentityKeys(walletId: walletId, upserts: [entry], removed: [])
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
