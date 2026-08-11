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
        keychainId: String? = nil,
        keyType: KeyType = .ecdsaSecp256k1
    ) -> PersistentPublicKey {
        let row = PersistentPublicKey(
            keyId: keyId,
            purpose: .authentication,
            securityLevel: .high,
            keyType: keyType,
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

    /// Regression: a breadcrumb backfill that lands while a Rust persister
    /// round is open (between `beginChangeset` and `endChangeset`) must NOT
    /// commit the round's half-applied writes early — the backfill's own
    /// `save()` would otherwise flush the staged (uncommitted) round rows,
    /// so a later `endChangeset(success: false)` could no longer roll the
    /// round back cleanly. The deferred write must still complete once the
    /// round closes, so the breadcrumb is not silently dropped.
    func testBackfillDefersDuringOpenChangesetRoundAndCompletesAfter() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        // Owner identity so the mid-round `persistDashpayPayments` write has a
        // row to attach to; the backfill target key is matched by pubkey.
        let ownerId = Data(repeating: 0x01, count: 32)
        let counterpartyId = Data(repeating: 0x02, count: 32)
        seed.insert(PersistentIdentity(identityId: ownerId, network: .testnet))
        let pubKey = Data(repeating: 0x77, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            keychainId: "identity_privkey.x"))
        try seed.save()

        let canonical = try KeyDerivation.getIdentityAuthenticationPath(
            network: .testnet, identityIndex: 3, keyIndex: 0)

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)

        // Open a round and stage an (uncommitted) payment write.
        handler.beginChangeset(walletId: walletId)
        handler.persistDashpayPayments(
            ownerIdentityId: ownerId,
            payments: [
                DashPayPayment(
                    counterpartyId: counterpartyId,
                    amountDuffs: 1_000,
                    direction: .sent,
                    status: .pending,
                    txid: "0011223344556677"
                )
            ]
        )

        // Backfill lands mid-round. It must defer: no breadcrumb written yet,
        // and — critically — nothing flushed to disk.
        let midRound = handler.backfillIdentityKeyBreadcrumbs(
            walletId: walletId,
            items: [meta(keyId: 0, publicKey: pubKey, identityIndex: 3, derivationPath: canonical)]
        )
        XCTAssertEqual(midRound.written, 0, "a mid-round backfill must defer, not write")

        // Neither the round's staged payment nor the backfill's breadcrumb may
        // be visible from another context while the round is open.
        let midContext = ModelContext(container)
        XCTAssertEqual(
            try midContext.fetch(FetchDescriptor<PersistentDashpayPayment>()).count, 0,
            "a mid-round backfill must not flush the open changeset early"
        )
        let midRow = try XCTUnwrap(
            midContext.fetch(FetchDescriptor<PersistentPublicKey>()).first
        )
        XCTAssertNil(
            midRow.identityDerivationPath,
            "the backfill breadcrumb must not be written during the open round"
        )

        // Fail the round — the staged payment must roll back, yet the deferred
        // backfill (which never rode the round's transaction) must still
        // complete once the round closes.
        handler.endChangeset(walletId: walletId, success: false)

        let afterContext = ModelContext(container)
        XCTAssertEqual(
            try afterContext.fetch(FetchDescriptor<PersistentDashpayPayment>()).count, 0,
            "the failed round's staged payment must roll back"
        )
        let afterRow = try XCTUnwrap(
            afterContext.fetch(FetchDescriptor<PersistentPublicKey>()).first
        )
        XCTAssertEqual(
            afterRow.identityDerivationPath, canonical,
            "the deferred backfill must still complete after the round closes"
        )
        XCTAssertEqual(afterRow.walletId, walletId)
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

    // MARK: - canSign preflight ⇄ sign-time consistency

    /// A breadcrumb-only row (no stored scalar) whose key type the resolver
    /// does NOT derive-sign must NOT be reported signable: at sign time the
    /// resolver returns `UNSUPPORTED_KEY_TYPE`, routing to the (absent) stored
    /// scalar and failing with `publicKeyNotFound`. Preflight must agree.
    func testCanSignRejectsBreadcrumbOnlyNonEcdsaKeyType() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let path = "m/9'/1'/5'/0'/0'/0'/0'"
        // Breadcrumb present, no `privateKeyKeychainIdentifier` (derive-only).
        let pubKey = Data(repeating: 0x88, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            walletId: walletId, derivationPath: path,
                            keyType: .eddsa25519Hash160))
        try seed.save()

        let signer = KeychainSigner(modelContainer: container, network: .testnet)

        // Non-ECDSA type: the resolver-derivable gate short-circuits before the
        // Keychain mnemonic read, so this is deterministic without a mnemonic.
        XCTAssertFalse(
            signer.canSign(publicKey: pubKey, keyType: KeyType.eddsa25519Hash160.rawValue),
            "a breadcrumb-only non-ECDSA key must not preflight as signable"
        )
        XCTAssertFalse(
            signer.canSign(publicKey: pubKey, keyType: KeyType.bls12_381.rawValue),
            "a breadcrumb-only BLS key must not preflight as signable"
        )
    }

    /// The stored-scalar branch is key-type independent (the scalar signs via
    /// `ffiSign` regardless of the declared type), so a row carrying a keychain
    /// identifier preflights as signable for any key type — the resolver gate
    /// only governs the breadcrumb-only path. Deterministic: no mnemonic read.
    func testCanSignAcceptsStoredScalarRegardlessOfKeyType() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let pubKey = Data(repeating: 0x99, count: 33)
        seed.insert(makeRow(keyId: 0, publicKeyData: pubKey, identityId: "id1",
                            keychainId: "identity_privkey.x",
                            keyType: .eddsa25519Hash160))
        try seed.save()

        let signer = KeychainSigner(modelContainer: container, network: .testnet)

        XCTAssertTrue(
            signer.canSign(publicKey: pubKey, keyType: KeyType.ecdsaSecp256k1.rawValue),
            "a stored-scalar row is signable for an ECDSA type"
        )
        XCTAssertTrue(
            signer.canSign(publicKey: pubKey, keyType: KeyType.eddsa25519Hash160.rawValue),
            "a stored-scalar row is signable regardless of key type"
        )
    }

    /// The Swift preflight wrapper forwards to the resolver's FFI predicate
    /// `dash_sdk_resolver_supports_key_type`, which reports exactly the
    /// wallet-derivable ECDSA key types (`ECDSA_SECP256K1 = 0`,
    /// `ECDSA_HASH160 = 2`).
    func testResolverCanDeriveSignMatchesRustSupportedSet() {
        XCTAssertTrue(KeychainSigner.resolverCanDeriveSign(keyType: 0))  // ECDSA_SECP256K1
        XCTAssertTrue(KeychainSigner.resolverCanDeriveSign(keyType: 2))  // ECDSA_HASH160
        XCTAssertFalse(KeychainSigner.resolverCanDeriveSign(keyType: 1)) // BLS12_381
        XCTAssertFalse(KeychainSigner.resolverCanDeriveSign(keyType: 3)) // BIP13_SCRIPT_HASH
        XCTAssertFalse(KeychainSigner.resolverCanDeriveSign(keyType: 4)) // EDDSA_25519_HASH160
    }
}
