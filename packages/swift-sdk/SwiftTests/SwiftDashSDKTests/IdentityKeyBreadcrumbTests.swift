import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Coverage for the derive-sign-destroy migration pieces that the funded UAT
/// could not exercise: the Keychain-driven breadcrumb backfill's three
/// outcomes (the `failed` count is the signal the scalar-deletion gate reads),
/// and the signer's `(walletId, derivationPath)` resolution incl. its
/// `wid.count == 32` guard.
@MainActor
final class IdentityKeyBreadcrumbTests: XCTestCase {

    private let walletId = Data(repeating: 0xAB, count: 32)

    private func makeRow(
        keyId: Int32,
        publicKeyData: Data,
        identityId: String,
        walletId: Data? = nil,
        derivationPath: String? = nil,
        keychainId: String? = nil
    ) -> PersistentPublicKey {
        let row = PersistentPublicKey(
            keyId: keyId,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData,
            identityId: identityId
        )
        row.walletId = walletId
        row.identityDerivationPath = derivationPath
        row.privateKeyKeychainIdentifier = keychainId
        return row
    }

    private func meta(
        keyId: UInt32,
        publicKey: Data,
        identityIndex: UInt32,
        derivationPath: String
    ) -> KeychainManager.IdentityPrivateKeyMetadata {
        KeychainManager.IdentityPrivateKeyMetadata(
            identityId: "id1",
            keyId: keyId,
            walletId: walletId.toHexString(),
            identityIndex: identityIndex,
            keyIndex: keyId,
            derivationPath: derivationPath,
            publicKey: publicKey.toHexString(),
            publicKeyHash: "",
            keyType: 0,
            purpose: 0,
            securityLevel: 2
        )
    }

    // MARK: - Backfill outcomes (the deletion-gate signal)

    /// A metadata item whose stored path equals the canonical DIP-9 path for
    /// its indices populates the breadcrumb columns and counts as `written`.
    func testBackfillWritesBreadcrumbWhenPathIsCanonical() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let pubKey = Data(repeating: 0x11, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            keychainId: "identity_privkey.x"))
        try seed.save()

        // No PersistentWallet row → handler resolves network to .testnet, so
        // build the canonical path on .testnet to match.
        let canonical = try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: 3, keyIndex: 0)

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let result = handler.backfillIdentityKeyBreadcrumbs(
            walletId: walletId,
            items: [meta(keyId: 0, publicKey: pubKey, identityIndex: 3, derivationPath: canonical)]
        )

        XCTAssertEqual(result.written, 1)
        XCTAssertEqual(result.failed, 0)
        XCTAssertEqual(result.skipped, 0)

        let fresh = ModelContext(container)
        let row = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentPublicKey>()).first)
        XCTAssertEqual(row.identityDerivationPath, canonical)
        XCTAssertEqual(row.walletId, walletId)
    }

    /// A path that is NOT the canonical DIP-9 path for its indices fails the
    /// self-check: the column is left nil and the row is counted `failed` —
    /// this is exactly the count the deletion gate must read as zero.
    func testBackfillCountsFailedAndLeavesColumnNilOnPathDrift() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let pubKey = Data(repeating: 0x22, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            keychainId: "identity_privkey.x"))
        try seed.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let result = handler.backfillIdentityKeyBreadcrumbs(
            walletId: walletId,
            items: [meta(keyId: 0, publicKey: pubKey, identityIndex: 3,
                        derivationPath: "m/9'/1'/5'/0'/0'/999'/0'")]  // not canonical for index 3
        )

        XCTAssertEqual(result.failed, 1)
        XCTAssertEqual(result.written, 0)

        let fresh = ModelContext(container)
        let row = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentPublicKey>()).first)
        XCTAssertNil(row.identityDerivationPath, "a drifted path must not be written")
    }

    /// A row that already carries a path is skipped (idempotent), not rewritten
    /// or counted as failed.
    func testBackfillIsIdempotentForAlreadyMigratedRow() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let pubKey = Data(repeating: 0x33, count: 33)
        let existing = "m/9'/1'/5'/0'/0'/3'/0'"
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            walletId: walletId, derivationPath: existing,
                            keychainId: "identity_privkey.x"))
        try seed.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let result = handler.backfillIdentityKeyBreadcrumbs(
            walletId: walletId,
            items: [meta(keyId: 0, publicKey: pubKey, identityIndex: 3, derivationPath: existing)]
        )

        XCTAssertEqual(result.skipped, 1)
        XCTAssertEqual(result.written, 0)
        XCTAssertEqual(result.failed, 0)
    }

    // MARK: - Signer context resolution

    /// A row with a 32-byte walletId + a non-empty path resolves to that
    /// breadcrumb; one without a breadcrumb, and one with a non-32-byte
    /// walletId, both resolve to nil (so the signer falls back to the scalar).
    func testResolveIdentityKeyContextHonorsBreadcrumbAndWidGuard() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let path = "m/9'/1'/5'/0'/0'/0'/0'"

        let good = Data(repeating: 0x44, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: good, identityId: "id1",
                            walletId: walletId, derivationPath: path))
        let noCrumb = Data(repeating: 0x55, count: 33)
        seed.insert(makeRow(keyId: 1, publicKeyData: noCrumb, identityId: "id1"))
        let badWid = Data(repeating: 0x66, count: 33)
        seed.insert(makeRow(keyId: 2, publicKeyData: badWid, identityId: "id1",
                            walletId: Data(repeating: 0x01, count: 16), derivationPath: path))
        try seed.save()

        let signer = KeychainSigner(modelContainer: container, network: .testnet)

        let resolved = signer.resolveIdentityKeyContext(publicKey: good)
        XCTAssertEqual(resolved?.walletId, walletId)
        XCTAssertEqual(resolved?.derivationPath, path)

        XCTAssertNil(signer.resolveIdentityKeyContext(publicKey: noCrumb),
                     "no breadcrumb → nil → caller falls back to the stored scalar")
        XCTAssertNil(signer.resolveIdentityKeyContext(publicKey: badWid),
                     "non-32-byte walletId must not resolve (FFI reads 32 bytes)")
    }
}
