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

    func testConflictRevisionRejectsStaleSnapshotsAndRemoval() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try persist(handler, status: 1, operationId: operationId)
        var conflict = snapshot(status: 7, operationId: operationId)
        conflict.operationRevision = 1
        conflict.expectedOperationRevision = 0
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [conflict], removed: [], revisionAware: true))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        handler.beginChangeset(walletId: walletId)
        XCTAssertFalse(handler.persistAssetLocks(walletId: walletId,
            upserts: [snapshot(status: 1, operationId: operationId)], removed: [], revisionAware: true))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [], removed: [rawOutPoint], revisionAware: true))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        let row = try XCTUnwrap(handler.loadCachedAssetLocks(walletId: walletId).first)
        XCTAssertEqual(row.statusRaw, 7)
        XCTAssertEqual(row.operationRevision, 1)
        XCTAssertEqual(row.operationId, operationId)
        // A lost commit acknowledgement permits identical CAS redelivery.
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [conflict], removed: [], revisionAware: true))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testV4OperationBindingMigratesWithLegacyRevisionAndSurvivesReopen() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("migration.store")
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV4.self)
            let container = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let row = snapshot(status: 1, operationId: operationId)
            container.mainContext.insert(DashSchemaSnapshotV4.PersistentAssetLock(
                outPointHex: row.outPointHex, walletId: walletId, transactionBytes: row.transactionBytes,
                fundingTypeRaw: row.fundingTypeRaw, identityIndexRaw: row.identityIndexRaw,
                accountIndexRaw: row.accountIndexRaw, amountDuffs: row.amountDuffs,
                statusRaw: row.statusRaw, operationId: operationId))
            try container.mainContext.save()
        }
        XCTAssertEqual(try DashLegacySchemaBridge.identity(at: url).versions, ["4.0.0"])
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            let migrated = try XCTUnwrap(handler.loadCachedAssetLocks(walletId: walletId).first)
            XCTAssertEqual(migrated.operationId, operationId)
            XCTAssertEqual(migrated.operationRevision, 0)
            var conflict = snapshot(status: 7, operationId: operationId)
            conflict.operationRevision = 1
            conflict.expectedOperationRevision = 0
            handler.beginChangeset(walletId: walletId)
            XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [conflict], removed: [], revisionAware: true))
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        }
        XCTAssertEqual(try DashLegacySchemaBridge.identity(at: url).versions, ["5.0.0"])
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        let row = try XCTUnwrap(handler.loadCachedAssetLocks(walletId: walletId).first)
        XCTAssertEqual(row.operationId, operationId)
        XCTAssertEqual(row.operationRevision, 1)
        XCTAssertEqual(row.statusRaw, 7)
        XCTAssertEqual(row.outPointHex, outPointHex)
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
        XCTAssertEqual(rows.first?.amountDuffs, 1000)
        XCTAssertEqual(DashModelContainer.schema.version, Schema.Version(5, 0, 0))
    }

    func testMissingOrNegativeOperationRevisionFailsRestore() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertThrowsError(try handler.loadAssetLockOperationRevision(walletId: walletId, outPoint: rawOutPoint))
        let context = ModelContext(container)
        let row = PersistentAssetLock(outPointHex: outPointHex, walletId: walletId,
            transactionBytes: snapshot(status: 1, operationId: nil).transactionBytes,
            fundingTypeRaw: 5, identityIndexRaw: 0, amountDuffs: 1000, statusRaw: 7,
            operationId: operationId)
        row.operationRevision = -1
        context.insert(row)
        try context.save()
        XCTAssertThrowsError(try handler.loadAssetLockOperationRevision(walletId: walletId, outPoint: rawOutPoint))
        XCTAssertThrowsError(try handler.loadCachedAssetLocks(walletId: walletId))
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

    func testShouldPreserveProvenDrainCompletionAfterReopenAndStaleSnapshot() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            try persist(handler, status: 4, operationId: operationId)
            try persist(handler, status: 6, operationId: nil)
            try persist(handler, status: 4, operationId: nil)
            try persist(handler, status: 0, operationId: nil)
        }
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: try DashModelContainer.create(url: url), network: .testnet)
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 6)
        XCTAssertEqual(try handler.loadAssetLockOperationId(walletId: walletId, outPoint: rawOutPoint), operationId)
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
