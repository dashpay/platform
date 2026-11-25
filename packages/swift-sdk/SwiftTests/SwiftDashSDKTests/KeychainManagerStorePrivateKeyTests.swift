import XCTest

@testable import SwiftDashSDK

@MainActor
final class KeychainManagerStorePrivateKeyTests: XCTestCase {
    /// A retried import stores the same key into a slot it already filled:
    /// that must succeed with the same identifier, not fail as a duplicate.
    func testStoringTheSameKeyAgainSucceeds() {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x41, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deletePrivateKey(identityId: identityId, keyIndex: 1)
        }

        let key = Data(repeating: 0x05, count: 32)
        let first = keychain.storePrivateKey(key, identityId: identityId, keyIndex: 1)
        XCTAssertNotNil(first)
        let second = keychain.storePrivateKey(key, identityId: identityId, keyIndex: 1)
        XCTAssertEqual(second, first)
        XCTAssertEqual(keychain.retrievePrivateKey(identityId: identityId, keyIndex: 1), key)
    }

    /// The slot holds one key per identity and key index; storing a key
    /// into it replaces what was there.
    func testStoringAnotherKeyReplacesTheStoredOne() {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x42, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deletePrivateKey(identityId: identityId, keyIndex: 2)
        }

        let stale = Data(repeating: 0x06, count: 32)
        let current = Data(repeating: 0x07, count: 32)
        XCTAssertNotNil(keychain.storePrivateKey(stale, identityId: identityId, keyIndex: 2))
        XCTAssertNotNil(keychain.storePrivateKey(current, identityId: identityId, keyIndex: 2))
        XCTAssertEqual(keychain.retrievePrivateKey(identityId: identityId, keyIndex: 2), current)
    }
}
