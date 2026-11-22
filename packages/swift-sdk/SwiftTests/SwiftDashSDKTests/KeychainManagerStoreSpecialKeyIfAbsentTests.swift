import CryptoKit
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
            .conflicting(storedKeyDigest: Data(SHA256.hash(data: first)))
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
            .conflicting(storedKeyDigest: Data(SHA256.hash(data: stale)))
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

    /// A confirmed replacement stores the conflicting key in the slot.
    func testReplacingAConflictingKeyStoresTheNewKey() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x34, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let stored = Data(repeating: 0x07, count: 32)
        let replacement = Data(repeating: 0x08, count: 32)
        guard case .stored = keychain.storeSpecialKeyIfAbsent(stored, identityId: identityId, keyType: .voting) else {
            return XCTFail("an empty slot stores the key")
        }
        guard case .conflicting(let digest) = keychain.storeSpecialKeyIfAbsent(
            replacement, identityId: identityId, keyType: .voting
        ) else {
            return XCTFail("a different key for an occupied slot conflicts")
        }
        XCTAssertEqual(digest, Data(SHA256.hash(data: stored)))
        guard case .stored = keychain.replaceSpecialKey(
            replacement, identityId: identityId, keyType: .voting, replacing: digest
        ) else {
            return XCTFail("a confirmed replacement stores the new key")
        }
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            replacement
        )
    }

    /// Two confirmations shown for the same stored key: the second one,
    /// answered after the first replaced the key, changes nothing.
    func testAStaleConfirmationLeavesTheFirstReplacementIntact() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x35, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let original = Data(repeating: 0x09, count: 32)
        let first = Data(repeating: 0x0A, count: 32)
        let second = Data(repeating: 0x0B, count: 32)
        guard case .stored = keychain.storeSpecialKeyIfAbsent(original, identityId: identityId, keyType: .voting) else {
            return XCTFail("an empty slot stores the key")
        }
        // Both dialogs were shown for the original key.
        guard case .conflicting(let shown) = keychain.storeSpecialKeyIfAbsent(
            first, identityId: identityId, keyType: .voting
        ) else {
            return XCTFail("a different key for an occupied slot conflicts")
        }

        guard case .stored = keychain.replaceSpecialKey(
            first, identityId: identityId, keyType: .voting, replacing: shown
        ) else {
            return XCTFail("the first confirmation replaces the original key")
        }
        XCTAssertEqual(
            keychain.replaceSpecialKey(second, identityId: identityId, keyType: .voting, replacing: shown),
            .changedSinceConfirmation
        )
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            first
        )
    }

    /// A confirmation shown for a key that was removed since (for example
    /// in another window) changes nothing: the emptied slot stays empty.
    func testAConfirmationForAClearedSlotLeavesItEmpty() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x36, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let original = Data(repeating: 0x0C, count: 32)
        let replacement = Data(repeating: 0x0D, count: 32)
        guard case .stored = keychain.storeSpecialKeyIfAbsent(original, identityId: identityId, keyType: .voting) else {
            return XCTFail("an empty slot stores the key")
        }
        guard case .conflicting(let digest) = keychain.storeSpecialKeyIfAbsent(
            replacement, identityId: identityId, keyType: .voting
        ) else {
            return XCTFail("a different key for an occupied slot conflicts")
        }

        XCTAssertTrue(keychain.deleteSpecialKey(identityId: identityId, keyType: .voting))
        XCTAssertEqual(
            keychain.replaceSpecialKey(replacement, identityId: identityId, keyType: .voting, replacing: digest),
            .changedSinceConfirmation
        )
        XCTAssertNil(try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get())
    }

    /// Another writer fills the slot after the confirmed delete and before
    /// the add: its key is kept and reported as conflicting.
    func testAWriterFillingTheSlotAfterTheDeleteKeepsItsKey() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x37, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let original = Data(repeating: 0x0E, count: 32)
        let ours = Data(repeating: 0x0F, count: 32)
        let theirs = Data(repeating: 0x10, count: 32)
        guard case .stored = keychain.storeSpecialKeyIfAbsent(original, identityId: identityId, keyType: .voting) else {
            return XCTFail("an empty slot stores the key")
        }
        guard case .conflicting(let digest) = keychain.storeSpecialKeyIfAbsent(
            ours, identityId: identityId, keyType: .voting
        ) else {
            return XCTFail("a different key for an occupied slot conflicts")
        }

        let outcome = keychain.replaceSpecialKey(
            ours,
            identityId: identityId,
            keyType: .voting,
            replacing: digest,
            afterDelete: {
                _ = keychain.storeSpecialKey(theirs, identityId: identityId, keyType: .voting)
            }
        )
        XCTAssertEqual(outcome, .conflicting(storedKeyDigest: Data(SHA256.hash(data: theirs))))
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            theirs
        )
    }

    /// Another writer fills the slot after it was read as empty: the add
    /// reports a duplicate and the slot is compared, never overwritten.
    func testAWriterFillingTheSlotAfterTheEmptyReadIsComparedNotOverwritten() throws {
        let keychain = KeychainManager(serviceName: "org.dashfoundation.tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0x33, count: 32)
        addTeardownBlock { @MainActor in
            _ = keychain.deleteSpecialKey(identityId: identityId, keyType: .voting)
        }

        let theirs = Data(repeating: 0x05, count: 32)
        let ours = Data(repeating: 0x06, count: 32)
        let outcome = keychain.storeSpecialKeyIfAbsent(ours, identityId: identityId, keyType: .voting) {
            _ = keychain.storeSpecialKey(theirs, identityId: identityId, keyType: .voting)
        }
        XCTAssertEqual(outcome, .conflicting(storedKeyDigest: Data(SHA256.hash(data: theirs))))
        XCTAssertEqual(
            try keychain.readSpecialKey(identityId: identityId, keyType: .voting).get(),
            theirs
        )

        // The same bytes from the competing writer count as already stored.
        XCTAssertTrue(keychain.deleteSpecialKey(identityId: identityId, keyType: .voting))
        guard case .alreadyStored = keychain.storeSpecialKeyIfAbsent(ours, identityId: identityId, keyType: .voting, afterEmptyRead: {
            _ = keychain.storeSpecialKey(ours, identityId: identityId, keyType: .voting)
        }) else {
            return XCTFail("matching bytes from the competing writer are already stored")
        }
    }
}
