import Security
import XCTest

@testable import SwiftDashSDK

@MainActor
final class KeychainManagerStoreKeyDataTests: XCTestCase {
    func testReplacingAKeyUpdatesTheOneItemInPlaceWithTheDeviceOnlyClass() throws {
        let serviceName = "org.dashfoundation.tests.\(UUID().uuidString)"
        let keychain = KeychainManager(serviceName: serviceName)
        let identifier = "specialkey_test_voting"
        addTeardownBlock { @MainActor in
            _ = keychain.deleteKeyData(identifier: identifier)
        }

        let first = Data(repeating: 0x01, count: 32)
        let second = Data(repeating: 0x02, count: 32)
        XCTAssertEqual(keychain.storeKeyData(first, identifier: identifier), identifier)
        XCTAssertEqual(try keychain.readKeyData(identifier: identifier).get(), first)

        XCTAssertEqual(keychain.storeKeyData(second, identifier: identifier), identifier)
        XCTAssertEqual(try keychain.readKeyData(identifier: identifier).get(), second)

        // Still exactly one item, with the device-only protection class.
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: identifier,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
        ]
        var found: AnyObject?
        XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &found), errSecSuccess)
        let items = try XCTUnwrap(found as? [[String: Any]])
        XCTAssertEqual(items.count, 1)
        XCTAssertEqual(
            items[0][kSecAttrAccessible as String] as? String,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String
        )
    }
}
