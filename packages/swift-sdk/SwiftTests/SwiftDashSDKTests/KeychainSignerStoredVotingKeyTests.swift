import SwiftData
import XCTest

@testable import SwiftDashSDK

@MainActor
final class KeychainSignerStoredVotingKeyTests: XCTestCase {
    func testStoredVotingKeyIsSignableUnderItsAddressWithinScopeOnly() async throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)

        let identityId = Data(repeating: 0x21, count: 32)
        let privateKey = Data(repeating: 0x14, count: 32)
        XCTAssertNotNil(keychain.storeSpecialKey(privateKey, identityId: identityId, keyType: .voting))
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let publicKey = try Secp256k1Primitives.compressedPublicKey(privateKey: privateKey)
        let expectedAddress = try XCTUnwrap(
            Data(hexString: KeychainManager.computePublicKeyHashHex(publicKey))
        )
        XCTAssertEqual(expectedAddress.count, 20)

        let address = try await signer.withStoredMasternodeVotingKey(identityId: identityId) { votingAddress in
            XCTAssertEqual(votingAddress, expectedAddress)
            XCTAssertTrue(
                signer.canSign(publicKey: votingAddress, keyType: KeyType.ecdsaHash160.rawValue)
            )
            // The signer signs with the stored key, not only claims it can.
            let payload = Data("stored voting key payload".utf8)
            let signature = try signer.signOnDemand(
                publicKey: votingAddress,
                keyType: KeyType.ecdsaHash160.rawValue,
                data: payload
            ).get()
            XCTAssertEqual(
                signature,
                try RawKeySigner.sign(data: payload, privateKey: privateKey, network: .testnet)
            )
            return votingAddress
        }

        XCTAssertEqual(address, expectedAddress)
        XCTAssertFalse(
            signer.canSign(publicKey: expectedAddress, keyType: KeyType.ecdsaHash160.rawValue)
        )
    }

    func testMissingVotingKeyThrowsVotingKeyNotStored() async throws {
        let container = try DashModelContainer.createInMemory()
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let signer = KeychainSigner(modelContainer: container, network: .testnet, keychain: keychain)
        let identityId = Data(repeating: 0x22, count: 32)

        do {
            _ = try await signer.withStoredMasternodeVotingKey(identityId: identityId) { _ in
                XCTFail("the body must not run without a stored key")
            }
            XCTFail("expected votingKeyNotStored")
        } catch KeychainSigner.Error.votingKeyNotStored(let identityIdHex) {
            XCTAssertEqual(identityIdHex, identityId.toHexString())
        }
    }
}
