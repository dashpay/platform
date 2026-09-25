import Security
import XCTest

@testable import SwiftDashSDK

/// The presence-marker lifecycle of `WalletStorage` — `storeMnemonic`,
/// `deleteMnemonic` and `walletPresence()` — run against an in-memory
/// keychain instead of the real one (the same reason
/// `IdentityResolverSignIntegrationTests` uses an in-memory storage: a
/// CI runner's keychain session is an environment, not what is under
/// test). The fake models the *outcome* of each keychain step on a
/// device in a given state — unreadable, readable-empty, readable-with-
/// ids — and records the order of the steps, so the invariant *marker ⇒
/// a mnemonic was stored* can be checked at every interruption point
/// and across an interleaving between two storage instances.
final class WalletPresenceTests: XCTestCase {
    /// One keychain shared by every `FakeStorage` in a test, the way the
    /// real one is shared by every `WalletStorage()` in the process.
    private final class FakeKeychain: @unchecked Sendable {
        private let lock = NSLock()
        private var mnemonics: Set<Data> = []
        private var markers: Set<Data> = []
        private var log: [String] = []

        /// `nil` means readable; a status means every read of that set
        /// throws `keychainError(status)`, as on a locked device.
        var inventoryFailure: OSStatus?
        var markerReadFailure: OSStatus?
        var markerWriteFailure: OSStatus?
        var mnemonicAddFailure: OSStatus?

        func seed(mnemonics: [Data] = [], markers: [Data] = []) {
            lock.withLock {
                self.mnemonics = Set(mnemonics)
                self.markers = Set(markers)
            }
        }

        var mnemonicIds: Set<Data> { lock.withLock { mnemonics } }
        var markerIds: Set<Data> { lock.withLock { markers } }
        var steps: [String] { lock.withLock { log } }

        private static func tag(_ id: Data) -> String {
            String(format: "%02x", id.first ?? 0)
        }

        func listMnemonics() throws -> [Data] {
            try lock.withLock {
                log.append("inventory")
                if let status = inventoryFailure { throw WalletStorageError.keychainError(status) }
                return Array(mnemonics)
            }
        }

        func listMarkers() throws -> [Data] {
            try lock.withLock {
                log.append("markers")
                if let status = markerReadFailure { throw WalletStorageError.keychainError(status) }
                return Array(markers)
            }
        }

        func addMnemonic(_ id: Data) throws {
            try lock.withLock {
                log.append("addMnemonic:\(Self.tag(id))")
                if let status = mnemonicAddFailure { throw WalletStorageError.keychainError(status) }
                mnemonics.insert(id)
            }
        }

        func deleteMnemonic(_ id: Data) {
            lock.withLock {
                log.append("deleteMnemonic:\(Self.tag(id))")
                mnemonics.remove(id)
            }
        }

        func addMarker(_ id: Data) throws {
            try lock.withLock {
                log.append("addMarker:\(Self.tag(id))")
                if let status = markerWriteFailure { throw WalletStorageError.keychainError(status) }
                markers.insert(id)
            }
        }

        func deleteMarker(_ id: Data) {
            lock.withLock {
                log.append("deleteMarker:\(Self.tag(id))")
                markers.remove(id)
            }
        }
    }

    /// `WalletStorage` whose keychain steps go to a `FakeKeychain`. Only
    /// the item-level primitives are overridden; the lifecycle logic under
    /// test is the real one.
    private class FakeStorage: WalletStorage, @unchecked Sendable {
        let keychain: FakeKeychain

        init(_ keychain: FakeKeychain) {
            self.keychain = keychain
            super.init()
        }

        override func listWalletIdsWithMnemonic() throws -> [Data] { try keychain.listMnemonics() }
        override func markedWalletIds() throws -> [Data] { try keychain.listMarkers() }
        override func addMnemonicItem(_ data: Data, for walletId: Data) throws {
            try keychain.addMnemonic(walletId)
        }
        override func deleteMnemonicItem(for walletId: Data) throws { keychain.deleteMnemonic(walletId) }
        override func storePresenceMarker(for walletId: Data) throws { try keychain.addMarker(walletId) }
        override func deletePresenceMarker(for walletId: Data) throws { keychain.deleteMarker(walletId) }
    }

