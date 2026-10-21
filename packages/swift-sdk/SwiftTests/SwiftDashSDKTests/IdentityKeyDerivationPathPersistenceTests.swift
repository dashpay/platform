import SwiftData
import XCTest

@testable import SwiftDashSDK

/// What a persister round does to a stored key's derivation breadcrumb
/// (`identityDerivationPath`) and private-key linkage
/// (`privateKeyKeychainIdentifier`) when the upsert carries no derivation
/// indices of its own.
///
/// The rules `persistIdentityKeys` follows:
/// - no breadcrumb and no rejection: the key's origin is unknown, and the
///   stored path of the same public key is kept;
/// - `derivationRejected` clears the stored path only when the row belongs to
///   the persisting wallet (a legacy row without a wallet has an unknown owner
///   and keeps its path) and
///   the held path is the one at a rejected `(identityIndex, keyIndex)` pair;
/// - another public key under the same key id clears both the path and the
///   keychain linkage, which belonged to the old key.
@MainActor
final class IdentityKeyDerivationPathPersistenceTests: XCTestCase {

    private let walletId = Data(repeating: 0xD1, count: 32)
    private let otherWalletId = Data(repeating: 0xD2, count: 32)
    private let identityId = Data(repeating: 0xD3, count: 32)
    private let publicKeyData = Data(repeating: 0x02, count: 33)
    private let keychainId = "identity_privkey.stored"
    private let keyId: UInt32 = 0

