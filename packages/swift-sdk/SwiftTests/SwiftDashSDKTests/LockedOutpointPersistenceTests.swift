import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// A wallet's locked outpoints (the outpoints kept out of coin selection)
/// round-trip through SwiftData and the persistence extension's C shims.
final class LockedOutpointPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let otherWalletId = Data(repeating: 43, count: 32)

    private typealias Change = PlatformWalletPersistenceHandler.OutpointLockChange

    /// The 36-byte key of the outpoint `vout` of the txid made of `byte`.
    private func outpoint(_ byte: UInt8, _ vout: UInt32) -> Data {
        PersistentTxo.makeOutpoint(txid: Data(repeating: byte, count: 32), vout: vout)
    }

    private func makeHandler() throws -> (ModelContainer, PlatformWalletPersistenceHandler) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: otherWalletId, network: .testnet))
        try context.save()
        return (container, PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet))
    }

    private func round(
        _ handler: PlatformWalletPersistenceHandler,
        _ locks: [Change],
        wallet: Data? = nil,
        success: Bool = true
    ) throws {
        let wallet = wallet ?? walletId
        handler.beginChangeset(walletId: wallet)
        try handler.persistWalletChangesetOutpointLocks(walletId: wallet, locks: locks)
        XCTAssertEqual(handler.endChangeset(walletId: wallet, success: success), success)
    }

    private func storedRows(_ container: ModelContainer) throws -> [PersistentLockedOutpoint] {
        try ModelContext(container).fetch(FetchDescriptor<PersistentLockedOutpoint>())
    }

    func testShouldStoreARowForALockWithItsRound() throws {
        let (container, handler) = try makeHandler()
        try round(handler, [
            .init(outpoint: outpoint(0x11, 0), locked: true),
            .init(outpoint: outpoint(0x22, 3), locked: true),
        ])
        let rows = try storedRows(container)
        XCTAssertEqual(Set(rows.map(\.outpoint)), [outpoint(0x11, 0), outpoint(0x22, 3)])
        XCTAssertEqual(Set(rows.map(\.walletId)), [walletId])
        XCTAssertEqual(Set(rows.map(\.networkRaw)), [Network.testnet.rawValue])

        // Locking a locked outpoint again keeps its one row.
        try round(handler, [.init(outpoint: outpoint(0x11, 0), locked: true)])
        XCTAssertEqual(try storedRows(container).count, 2)

        // A rolled-back round leaves the stored locks as they were.
        try round(handler, [.init(outpoint: outpoint(0x33, 1), locked: true)], success: false)
        XCTAssertEqual(
            try Set(handler.loadWalletLockedOutpoints(walletId: walletId)),
            [outpoint(0x11, 0), outpoint(0x22, 3)])
    }

    func testShouldDeleteTheRowOfAnUnlockedOutpoint() throws {
        let (container, handler) = try makeHandler()
        try round(handler, [
            .init(outpoint: outpoint(0x11, 0), locked: true),
            .init(outpoint: outpoint(0x22, 3), locked: true),
        ])
        try round(handler, [.init(outpoint: outpoint(0x11, 0), locked: false)])
        XCTAssertEqual(try storedRows(container).map(\.outpoint), [outpoint(0x22, 3)])

        // Unlocking an outpoint with no stored lock changes nothing.
        try round(handler, [.init(outpoint: outpoint(0x44, 9), locked: false)])
        XCTAssertEqual(try storedRows(container).map(\.outpoint), [outpoint(0x22, 3)])
    }

    func testShouldLoadTheLocksOfTheAskedWalletOnly() throws {
        let (container, handler) = try makeHandler()
        try round(handler, [
            .init(outpoint: outpoint(0x11, 0), locked: true),
            .init(outpoint: outpoint(0x22, 3), locked: true),
        ])
        try round(handler, [.init(outpoint: outpoint(0x66, 7), locked: true)], wallet: otherWalletId)

        XCTAssertEqual(
            try Set(handler.loadWalletLockedOutpoints(walletId: walletId)),
            [outpoint(0x11, 0), outpoint(0x22, 3)])
        XCTAssertEqual(try handler.loadWalletLockedOutpoints(walletId: otherWalletId), [outpoint(0x66, 7)])
        XCTAssertTrue(try handler.loadWalletLockedOutpoints(walletId: Data(repeating: 44, count: 32)).isEmpty)

        // The same wallet id under another network holds none of them.
        let mainnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        XCTAssertTrue(try mainnet.loadWalletLockedOutpoints(walletId: walletId).isEmpty)
    }

    func testShouldPurgeAWalletsLocksWithItsData() throws {
        let (container, handler) = try makeHandler()
        try round(handler, [
            .init(outpoint: outpoint(0x11, 0), locked: true),
            .init(outpoint: outpoint(0x22, 3), locked: true),
        ])
        try round(handler, [.init(outpoint: outpoint(0x66, 7), locked: true)], wallet: otherWalletId)

        try handler.deleteWalletData(walletId: walletId)
        let rows = try storedRows(container)
        XCTAssertEqual(rows.map(\.walletId), [otherWalletId], "only the deleted wallet's locks go")
        XCTAssertEqual(rows.map(\.outpoint), [outpoint(0x66, 7)])
        XCTAssertTrue(try handler.loadWalletLockedOutpoints(walletId: walletId).isEmpty)
    }

    /// A handler configured with no network stores a lock under its wallet
    /// row's network, and reads it back.
    func testShouldStoreALockUnderItsWalletsNetworkWhenTheHandlerHasNone() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .mainnet))
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        try round(handler, [.init(outpoint: outpoint(0x11, 0), locked: true)])
        XCTAssertEqual(try storedRows(container).map(\.networkRaw), [Network.mainnet.rawValue])
        XCTAssertEqual(try handler.loadWalletLockedOutpoints(walletId: walletId), [outpoint(0x11, 0)])
    }

    /// With no network known on the handler or the wallet row there is
    /// nowhere to file a lock: the write refuses, so the round does not
    /// report a lock stored that is not.
    func testShouldRefuseALockWriteWhenNoNetworkIsKnown() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId))
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        handler.beginChangeset(walletId: walletId)
        XCTAssertThrowsError(try handler.persistWalletChangesetOutpointLocks(
            walletId: walletId, locks: [.init(outpoint: outpoint(0x11, 0), locked: true)]))
        _ = handler.endChangeset(walletId: walletId, success: false)
        XCTAssertTrue(try storedRows(container).isEmpty)
    }

    /// A round for a wallet whose row is gone (a stale callback after the
    /// wallet's deletion) writes nothing and does not fail.
    func testShouldWriteNothingForAWalletWithNoRow() throws {
        let (container, handler) = try makeHandler()
        try round(
            handler, [.init(outpoint: outpoint(0x11, 0), locked: true)],
            wallet: Data(repeating: 44, count: 32))
        XCTAssertTrue(try storedRows(container).isEmpty)
    }

    func testShouldRefuseALockWriteOutsideAChangeset() throws {
        let (container, handler) = try makeHandler()
        XCTAssertThrowsError(try handler.persistWalletChangesetOutpointLocks(
            walletId: walletId, locks: [.init(outpoint: outpoint(0x11, 0), locked: true)]))
        XCTAssertTrue(try storedRows(container).isEmpty)
    }

    private func ffiLock(_ byte: UInt8, _ vout: UInt32, locked: Bool) -> OutpointLockFFI {
        var entry = OutpointLockFFI()
        Swift.withUnsafeMutableBytes(of: &entry.outpoint.txid) {
            _ = Data(repeating: byte, count: 32).copyBytes(to: $0)
        }
        entry.outpoint.vout = vout
        entry.locked = locked
        return entry
    }

    /// The C shims Rust calls: a lock persisted through the extension's
    /// slot is read back through its load slot, an unlock removes it, and
    /// the free slot returns the array. The capability is declared alongside
    /// them.
    func testShouldDeclareTheCapabilityAndRoundTripThroughTheCallbackShims() throws {
        let (_, handler) = try makeHandler()
        XCTAssertNotEqual(
            handler.makePersistenceCapabilities().bits & PlatformWalletPersistenceCapabilities.outpointLocks, 0)
        // Bit 16: bit 15 is the private contact request scan state's.
        XCTAssertEqual(PlatformWalletPersistenceCapabilities.outpointLocks, 1 << 16)
        XCTAssertEqual(
            PlatformWalletPersistenceCapabilities.outpointLocks,
            UInt64(PLATFORM_WALLET_PERSISTENCE_CAPABILITY_OUTPOINT_LOCKS))
        let ext = handler.makePersistenceCallbacksExtension()
        let persist = try XCTUnwrap(ext.on_persist_wallet_changeset_outpoint_locks_fn)
        let load = try XCTUnwrap(ext.on_load_wallet_locked_outpoints_fn)
        let free = try XCTUnwrap(ext.on_load_wallet_locked_outpoints_free_fn)
        let context = Unmanaged.passUnretained(handler).toOpaque()

        func persistRound(_ locks: [OutpointLockFFI]) -> Int32 {
            handler.beginChangeset(walletId: walletId)
            let rc = walletId.withUnsafeBytes { id in
                persist(context, id.bindMemory(to: UInt8.self).baseAddress, locks, UInt(locks.count))
            }
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: rc == 0))
            return rc
        }
        func loadKeys(_ wallet: Data) throws -> [Data] {
            // Stale values: the shim resets both before it reads the store.
            var outpoints: UnsafePointer<OutPointFFI>? = UnsafePointer(bitPattern: 8)
            var count: UInt = 7
            let rc = wallet.withUnsafeBytes { id in
                load(context, id.bindMemory(to: UInt8.self).baseAddress, &outpoints, &count)
            }
            XCTAssertEqual(rc, 0)
            guard count > 0 else {
                XCTAssertNil(outpoints)
                return []
            }
            let rows = UnsafeBufferPointer(start: try XCTUnwrap(outpoints), count: Int(count))
            let keys = rows.map { PersistentTxo.makeOutpoint(txid: hashData($0.txid), vout: $0.vout) }
            free(context, outpoints, count)
            return keys
        }

        // A vout with four distinct bytes: its byte order survives the trip.
        XCTAssertEqual(persistRound([
            ffiLock(0x11, 0, locked: true),
            ffiLock(0x22, 0x0102_0304, locked: true),
        ]), 0)
        XCTAssertEqual(try Set(loadKeys(walletId)), [outpoint(0x11, 0), outpoint(0x22, 0x0102_0304)])

        XCTAssertEqual(persistRound([ffiLock(0x11, 0, locked: false)]), 0)
        XCTAssertEqual(try loadKeys(walletId), [outpoint(0x22, 0x0102_0304)])

        // A wallet with no lock hands back a null array with a zero count.
        XCTAssertTrue(try loadKeys(otherWalletId).isEmpty)
    }

    /// A stored key that is not 36 bytes names no outpoint: the load fails
    /// instead of handing back the rest without it, which would restore
    /// the wallet with that lock missing.
    func testShouldFailTheLoadOnAStoredKeyThatNamesNoOutpoint() throws {
        let (container, handler) = try makeHandler()
        try round(handler, [.init(outpoint: outpoint(0x11, 0), locked: true)])
        let context = ModelContext(container)
        context.insert(PersistentLockedOutpoint(
            networkRaw: Network.testnet.rawValue, walletId: walletId, outpoint: Data(repeating: 9, count: 35)))
        try context.save()

        let load = try XCTUnwrap(handler.makePersistenceCallbacksExtension().on_load_wallet_locked_outpoints_fn)
        let shimContext = Unmanaged.passUnretained(handler).toOpaque()
        var outpoints: UnsafePointer<OutPointFFI>?
        var count: UInt = 0
        let rc = walletId.withUnsafeBytes { wallet in
            load(shimContext, wallet.bindMemory(to: UInt8.self).baseAddress, &outpoints, &count)
        }
        XCTAssertNotEqual(rc, 0)
        XCTAssertNil(outpoints)
        XCTAssertEqual(count, 0)
    }
}
