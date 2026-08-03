import SwiftData
import XCTest

@testable import SwiftDashSDK

@MainActor
final class KeychainSignerAdditionalSigningKeysTests: XCTestCase {
    private enum ScopeError: Swift.Error {
        case expected
    }

    func testAdditionalHash160KeyIsSignableWithinScopeAndUnavailableAfterwards() async throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)

        let privateKey = Data(repeating: 0x11, count: 32)
        let publicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: privateKey)
        let publicKeyHash = try XCTUnwrap(
            Data(hexString: KeychainManager.computePublicKeyHashHex(publicKey))
        )

        try await signer.withAdditionalSigningKeys([
            (publicKey: publicKeyHash, privateKey: privateKey),
        ]) {
            XCTAssertTrue(
                signer.canSign(publicKey: publicKeyHash, keyType: KeyType.ecdsaHash160.rawValue)
            )

            let result = signer.signOnDemand(
                publicKey: publicKeyHash,
                keyType: KeyType.ecdsaHash160.rawValue,
                data: Data("proof-of-possession".utf8)
            )

            guard case .success(let signature) = result else {
                return XCTFail("expected signature, got \(result)")
            }
            XCTAssertEqual(signature.count, 65)
        }

        XCTAssertFalse(
            signer.canSign(publicKey: publicKeyHash, keyType: KeyType.ecdsaHash160.rawValue)
        )
    }

    func testAdditionalSigningKeysAreZeroedAfterThrowingScope() async throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)

        let privateKey = Data(repeating: 0x12, count: 32)
        let publicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: privateKey)
        let entries = signer.makeAdditionalSigningKeyEntries([(publicKey: publicKey, privateKey: privateKey)])

        XCTAssertFalse(entries[0].isZeroedForTesting)

        do {
            _ = try await signer.withAdditionalSigningKeys(entries) {
                throw ScopeError.expected
            }
            XCTFail("expected throwing scope")
        } catch ScopeError.expected {
            XCTAssertTrue(entries[0].isZeroedForTesting)
            XCTAssertFalse(
                signer.canSign(publicKey: publicKey, keyType: KeyType.ecdsaSecp256k1.rawValue)
            )
        }
    }

    func testWalletOwnedKeyStillSignsThroughPersistence() throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)

        let persisted = try seedPersistedIdentityKey(
            container: container,
            keychain: keychain,
            privateKey: Data(repeating: 0x13, count: 32)
        )
        defer {
            _ = keychain.deleteIdentityPrivateKey(
                walletId: persisted.walletId,
                derivationPath: persisted.derivationPath
            )
        }

        XCTAssertTrue(
            signer.canSign(publicKey: persisted.publicKey, keyType: KeyType.ecdsaSecp256k1.rawValue)
        )

        let result = signer.signOnDemand(
            publicKey: persisted.publicKey,
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            data: Data("wallet-owned".utf8)
        )

        guard case .success(let signature) = result else {
            return XCTFail("expected persisted key signature, got \(result)")
        }
        XCTAssertEqual(signature.count, 65)
    }

    func testAdditionalRegistryDoesNotChangeWalletOwnedBehaviorWhenBytesMatch() async throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)

        let persisted = try seedPersistedIdentityKey(
            container: container,
            keychain: keychain,
            privateKey: Data(repeating: 0x14, count: 32)
        )
        defer {
            _ = keychain.deleteIdentityPrivateKey(
                walletId: persisted.walletId,
                derivationPath: persisted.derivationPath
            )
        }

        let message = Data("same-bytes".utf8)
        let baseline = signer.signOnDemand(
            publicKey: persisted.publicKey,
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            data: message
        )

        guard case .success(let baselineSignature) = baseline else {
            return XCTFail("expected baseline signature, got \(baseline)")
        }

        try await signer.withAdditionalSigningKeys([
            (publicKey: persisted.publicKey, privateKey: persisted.privateKey),
        ]) {
            let scoped = signer.signOnDemand(
                publicKey: persisted.publicKey,
                keyType: KeyType.ecdsaSecp256k1.rawValue,
                data: message
            )

            guard case .success(let scopedSignature) = scoped else {
                return XCTFail("expected scoped signature, got \(scoped)")
            }
            XCTAssertEqual(scopedSignature, baselineSignature)
        }
    }

    private func seedPersistedIdentityKey(
        container: ModelContainer,
        keychain: KeychainManager,
        privateKey: Data
    ) throws -> (publicKey: Data, privateKey: Data, walletId: Data, derivationPath: String) {
        let publicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: privateKey)
        let walletId = Data(repeating: 0xAB, count: 32)
        let derivationPath = "m/9'/1'/5'/0'/0'/0'/0'"
        let metadata = IdentityPrivateKeyMetadata(
            identityId: "id1",
            keyId: 0,
            walletId: walletId.toHexString(),
            identityIndex: 0,
            keyIndex: 0,
            derivationPath: derivationPath,
            publicKey: publicKey.toHexString(),
            publicKeyHash: KeychainManager.computePublicKeyHashHex(publicKey),
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue
        )
        let identifier = try XCTUnwrap(
            keychain.storeIdentityPrivateKey(
                privateKey,
                derivationPath: derivationPath,
                metadata: metadata
            )
        )

        let context = ModelContext(container)
        let row = PersistentPublicKey(
            keyId: 0,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKey,
            identityId: "id1"
        )
        row.privateKeyKeychainIdentifier = identifier
        context.insert(row)
        try context.save()

        return (publicKey, privateKey, walletId, derivationPath)
    }
}
