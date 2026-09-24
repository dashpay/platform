import Security
import XCTest

@testable import SwiftDashSDK

/// Classification logic of `WalletStorage.walletPresence()` with the two
/// Keychain reads and the backfill write injected, so the suite never
/// touches the real Keychain (the same reason
/// `IdentityResolverSignIntegrationTests` uses an in-memory storage: a
/// CI runner's keychain session is an environment, not what is under
/// test). What the fake models is the *outcome* of each read on a
/// device in a given state — unreadable, readable-empty, readable-with-ids.
final class WalletPresenceTests: XCTestCase {
    /// `WalletStorage` whose marker read, mnemonic inventory and marker
    /// write are scripted. A `nil` result throws `keychainError(status)`
    /// with the given status, the way the real reads fail on a locked
    /// device.
    private final class ScriptedStorage: WalletStorage {
        var marked: [Data]?
        var inventory: [Data]?
        var failureStatus: OSStatus = errSecInteractionNotAllowed
        var markerWriteStatus: OSStatus = errSecSuccess
        private(set) var markerWrites: [Data] = []
        private(set) var inventoryReads = 0

        override func markedWalletIds() throws -> [Data] {
            guard let marked else { throw WalletStorageError.keychainError(failureStatus) }
            return marked
        }

        override func listWalletIdsWithMnemonic() throws -> [Data] {
            inventoryReads += 1
            guard let inventory else { throw WalletStorageError.keychainError(failureStatus) }
            return inventory
        }

        override func storePresenceMarker(for walletId: Data) throws {
            guard markerWriteStatus == errSecSuccess else {
                throw WalletStorageError.keychainError(markerWriteStatus)
            }
            markerWrites.append(walletId)
        }
    }

    private let walletA = Data(repeating: 0xA1, count: 32)
    private let walletB = Data(repeating: 0xB2, count: 32)

    func testMarkedWalletIsPresentWithoutConsultingTheInventory() {
        let storage = ScriptedStorage()
        storage.marked = [walletA]
        storage.inventory = nil // would throw: the device is locked

        XCTAssertEqual(storage.walletPresence(), .present)
        XCTAssertEqual(storage.inventoryReads, 0, "a marker answers on its own")
        XCTAssertTrue(storage.markerWrites.isEmpty, "nothing to backfill")
    }

    func testReadableEmptyMarkerAndEmptyInventoryIsAbsent() {
        let storage = ScriptedStorage()
        storage.marked = []
        storage.inventory = []

        XCTAssertEqual(storage.walletPresence(), .absent)
        XCTAssertEqual(storage.inventoryReads, 1)
        XCTAssertTrue(storage.markerWrites.isEmpty)
    }

    func testEmptyMarkerFallsBackToInventoryAndBackfillsEveryWallet() {
        let storage = ScriptedStorage()
        storage.marked = []
        storage.inventory = [walletA, walletB]

        XCTAssertEqual(storage.walletPresence(), .present)
        XCTAssertEqual(storage.markerWrites, [walletA, walletB])
    }

    func testUnreadableMarkerIsUnknownWithTheKeychainStatus() {
        let storage = ScriptedStorage()
        storage.marked = nil // before the first unlock since boot
        storage.inventory = [walletA] // must not be reached

        XCTAssertEqual(storage.walletPresence(), .unknown(errSecInteractionNotAllowed))
        XCTAssertEqual(storage.inventoryReads, 0, "an unreadable marker ends the lookup")
        XCTAssertTrue(storage.markerWrites.isEmpty)
    }

    func testEmptyMarkerWithUnreadableInventoryIsUnknownNotAbsent() {
        // A locked device holding wallets stored before the marker
        // existed: the marker reads empty, the inventory cannot be read.
        let storage = ScriptedStorage()
        storage.marked = []
        storage.inventory = nil
        storage.failureStatus = errSecInteractionNotAllowed

        XCTAssertEqual(storage.walletPresence(), .unknown(errSecInteractionNotAllowed))
        XCTAssertTrue(storage.markerWrites.isEmpty)
    }

    func testFailedBackfillDoesNotChangeThePresentVerdict() {
        let storage = ScriptedStorage()
        storage.marked = []
        storage.inventory = [walletA]
        storage.markerWriteStatus = errSecWrPerm

        XCTAssertEqual(storage.walletPresence(), .present)
        XCTAssertTrue(storage.markerWrites.isEmpty)
    }

    func testMarkerServiceIsASiblingOfTheMnemonicService() {
        // The marker items are `AfterFirstUnlock`, the mnemonic items
        // `WhenUnlocked`; a shared service would make one service-wide
        // query cover both accessibility classes. See the doc comment on
        // `presenceKeychainService` for why that must not happen.
        XCTAssertEqual(
            WalletStorage.presenceKeychainService,
            "\(WalletStorage.keychainService).presence"
        )
        XCTAssertNotEqual(WalletStorage.presenceKeychainService, WalletStorage.keychainService)
    }
}
