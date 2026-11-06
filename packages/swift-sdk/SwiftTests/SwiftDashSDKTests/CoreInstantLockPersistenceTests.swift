import XCTest
import SwiftData
@testable import SwiftDashSDK

final class CoreInstantLockPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let txid = Data(repeating: 7, count: 32)

    // TODO(macOS-validation): run these SwiftData rollback/reopen tests on an Apple SDK runner.
    func testShouldCommitRawLockAndRollbackReplacement() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        handler.beginChangeset(walletId: walletId)
        try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([1, 2, 3]))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        handler.beginChangeset(walletId: walletId)
        try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([4]))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        XCTAssertEqual(try handler.loadCoreInstantLock(walletId: walletId, txid: txid), Data([1, 2, 3]))
        XCTAssertNil(try handler.loadCoreInstantLock(walletId: Data(repeating: 43, count: 32), txid: txid))
        let mainnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        XCTAssertNil(try mainnet.loadCoreInstantLock(walletId: walletId, txid: txid))
        XCTAssertThrowsError(try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([5])))
    }

    func testShouldDeleteOnlyTheSelectedWalletNetworkSidecar() throws {
        let container = try DashModelContainer.createInMemory()
        let testnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let mainnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        for handler in [testnet, mainnet] {
            handler.beginChangeset(walletId: walletId)
            try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([1]))
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        }
        try testnet.deleteWalletData(walletId: walletId)
        XCTAssertNil(try testnet.loadCoreInstantLock(walletId: walletId, txid: txid))
        XCTAssertEqual(try mainnet.loadCoreInstantLock(walletId: walletId, txid: txid), Data([1]))
    }

    func testShouldPreserveRawLockAcrossStoreReopen() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            handler.beginChangeset(walletId: walletId)
            try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([9, 8, 7]))
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        }
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        XCTAssertEqual(try handler.loadCoreInstantLock(walletId: walletId, txid: txid), Data([9, 8, 7]))
    }
}
