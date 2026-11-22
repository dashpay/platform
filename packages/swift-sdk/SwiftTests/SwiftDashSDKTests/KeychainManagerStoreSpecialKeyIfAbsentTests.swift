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
}