    private let walletA = Data(repeating: 0xA1, count: 32)
    private let walletB = Data(repeating: 0xB2, count: 32)
    private let locked: OSStatus = errSecInteractionNotAllowed

    // MARK: - Classification

    func testUnreadableMarkerIsUnknownWithoutConsultingTheInventory() {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA])
        keychain.markerReadFailure = locked // before the first unlock since boot

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .unknown(locked))
        XCTAssertEqual(keychain.steps, ["markers"], "an unreadable marker set ends the lookup")
    }

    func testMarkedWalletIsPresentWhileTheInventoryIsUnreadable() {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA], markers: [walletA])
        keychain.inventoryFailure = locked

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .present)
        XCTAssertEqual(keychain.steps, ["markers", "inventory"], "no writes while locked")
    }

    func testEmptyMarkerSetWithUnreadableInventoryIsUnknownNotAbsent() {
        // A locked device holding wallets stored before the marker
        // existed: the marker set reads empty, the inventory cannot be read.
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA])
        keychain.inventoryFailure = locked

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .unknown(locked))
        XCTAssertTrue(keychain.markerIds.isEmpty)
    }

    func testReadableEmptyInventoryIsAbsent() {
        let keychain = FakeKeychain()

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .absent)
        XCTAssertEqual(keychain.steps, ["markers", "inventory"])
    }

    // MARK: - Reconciliation while the inventory is readable

    func testUnmarkedWalletsAreBackfilledFromTheInventory() {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA, walletB])

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .present)
        XCTAssertEqual(keychain.markerIds, [walletA, walletB])
    }

    func testMarkerWithoutMnemonicIsRemovedAndTheVerdictIsAbsent() {
        // A phantom left by an older build or an interrupted write: the
        // inventory is authoritative once readable.
        let keychain = FakeKeychain()
        keychain.seed(markers: [walletA])

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .absent)
        XCTAssertTrue(keychain.markerIds.isEmpty)
    }

    func testMarkerSetIsRewrittenToMatchTheInventory() {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletB], markers: [walletA])

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .present)
        XCTAssertEqual(keychain.markerIds, [walletB])
    }

    func testFailedBackfillDoesNotChangeTheVerdict() {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA])
        keychain.markerWriteFailure = errSecWrPerm

        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .present)
        XCTAssertTrue(keychain.markerIds.isEmpty)
    }

    // MARK: - Ordering inside storeMnemonic / deleteMnemonic

    func testStoreWritesTheMnemonicBeforeTheMarker() throws {
        let keychain = FakeKeychain()

        try FakeStorage(keychain).storeMnemonic("abandon", for: walletA)

        XCTAssertEqual(
            keychain.steps,
            ["deleteMarker:a1", "deleteMnemonic:a1", "addMnemonic:a1", "addMarker:a1"]
        )
        XCTAssertEqual(keychain.mnemonicIds, [walletA])
        XCTAssertEqual(keychain.markerIds, [walletA])
    }

    func testFailedMnemonicWriteLeavesNoMarker() {
        // The interruption the order exists for: a process killed after
        // the marker step but before the mnemonic step must not leave a
        // marker behind — so the marker step is last.
        let keychain = FakeKeychain()
        keychain.mnemonicAddFailure = errSecWrPerm

        XCTAssertThrowsError(try FakeStorage(keychain).storeMnemonic("abandon", for: walletA))
        XCTAssertTrue(keychain.markerIds.isEmpty)
        XCTAssertFalse(keychain.steps.contains("addMarker:a1"))
    }

    func testRewriteTakesTheMarkerOffBeforeTheDeleteAddGap() throws {
        // A kill between the old mnemonic's delete and the new one's add
        // must not retain the marker across the gap.
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA], markers: [walletA])

        try FakeStorage(keychain).storeMnemonic("abandon", for: walletA)

        let steps = keychain.steps
        let markerOff = try XCTUnwrap(steps.firstIndex(of: "deleteMarker:a1"))
        let mnemonicOff = try XCTUnwrap(steps.firstIndex(of: "deleteMnemonic:a1"))
        XCTAssertLessThan(markerOff, mnemonicOff)
        XCTAssertEqual(keychain.markerIds, [walletA])
    }

    func testFailedTrailingMarkerWriteDoesNotFailTheStore() throws {
        let keychain = FakeKeychain()
        keychain.markerWriteFailure = errSecWrPerm

        try FakeStorage(keychain).storeMnemonic("abandon", for: walletA)

        XCTAssertEqual(keychain.mnemonicIds, [walletA], "the mnemonic is stored")
        XCTAssertTrue(keychain.markerIds.isEmpty, "the safe direction of drift")
        keychain.markerWriteFailure = nil
        XCTAssertEqual(FakeStorage(keychain).walletPresence(), .present)
        XCTAssertEqual(keychain.markerIds, [walletA], "and the next unlocked read repairs it")
    }

    func testDeleteRemovesTheMnemonicBeforeTheMarker() throws {
        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA], markers: [walletA])

        try FakeStorage(keychain).deleteMnemonic(for: walletA)

        XCTAssertEqual(keychain.steps, ["deleteMnemonic:a1", "deleteMarker:a1"])
        XCTAssertTrue(keychain.mnemonicIds.isEmpty)
        XCTAssertTrue(keychain.markerIds.isEmpty)
    }

    // MARK: - Interleaving across storage instances

    func testPresenceReadCannotBackfillAMarkerDuringADeletion() throws {
        // `deleteMnemonic` on one instance is paused at its first keychain
        // step; `walletPresence()` on a second instance (the inventory
        // still lists the wallet, no marker yet) must wait for the
        // deletion instead of backfilling a marker the deletion would
        // then leave behind.
        final class PausingStorage: FakeStorage, @unchecked Sendable {
            let entered = DispatchSemaphore(value: 0)
            let release = DispatchSemaphore(value: 0)
            override func deleteMnemonicItem(for walletId: Data) throws {
                entered.signal()
                release.wait()
                try super.deleteMnemonicItem(for: walletId)
            }
        }

        let keychain = FakeKeychain()
        keychain.seed(mnemonics: [walletA])
        let deleting = PausingStorage(keychain)
        let reading = FakeStorage(keychain)

        let walletA = walletA
        let deleteDone = expectation(description: "delete finished")
        Thread.detachNewThread {
            try? deleting.deleteMnemonic(for: walletA)
            deleteDone.fulfill()
        }
        XCTAssertEqual(deleting.entered.wait(timeout: .now() + 5), .success)

        let presenceDone = expectation(description: "presence finished")
        let verdict = LockedBox<WalletStorage.WalletPresence?>(nil)
        Thread.detachNewThread {
            verdict.value = reading.walletPresence()
            presenceDone.fulfill()
        }
        // The read is blocked behind the paused deletion: it has not
        // touched the keychain yet.
        Thread.sleep(forTimeInterval: 0.2)
        XCTAssertNil(verdict.value)
        XCTAssertFalse(keychain.steps.contains("markers"))

        deleting.release.signal()
        wait(for: [deleteDone, presenceDone], timeout: 5)

        XCTAssertEqual(verdict.value, .absent)
        XCTAssertTrue(keychain.mnemonicIds.isEmpty)
        XCTAssertTrue(keychain.markerIds.isEmpty, "no marker survives the deletion")
    }

    private final class LockedBox<T>: @unchecked Sendable {
        private let lock = NSLock()
        private var stored: T
        init(_ value: T) { stored = value }
        var value: T {
            get { lock.withLock { stored } }
            set { lock.withLock { stored = newValue } }
        }
    }

    // MARK: - Layout

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
