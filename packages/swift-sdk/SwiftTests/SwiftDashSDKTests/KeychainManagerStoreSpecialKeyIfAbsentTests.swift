import XCTest

@testable import SwiftDashSDK

@MainActor
final class KeychainManagerStoreSpecialKeyIfAbsentTests: XCTestCase {
    func testAStoredKeyIsNeverReplaced() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x31, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let first = Data(repeating: 0x01, count: 32)
        let second = Data(repeating: 0x02, count: 32)

        guard case .stored(let identifier) = keychain.storeSpecialKeyIfAbsent(
            first, identityId: identityId, keyType: .voting
        ) else {
            return XCTFail("an empty slot stores the key")
        }
        XCTAssertEqual(
            keychain.storeSpecialKeyIfAbsent(first, identityId: identityId, keyType: .voting),
            .alreadyStored(identifier: identifier)
        )
        XCTAssertEqual(
            keychain.storeSpecialKeyIfAbsent(second, identityId: identityId, keyType: .voting),
            .conflicting
        )
        // The refused key left the stored one as it was.
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            first
        )
    }

    /// A slot filled by a load whose identity save failed has no row
    /// reference; deleting the slot by identity id clears the conflict.
    func testDeletingTheSlotByIdentityIdClearsAConflict() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x32, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let stale = Data(repeating: 0x03, count: 32)
        let correct = Data(repeating: 0x04, count: 32)
        guard case .stored = keychain.storeSpecialKeyIfAbsent(stale, identityId: identityId, keyType: .voting) else {
            return XCTFail("an empty slot stores the key")
        }
        XCTAssertEqual(
            keychain.storeSpecialKeyIfAbsent(correct, identityId: identityId, keyType: .voting),
            .conflicting
        )

        XCTAssertTrue(keychain.deleteSpecialKey(identityId: identityId, keyType: .voting))
        guard case .stored = keychain.storeSpecialKeyIfAbsent(correct, identityId: identityId, keyType: .voting) else {
            return XCTFail("the cleared slot stores the new key")
        }
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            correct
        )
    }
}
