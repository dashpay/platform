import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

@MainActor
final class AssetLockOperationPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let operationId = Data((0..<16).map(UInt8.init))
    private let outPointHex = String(repeating: "31", count: 32) + ":0"

    private func snapshot(status: Int, operationId: Data?) -> PlatformWalletPersistenceHandler.AssetLockEntrySnapshot {
        .init(outPointHex: outPointHex, transactionBytes: Data([3, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
              fundingTypeRaw: 5, identityIndexRaw: 0, accountIndexRaw: 0,
              amountDuffs: 1000, statusRaw: status, proofBytes: nil, operationId: operationId)
    }

    private var rawOutPoint: Data { Data(repeating: 0x31, count: 32) + Data(repeating: 0, count: 4) }

    private func persist(_ handler: PlatformWalletPersistenceHandler, status: Int, operationId: Data?) throws {
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId,
            upserts: [snapshot(status: status, operationId: nil)], removed: []))
        if let operationId {
            try handler.persistAssetLockOperationId(walletId: walletId, outPoint: rawOutPoint, operationId: operationId)
        }
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testOperationIdSurvivesConsumptionAndStoreReopen() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            try persist(handler, status: 0, operationId: operationId)
            try persist(handler, status: 4, operationId: operationId)
            // A stale legacy snapshot cannot erase the operation correlation.
            try persist(handler, status: 4, operationId: nil)
        }
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        let rows = try handler.loadCachedAssetLocks(walletId: walletId)
        XCTAssertEqual(rows.count, 1)
        XCTAssertEqual(rows.first?.operationId, operationId)
        XCTAssertEqual(rows.first?.statusRaw, 4)
        XCTAssertEqual(try handler.loadAssetLockOperationId(walletId: walletId, outPoint: rawOutPoint), operationId)
    }

    func testOperationSidecarFailsClosedAndCannotChangeItsAssociation() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertThrowsError(try handler.loadAssetLockOperationId(walletId: walletId, outPoint: rawOutPoint))
        XCTAssertThrowsError(try handler.persistAssetLockOperationId(
            walletId: walletId, outPoint: rawOutPoint, operationId: operationId))
        try persist(handler, status: 0, operationId: operationId)
        handler.beginChangeset(walletId: walletId)
        XCTAssertThrowsError(try handler.persistAssetLockOperationId(
            walletId: walletId, outPoint: rawOutPoint, operationId: Data(repeating: 9, count: 16)))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        XCTAssertEqual(try handler.loadAssetLockOperationId(walletId: walletId, outPoint: rawOutPoint), operationId)
    }

    func testPublishedV3MigratesOperationIdToNil() throws {
        _ = DashModelContainer.schema
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV3.self)
            let container = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let context = ModelContext(container)
            context.insert(DashSchemaSnapshotV3.PersistentAssetLock(
                outPointHex: outPointHex, walletId: walletId, transactionBytes: Data([1]),
                fundingTypeRaw: 5, identityIndexRaw: 0, accountIndexRaw: 0,
                amountDuffs: 1000, statusRaw: 1))
            try context.save()
        }
        let migrated = try DashModelContainer.create(url: url)
        let rows = try ModelContext(migrated).fetch(FetchDescriptor<PersistentAssetLock>())
        XCTAssertEqual(rows.count, 1)
        XCTAssertNil(rows.first?.operationId)
        XCTAssertNil(rows.first?.shieldedFundingAttemptsBlob)
        XCTAssertEqual(rows.first?.amountDuffs, 1000)
        XCTAssertEqual(DashModelContainer.schema.version, Schema.Version(4, 0, 0))
    }

    func testMalformedOperationIdFailsLoadInsteadOfBecomingAbsent() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentAssetLock(outPointHex: outPointHex, walletId: walletId,
            transactionBytes: Data([1]), fundingTypeRaw: 5, identityIndexRaw: 0,
            amountDuffs: 1000, statusRaw: 0, operationId: Data(repeating: 1, count: 15)))
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertThrowsError(try handler.loadCachedAssetLocks(walletId: walletId))
    }

    func testSignedFundingAttemptsAreAppendOnlyAndSurviveConsumedReopen() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("attempts.store")
        let first = Data([18, 1, 2, 3])
        let second = Data([18, 4, 5, 6, 7])
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            XCTAssertThrowsError(try handler.loadShieldedFundingAttempts(walletId: walletId, outPoint: rawOutPoint))
            try persist(handler, status: 1, operationId: operationId)
            handler.beginChangeset(walletId: walletId)
            for (index, bytes) in [first, second].enumerated() {
                try handler.persistShieldedFundingAttempt(walletId: walletId, outPoint: rawOutPoint,
                    operationId: operationId, attemptIndex: UInt32(index), bytes: bytes)
            }
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
            handler.beginChangeset(walletId: walletId)
            try handler.persistShieldedFundingAttempt(walletId: walletId, outPoint: rawOutPoint,
                operationId: operationId, attemptIndex: 0, bytes: first)
            XCTAssertThrowsError(try handler.persistShieldedFundingAttempt(walletId: walletId, outPoint: rawOutPoint,
                operationId: operationId, attemptIndex: 0, bytes: second))
            XCTAssertThrowsError(try handler.persistShieldedFundingAttempt(walletId: walletId, outPoint: rawOutPoint,
                operationId: operationId, attemptIndex: 3, bytes: second))
            XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
            try persist(handler, status: 4, operationId: nil)
        }
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        XCTAssertEqual(try handler.loadShieldedFundingAttempts(walletId: walletId, outPoint: rawOutPoint), [first, second])
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 4)
    }

    func testMalformedAttemptArchiveFailsRecoveryRead() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let record = PersistentAssetLock(outPointHex: outPointHex, walletId: walletId,
            transactionBytes: Data([1]), fundingTypeRaw: 5, identityIndexRaw: 0,
            amountDuffs: 1000, statusRaw: 4, operationId: operationId)
        record.shieldedFundingAttemptsBlob = Data([0])
        context.insert(record)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertThrowsError(try handler.loadShieldedFundingAttempts(walletId: walletId, outPoint: rawOutPoint))
    }

    func testFetchFailureRejectsPersistenceAndRecoveryRead() throws {
        struct FailingAssetLockFetcher: ModelFetching {
            enum Failure: Error { case read }
            func fetch<T: PersistentModel>(_ descriptor: FetchDescriptor<T>, in context: ModelContext) throws -> [T] {
                if T.self == PersistentAssetLock.self { throw Failure.read }
                return try context.fetch(descriptor)
            }
        }
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container,
            network: .testnet, modelFetcher: FailingAssetLockFetcher())
        XCTAssertThrowsError(try handler.loadCachedAssetLocks(walletId: walletId))
        handler.beginChangeset(walletId: walletId)
        XCTAssertFalse(handler.persistAssetLocks(walletId: walletId,
            upserts: [snapshot(status: 0, operationId: operationId)], removed: []))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        XCTAssertTrue(try ModelContext(container).fetch(FetchDescriptor<PersistentAssetLock>()).isEmpty)
    }
}