    /// The DIP-9 identity-auth path the handler formats for the persisting
    /// wallet, which lives on testnet.
    private func authPath(identityIndex: UInt32, keyIndex: UInt32) throws -> String {
        try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: identityIndex, keyIndex: keyIndex)
    }

    /// An in-memory store holding the persisting wallet's row (the handler
    /// needs it to format a rejected pair into a path) and one key row that
    /// already carries a breadcrumb and a keychain linkage.
    private func makeStore(
        rowWalletId: Data?,
        storedPath: String
    ) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let row = PersistentPublicKey(
            keyId: Int32(bitPattern: keyId),
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData,
            identityId: identityId.toBase58String())
        row.walletId = rowWalletId
        row.identityDerivationPath = storedPath
        row.privateKeyKeychainIdentifier = keychainId
        context.insert(row)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// An upsert of the stored key with no derivation indices, as Rust emits
    /// it for a key whose origin it could not confirm.
    private func snapshot(
        publicKey: Data? = nil,
        rejected: [(identityIndex: UInt32, keyIndex: UInt32)] = []
    ) -> PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot {
        .init(
            identityId: identityId,
            keyId: keyId,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue,
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            readOnly: false,
            disabledAt: nil,
            totalBudget: nil,
            expiresAt: nil,
            publicKeyData: publicKey ?? publicKeyData,
            publicKeyHash: Data(repeating: 0x03, count: 20),
            walletId: nil,
            derivationIndices: nil,
            contractBounds: nil,
            derivationRejected: rejected)
    }

    /// One persist round, bracketed the way the Rust store() round is:
    /// `persistIdentityKeys` stages rows and `endChangeset` saves them.
    private func persist(
        _ handler: PlatformWalletPersistenceHandler,
        _ entry: PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentityKeys(walletId: walletId, upserts: [entry], removed: [])
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    private func storedKey(_ container: ModelContainer) throws -> PersistentPublicKey {
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentPublicKey>())
        XCTAssertEqual(rows.count, 1, "one key row per (identity, key id)")
        return try XCTUnwrap(rows.first)
    }

    /// No breadcrumb and no rejection: Rust does not know where the key comes
    /// from, which refutes nothing, so the stored path and linkage stay.
    func testUnknownOriginKeepsTheStoredPath() throws {
        let path = try authPath(identityIndex: 0, keyIndex: 0)
        let (handler, container) = try makeStore(rowWalletId: walletId, storedPath: path)

        persist(handler, snapshot())

        let row = try storedKey(container)
        XCTAssertEqual(row.identityDerivationPath, path)
        XCTAssertEqual(row.privateKeyKeychainIdentifier, keychainId)
        XCTAssertEqual(row.walletId, walletId)
    }

    /// The rejection was checked under the persisting wallet's seed. A path
    /// another wallet derived is not refuted by it, even when it names the
    /// same coordinates (the path string carries no wallet).
    func testRejectionByAnotherWalletKeepsTheStoredPath() throws {
        let path = try authPath(identityIndex: 0, keyIndex: 0)
        let (handler, container) = try makeStore(rowWalletId: otherWalletId, storedPath: path)

        persist(handler, snapshot(rejected: [(identityIndex: 0, keyIndex: 0)]))

        let row = try storedKey(container)
        XCTAssertEqual(row.identityDerivationPath, path,
                       "another wallet's path is not refuted by this wallet's check")
        XCTAssertEqual(row.privateKeyKeychainIdentifier, keychainId)
        XCTAssertEqual(row.walletId, otherWalletId)
    }

    /// The same wallet rejecting other coordinates says nothing about the
    /// path the row holds: it stays.
    func testRejectionAtOtherCoordinatesKeepsTheStoredPath() throws {
        let path = try authPath(identityIndex: 0, keyIndex: 0)
        let (handler, container) = try makeStore(rowWalletId: walletId, storedPath: path)

        persist(handler, snapshot(rejected: [
            (identityIndex: 0, keyIndex: 1),
            (identityIndex: 1, keyIndex: 0),
        ]))

        let row = try storedKey(container)
        XCTAssertEqual(row.identityDerivationPath, path,
                       "a rejection at other coordinates must not clear this path")
        XCTAssertEqual(row.privateKeyKeychainIdentifier, keychainId)
    }

    /// The persisting wallet rejecting the coordinates the stored path names:
    /// the path is a stale inferred one and is cleared, so the signer does not
    /// derive at it. The public key is unchanged, so its keychain linkage
    /// stays.
    func testMatchingRejectionBySameWalletClearsTheStoredPath() throws {
        let path = try authPath(identityIndex: 0, keyIndex: 0)
        let (handler, container) = try makeStore(rowWalletId: walletId, storedPath: path)

        persist(handler, snapshot(rejected: [
            (identityIndex: 0, keyIndex: 1),
            (identityIndex: 0, keyIndex: 0),
        ]))

        let row = try storedKey(container)
        XCTAssertNil(row.identityDerivationPath,
                     "a path at rejected coordinates of this wallet must be cleared")
        XCTAssertEqual(row.privateKeyKeychainIdentifier, keychainId)
    }

    /// A legacy row with no wallet recorded has an unknown owner, which the
    /// persisting wallet's rejection does not refute: its path is kept, as
    /// the Kotlin persister keeps it.
    func testMatchingRejectionKeepsTheStoredPathOfARowWithoutAWallet() throws {
        let path = try authPath(identityIndex: 2, keyIndex: 3)
        let (handler, container) = try makeStore(rowWalletId: nil, storedPath: path)

        persist(handler, snapshot(rejected: [(identityIndex: 2, keyIndex: 3)]))

        let row = try storedKey(container)
        XCTAssertEqual(row.identityDerivationPath, path,
                       "a path whose owner is unknown must not be cleared by a rejection")
    }

    /// Another public key under the same key id: the stored path and keychain
    /// linkage belonged to the old key and must not pass to the new one, even
    /// with no rejection on the upsert.
    func testReplacementPublicKeyClearsPathAndKeychainLinkage() throws {
        let path = try authPath(identityIndex: 0, keyIndex: 0)
        let (handler, container) = try makeStore(rowWalletId: walletId, storedPath: path)
        let replacement = Data(repeating: 0x03, count: 33)

        persist(handler, snapshot(publicKey: replacement))

        let row = try storedKey(container)
        XCTAssertEqual(row.publicKeyData, replacement)
        XCTAssertNil(row.identityDerivationPath,
                     "the old key's derivation path must not sign for the new key")
        XCTAssertNil(row.privateKeyKeychainIdentifier,
                     "the old key's private-key linkage must not pass to the new key")
    }
}
