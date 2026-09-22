import CoreData
import Darwin
import Foundation
import SQLite3
import SwiftData
import XCTest
@testable import SwiftDashSDK

@Model
private final class BridgeFutureMarker {
    var value: String = "future"
    init() {}
}
private enum BridgeFutureV4: VersionedSchema {
    static var versionIdentifier: Schema.Version { Schema.Version(4, 0, 0) }
    static var models: [any PersistentModel.Type] { DashSchemaV3.models + [BridgeFutureMarker.self] }
}
private enum BridgeFuturePlan: SchemaMigrationPlan {
    static var schemas: [any VersionedSchema.Type] { [DashSchemaV1.self, DashSchemaV3.self, BridgeFutureV4.self] }
    static var stages: [MigrationStage] {
        [.lightweight(fromVersion: DashSchemaV1.self, toVersion: DashSchemaV3.self),
         .custom(fromVersion: DashSchemaV3.self, toVersion: BridgeFutureV4.self,
                 willMigrate: { context in
                     for wallet in try context.fetch(FetchDescriptor<PersistentWallet>()) {
                         wallet.name = "explicit future transformation"
                     }
                     try context.save()
                 }, didMigrate: nil)]
    }
}

// The candidate immediately before restoring historical V2 used today's
// complete graph with a V2 version label. This is one exact supported alias.
private enum PreviousLiveV2: VersionedSchema {
    static var versionIdentifier: Schema.Version { Schema.Version(2, 0, 0) }
    static var models: [any PersistentModel.Type] { DashSchemaV3.models }
}
private enum UnsupportedBetaV2: VersionedSchema {
    static var versionIdentifier: Schema.Version { Schema.Version(2, 0, 0) }
    static var models: [any PersistentModel.Type] { DashSchemaV3.models + [BridgeFutureMarker.self] }
}

@MainActor
final class DashLegacySchemaMigrationTests: XCTestCase {
    private enum Injected: Error { case stop }

    private func withStore(baseline: Bool = false, _ body: (URL) throws -> Void) throws {
        let (directory, url) = try makeStore(baseline: baseline)
        defer { try? FileManager.default.removeItem(at: directory) }
        try autoreleasepool { try body(url) }
    }

    private func makeStore(baseline: Bool = false) throws -> (URL, URL) {
        let source = try XCTUnwrap(Bundle.module.url(
            forResource: baseline ? "dash-v1" : "fixture", withExtension: "store",
            subdirectory: baseline ? "Fixtures/SchemaStores" : "Fixtures/SchemaStores/legacy-fd8d8d13e5"))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("DashModel.store")
        try FileManager.default.copyItem(at: source, to: url)
        return (directory, url)
    }
    private func open(_ url: URL, hooks: DashLegacySchemaBridge.Hooks = .init()) throws -> ModelContainer {
        let schema = DashModelContainer.schema
        return try DashLegacySchemaBridge.open(
            configuration: ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none),
            schema: schema, plan: DashMigrationPlan.self, hooks: hooks)
    }
    private func operationDirectories(_ url: URL) throws -> [URL] {
        let root = DashLegacySchemaBridge.backupDirectory(for: url)
        guard FileManager.default.fileExists(atPath: root.path) else { return [] }
        return try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: [.isDirectoryKey])
            .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true }
    }
    private func verifyRows(_ context: ModelContext, walletName: String = "historical audit wallet") throws {
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentWallet>()), 1)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentDataContract>()), 1)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentDocumentType>()), 1)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentIndex>()), 1)
        let wallet = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentWallet>()).first)
        let contract = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentDataContract>()).first)
        let type = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentDocumentType>()).first)
        let index = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentIndex>()).first)
        XCTAssertEqual(wallet.walletId, Data(repeating: 0x61, count: 32))
        XCTAssertEqual(wallet.name, walletName)
        XCTAssertEqual(wallet.network, .testnet)
        XCTAssertEqual(contract.id, Data(repeating: 0x62, count: 32))
        XCTAssertEqual(contract.serializedContract, Data("{}".utf8))
        XCTAssertEqual(type.dataContract?.persistentModelID, contract.persistentModelID)
        XCTAssertEqual(index.documentType?.persistentModelID, type.persistentModelID)
        XCTAssertEqual(contract.documentTypes?.count, 1)
        XCTAssertEqual(type.indices?.count, 1)
        XCTAssertEqual(index.properties, ["name"])
        XCTAssertFalse(type.indexOnly)
        XCTAssertNil(index.countable)
        XCTAssertNil(index.summable)
        XCTAssertNil(index.averageable)
        XCTAssertNil(index.terminal)
        XCTAssertNil(index.timeRangeJSON)
        XCTAssertEqual([index.rangeCountable, index.rangeSummable, index.rangeAverageable,
                        index.rankedCountable, index.rankedSummable, index.rankedAverageable,
                        index.preallocated], Array(repeating: false, count: 7))
    }

    func testAcceptedV1UsesNormalPlanWithoutBridge() throws {
        try withStore(baseline: true) { url in
            _ = try open(url, hooks: .init(visit: { _, _ in XCTFail("Known V1 must not bridge") }))
            XCTAssertTrue(try operationDirectories(url).isEmpty)
            XCTAssertFalse(FileManager.default.fileExists(atPath: url.path + ".legacy-v2.lock"))
        }
        _ = try DashModelContainer.createInMemory()
    }

    func testOrdinaryStoresIgnoreBridgeLockButLegacyAndPendingRecoveryRequireIt() throws {
        try withStore(baseline: true) { url in
            let descriptor = Darwin.open(url.path + ".legacy-v2.lock", O_CREAT | O_RDWR, 0o600)
            XCTAssertGreaterThanOrEqual(descriptor, 0)
            defer { Darwin.close(descriptor) }
            XCTAssertEqual(flock(descriptor, LOCK_EX | LOCK_NB), 0)
            try autoreleasepool { _ = try open(url) } // Known V1.
            let current = try open(url) // Current V3.
            XCTAssertEqual(try current.mainContext.fetchCount(FetchDescriptor<PersistentWallet>()), 1)
        }
        for interrupted in [false, true] {
            try withStore { url in
                if interrupted {
                    XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                        if phase == .afterCommit { throw Injected.stop }
                    })))
                }
                let descriptor = Darwin.open(url.path + ".legacy-v2.lock", O_CREAT | O_RDWR, 0o600)
                XCTAssertGreaterThanOrEqual(descriptor, 0)
                defer { Darwin.close(descriptor) }
                XCTAssertEqual(flock(descriptor, LOCK_EX | LOCK_NB), 0)
                XCTAssertThrowsError(try open(url))
            }
        }
    }

    func testMissingBridgeMetadataUsesOrdinaryOpeningWithoutCreatingBridgeFiles() throws {
        try withStore(baseline: true) { url in
            var metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
            metadata.removeValue(forKey: "NSStoreModelVersionChecksumKey")
            try NSPersistentStoreCoordinator.setMetadata(metadata, type: .sqlite, at: url)
            let container = try open(url, hooks: .init(visit: { _, _ in XCTFail("Must use the ordinary plan") }))
            XCTAssertEqual(try container.mainContext.fetchCount(FetchDescriptor<PersistentWallet>()), 1)
            XCTAssertFalse(FileManager.default.fileExists(atPath: url.path + ".legacy-v2.lock"))
        }
    }

    func testDiskPreflightIncludesWALAndRejectsBeforeCopying() throws {
        try withStore { url in
            let writer = try DashLegacyStoreSQLite.Connection(url, writable: true)
            try writer.execute("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0")
            try writer.execute("UPDATE ZPERSISTENTWALLET SET ZNAME='committed WAL data'")
            let main = try XCTUnwrap(FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber).int64Value
            let wal = try XCTUnwrap(FileManager.default.attributesOfItem(atPath: url.path + "-wal")[.size] as? NSNumber).int64Value
            XCTAssertGreaterThan(wal, 0)
            let required = try DashLegacySchemaBridge.requiredFreeSpace(at: url)
            XCTAssertEqual(required, 4 * (main + wal) + max(64 * 1024 * 1024, (main + wal) / 2))
            let original = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try open(url, hooks: .init(
                visit: { _, _ in XCTFail("Insufficient space must reject before snapshotting") },
                availableCapacity: { directory in
                    XCTAssertEqual(directory, url.deletingLastPathComponent())
                    return required - 1
                }))) { error in
                guard case DashLegacyStoreSQLite.Failure.insufficientDiskSpace(let needed, let available) = error else {
                    return XCTFail("Expected an actionable storage error, got \(error)")
                }
                XCTAssertEqual(needed, required)
                XCTAssertEqual(available, required - 1)
                XCTAssertTrue(error.localizedDescription.contains("Free device storage and retry"))
            }
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            XCTAssertFalse(FileManager.default.fileExists(atPath: DashLegacySchemaBridge.backupDirectory(for: url).path))
            withExtendedLifetime(writer) {}
        }
        try withStore(baseline: true) { url in
            _ = try open(url, hooks: .init(availableCapacity: { _ in
                XCTFail("Registered stores must not require bridge copy headroom")
                return 0
            }))
        }
    }

    func testCapacityResolverConfirmsUnavailableImportantUsageWithFilesystemFreeSpace() throws {
        XCTAssertEqual(try DashLegacySchemaBridge.resolveAvailableCapacity(
            importantUsage: { 1024 }, fileSystem: { XCTFail("Positive capacity needs no fallback"); return 0 }), 1024)
        for reported: Int64? in [0, -1, nil] {
            XCTAssertEqual(try DashLegacySchemaBridge.resolveAvailableCapacity(
                importantUsage: { reported }, fileSystem: { 2048 }), 2048)
        }
        XCTAssertEqual(try DashLegacySchemaBridge.resolveAvailableCapacity(
            importantUsage: { throw Injected.stop }, fileSystem: { 4096 }), 4096)
        XCTAssertEqual(try DashLegacySchemaBridge.resolveAvailableCapacity(
            importantUsage: { 0 }, fileSystem: { 0 }), 0, "A genuinely full volume remains full")
        XCTAssertThrowsError(try DashLegacySchemaBridge.resolveAvailableCapacity(
            importantUsage: { 0 }, fileSystem: { throw Injected.stop }), "Cannot invent capacity when both queries fail")
    }

    func testSufficientDiskHeadroomPermitsMigration() throws {
        try withStore { url in
            let required = try DashLegacySchemaBridge.requiredFreeSpace(at: url)
            let container = try open(url, hooks: .init(availableCapacity: { _ in required }))
            try verifyRows(container.mainContext)
        }
    }

    func testCopyReportsNonContentionSQLiteFailureWithoutReplacingDestination() throws {
        try withStore { destination in
            let source = destination.deletingLastPathComponent().appendingPathComponent("invalid.store")
            try Data(repeating: 0x61, count: 4096).write(to: source)
            let original = try DashLegacyStoreSQLite.rawDigest(destination)
            XCTAssertThrowsError(try DashLegacyStoreSQLite.copy(from: source, to: destination)) { error in
                guard case DashLegacyStoreSQLite.Failure.sqlite(let operation, let code, let reason) = error else {
                    return XCTFail("Expected an actual SQLite status, got \(error)")
                }
                XCTAssertEqual(code & 0xff, SQLITE_NOTADB)
                XCTAssertFalse(reason.lowercased().contains("busy"))
                XCTAssertTrue(error.localizedDescription.contains("SQLite \(code)"))
                XCTAssertFalse(operation.isEmpty)
            }
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(destination), original)
        }
    }

    func testAbandonedAttemptsAreRemovedBeforeDiskCapacityIsMeasured() throws {
        try withStore { url in
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let abandoned = root.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: abandoned, withIntermediateDirectories: true)
            try Data(repeating: 0x61, count: 4096).write(to: abandoned.appendingPathComponent("original.store"))
            try Data(repeating: 0x62, count: 4096).write(to: abandoned.appendingPathComponent("candidate.store"))
            let unrelated = root.appendingPathComponent("operator-note.txt")
            try Data("keep".utf8).write(to: unrelated)
            let required = try DashLegacySchemaBridge.requiredFreeSpace(at: url)
            let container = try open(url, hooks: .init(availableCapacity: { _ in
                XCTAssertFalse(FileManager.default.fileExists(atPath: abandoned.path))
                XCTAssertTrue(FileManager.default.fileExists(atPath: unrelated.path))
                return required
            }))
            try verifyRows(container.mainContext)
            XCTAssertEqual(try operationDirectories(url).count, 1, "Keep the new migration backup through this open")
        }
        try withStore { url in
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let abandoned = root.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: abandoned, withIntermediateDirectories: true)
            // A failed removal must not be counted as reclaimed free space.
            try FileManager.default.setAttributes([.posixPermissions: 0o500], ofItemAtPath: root.path)
            defer { try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path) }
            XCTAssertThrowsError(try open(url, hooks: .init(availableCapacity: { _ in
                XCTAssertTrue(FileManager.default.fileExists(atPath: abandoned.path))
                return 0
            }))) { error in
                guard case DashLegacyStoreSQLite.Failure.insufficientDiskSpace(_, let available) = error else {
                    return XCTFail("Expected actual available capacity to decide admission: \(error)")
                }
                XCTAssertEqual(available, 0)
            }
        }
    }

    func testOrdinaryOpenSkipsCleanupWhileAnotherOpenerOwnsTheLock() throws {
        try withStore(baseline: true) { url in
            try autoreleasepool { _ = try open(url) }
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let active = root.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: active, withIntermediateDirectories: true)
            let copy = active.appendingPathComponent("original.store")
            try Data("active snapshot".utf8).write(to: copy)
            let descriptor = Darwin.open(url.path + ".legacy-v2.lock", O_CREAT | O_RDWR, 0o600)
            XCTAssertGreaterThanOrEqual(descriptor, 0)
            defer { Darwin.close(descriptor) }
            XCTAssertEqual(flock(descriptor, LOCK_EX | LOCK_NB), 0)
            try autoreleasepool {
                let container = try open(url)
                XCTAssertEqual(try container.mainContext.fetchCount(FetchDescriptor<PersistentWallet>()), 1)
            }
            XCTAssertEqual(try Data(contentsOf: copy), Data("active snapshot".utf8))
            XCTAssertEqual(flock(descriptor, LOCK_UN), 0)
            _ = try open(url)
            XCTAssertFalse(FileManager.default.fileExists(atPath: active.path), "A later unlocked successful open may reclaim it")
        }
    }

    func testPendingRecoveryProtectsAllAttemptsAndRetainsRecoveredBackup() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .afterCommit { throw Injected.stop }
            })))
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let marker = root.appendingPathComponent("active.json")
            let originalJournal = try Data(contentsOf: marker)
            let abandoned = root.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: abandoned, withIntermediateDirectories: true)
            let attempts = Set(try operationDirectories(url))
            try DashLegacyStoreSQLite.Connection(url, writable: true)
                .execute("UPDATE ZPERSISTENTWALLET SET ZNAME='unverified change'")
            XCTAssertThrowsError(try open(url))
            XCTAssertEqual(Set(try operationDirectories(url)), attempts)
            XCTAssertEqual(try Data(contentsOf: marker), originalJournal)
            try DashLegacyStoreSQLite.Connection(url, writable: true)
                .execute("UPDATE ZPERSISTENTWALLET SET ZNAME='historical audit wallet'")
            try autoreleasepool {
                let recovered = try open(url)
                try verifyRows(recovered.mainContext)
            }
            XCTAssertEqual(Set(try operationDirectories(url)), attempts, "The successful recovery open retains its backup")
            let reopened = try open(url)
            try verifyRows(reopened.mainContext)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
    }

    func testJournalRemovalFailureDoesNotExposeAMigratedContainer() throws {
        try withStore { url in
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let marker = root.appendingPathComponent("active.json")
            defer { try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path) }
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .afterCommit {
                    try FileManager.default.setAttributes([.posixPermissions: 0o500], ofItemAtPath: root.path)
                }
            })))
            XCTAssertTrue(FileManager.default.fileExists(atPath: marker.path))
            XCTAssertEqual(try operationDirectories(url).count, 1)
            try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path)
            let recovered = try open(url)
            try verifyRows(recovered.mainContext)
            XCTAssertFalse(FileManager.default.fileExists(atPath: marker.path))
        }
    }

    func testHistoricalStorePreservesDataDefaultsBackupAndDoesNotBridgeAgain() throws {
        try withStore { url in
            try autoreleasepool {
                let container = try DashModelContainer.create(url: url)
                try verifyRows(container.mainContext)
                let wallet = try XCTUnwrap(container.mainContext.fetch(FetchDescriptor<PersistentWallet>()).first)
                wallet.name = "saved after migration"
                try container.mainContext.save()
            }
            let operations = try operationDirectories(url)
            XCTAssertEqual(operations.count, 1)
            let backup = try XCTUnwrap(operations.first).appendingPathComponent("original.store")
            XCTAssertEqual(try DashSchemaFixtureSupport.describeStore(at: backup, version: Schema.Version(1, 0, 0)).model_checksum,
                           "wOm/tD2jkxoKsyP7GFXVNeebjqpLZbKZZ4EqYLkwlMk=")
            let reopened = try open(url, hooks: .init(visit: { _, _ in XCTFail("Completed bridge must not repeat") }))
            try verifyRows(reopened.mainContext, walletName: "saved after migration")
            XCTAssertTrue(try operationDirectories(url).isEmpty, "A later successful open reclaims the retained backup")
        }
    }

    func testHistoricalBridgeAddsEmptyWatermarkStorageAndPreservesNewStampOnReopen() throws {
        try withStore { url in
            try autoreleasepool {
                let container = try DashModelContainer.create(url: url)
                try verifyRows(container.mainContext)
                XCTAssertEqual(try container.mainContext.fetchCount(
                    FetchDescriptor<PersistentIdentityBalanceMetadata>()), 0)
                container.mainContext.insert(PersistentIdentityBalanceMetadata(
                    networkRaw: Network.testnet.rawValue, walletId: Data(repeating: 0x61, count: 32),
                    identityId: Data(repeating: 0x73, count: 32), platformHeight: 42,
                    coreHeight: 7, timestampMillis: 123))
                try container.mainContext.save()
            }
            let reopened = try open(url, hooks: .init(visit: { _, _ in
                XCTFail("Persisting a watermark must not trigger another legacy migration")
            }))
            try verifyRows(reopened.mainContext)
            let metadata = try XCTUnwrap(reopened.mainContext.fetch(
                FetchDescriptor<PersistentIdentityBalanceMetadata>()).first)
            XCTAssertEqual(metadata.walletId, Data(repeating: 0x61, count: 32))
            XCTAssertEqual(metadata.identityId, Data(repeating: 0x73, count: 32))
            XCTAssertEqual(metadata.platformHeight, 42)
            XCTAssertEqual(metadata.coreHeight, 7)
            XCTAssertEqual(metadata.timestampMillis, 123)
        }
    }

    func testWalletDeletionRemovesMigrationSnapshotsWithoutReopeningCachedContainer() throws {
        try withStore { url in
            let survivorId = Data(repeating: 0x63, count: 32)
            // Seed a second wallet into the actual historical fixture before
            // migrating. Both wallet rows must be present in the retained copy.
            try autoreleasepool {
                let connection = try DashLegacyStoreSQLite.Connection(url, writable: true)
                var columns: [String] = []
                try connection.query("PRAGMA table_info(ZPERSISTENTWALLET)") {
                    columns.append(String(cString: sqlite3_column_text($0, 1)))
                }
                let names = columns.map { "\"\($0)\"" }.joined(separator: ",")
                let values = columns.map { column in
                    switch column {
                    case "Z_PK": return "Z_PK + 1"
                    case "ZWALLETID": return "X'" + survivorId.map { String(format: "%02x", $0) }.joined() + "'"
                    case "ZNAME": return "'surviving wallet'"
                    default: return "\"\(column)\""
                    }
                }.joined(separator: ",")
                try connection.execute("INSERT INTO ZPERSISTENTWALLET (\(names)) SELECT \(values) FROM ZPERSISTENTWALLET LIMIT 1")
                try connection.execute("UPDATE Z_PRIMARYKEY SET Z_MAX=(SELECT MAX(Z_PK) FROM ZPERSISTENTWALLET) WHERE Z_NAME='PersistentWallet'")
            }
            let container = try DashModelContainer.create(url: url)
            let snapshot = try XCTUnwrap(operationDirectories(url).first).appendingPathComponent("original.store")
            var originalWalletCount: Int64 = 0
            try DashLegacyStoreSQLite.Connection(snapshot, writable: false).query("SELECT COUNT(*) FROM ZPERSISTENTWALLET") {
                originalWalletCount = sqlite3_column_int64($0, 0)
            }
            XCTAssertEqual(originalWalletCount, 2)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            try handler.deleteWalletData(walletId: Data(repeating: 0x61, count: 32))
            XCTAssertTrue(try operationDirectories(url).isEmpty)
            let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentWallet>())
            XCTAssertEqual(rows.map(\.walletId), [survivorId])
            XCTAssertEqual(rows.first?.name, "surviving wallet")
        }
    }

    func testDeleteAllClearsSnapshotsForEmptyInactiveStoreWithoutTouchingAnotherStore() throws {
        try withStore { activeURL in
            try withStore { inactiveURL in
                let active = try DashModelContainer.create(url: activeURL)
                let inactive = try DashModelContainer.create(url: inactiveURL)
                let activeSnapshots = try operationDirectories(activeURL)
                XCTAssertEqual(activeSnapshots.count, 1)
                // Reproduce an earlier deletion path that removed rows while
                // leaving a snapshot, without reopening the cached container.
                let context = ModelContext(inactive)
                for wallet in try context.fetch(FetchDescriptor<PersistentWallet>()) { context.delete(wallet) }
                try context.save()
                let handler = PlatformWalletPersistenceHandler(modelContainer: inactive, network: .testnet)
                XCTAssertTrue(handler.restorableWalletIds().isEmpty)
                try handler.deleteCompletedMigrationSnapshots()
                XCTAssertTrue(try operationDirectories(inactiveURL).isEmpty)
                XCTAssertEqual(try operationDirectories(activeURL), activeSnapshots)
                XCTAssertEqual(try ModelContext(active).fetchCount(FetchDescriptor<PersistentWallet>()), 1)
                XCTAssertEqual(try ModelContext(inactive).fetchCount(FetchDescriptor<PersistentWallet>()), 0)
            }
        }
    }

    func testSnapshotDeletionFailurePreservesLiveWalletAndCanBeRetried() throws {
        try withStore { url in
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let snapshots = try operationDirectories(url)
            try FileManager.default.setAttributes([.posixPermissions: 0o500], ofItemAtPath: root.path)
            defer { try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path) }
            XCTAssertThrowsError(try handler.deleteWalletData(walletId: Data(repeating: 0x61, count: 32)))
            XCTAssertEqual(try operationDirectories(url), snapshots)
            XCTAssertEqual(try ModelContext(container).fetchCount(FetchDescriptor<PersistentWallet>()), 1)
            try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: root.path)
            try handler.deleteWalletData(walletId: Data(repeating: 0x61, count: 32))
            XCTAssertTrue(try operationDirectories(url).isEmpty)
            XCTAssertEqual(try ModelContext(container).fetchCount(FetchDescriptor<PersistentWallet>()), 0)
        }
    }

    func testSnapshotDeletionRefusesPendingRecoveryBeforeDeletingLiveRows() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .afterCommit { throw Injected.stop }
            })))
            let root = DashLegacySchemaBridge.backupDirectory(for: url)
            let marker = root.appendingPathComponent("active.json")
            let evidence = try Data(contentsOf: marker)
            let snapshots = try operationDirectories(url)
            // Bypass factory recovery only to exercise the deletion boundary
            // when a cached container and a pending recovery marker coexist.
            let schema = DashModelContainer.schema
            let container = try ModelContainer(for: schema, migrationPlan: DashMigrationPlan.self,
                configurations: [ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)])
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            XCTAssertThrowsError(try handler.deleteWalletData(walletId: Data(repeating: 0x61, count: 32)))
            XCTAssertThrowsError(try handler.deleteCompletedMigrationSnapshots())
            XCTAssertEqual(try Data(contentsOf: marker), evidence)
            XCTAssertEqual(try operationDirectories(url), snapshots)
            XCTAssertEqual(try ModelContext(container).fetchCount(FetchDescriptor<PersistentWallet>()), 1)
        }
    }

    func testCommittedWALDataIsIncludedAndConcurrentWriterIsLockedOutAtPromotion() throws {
        try withStore { url in
            let connection = try DashLegacyStoreSQLite.Connection(url, writable: true)
            try connection.execute("PRAGMA journal_mode=WAL")
            try connection.execute("PRAGMA wal_autocheckpoint=0")
            try connection.execute("UPDATE ZPERSISTENTWALLET SET ZNAME='committed in WAL'")
            XCTAssertTrue(FileManager.default.fileExists(atPath: url.path + "-wal"))
            var checkedLock = false
            let migrated = try open(url, hooks: .init(visit: { phase, _ in
                if phase == .writeLocked {
                    XCTAssertThrowsError(try connection.execute("UPDATE ZPERSISTENTWALLET SET ZNAME='racing writer'"))
                    checkedLock = true
                }
            }))
            XCTAssertTrue(checkedLock)
            try verifyRows(migrated.mainContext, walletName: "committed in WAL")
        }
    }

    func testClosedWALStoreMigratesOnColdOpen() throws {
        try withStore { url in
            try autoreleasepool {
                let connection = try DashLegacyStoreSQLite.Connection(url, writable: true)
                try connection.execute("PRAGMA journal_mode=WAL")
                try connection.execute("UPDATE ZPERSISTENTWALLET SET ZNAME='closed WAL store'")
            }
            XCTAssertFalse(FileManager.default.fileExists(atPath: url.path + "-wal"))
            let migrated = try open(url)
            try verifyRows(migrated.mainContext, walletName: "closed WAL store")
        }
    }

    func testCheckpointRejectsBusyWALAndConfirmsDeleteMode() throws {
        try withStore { url in
            try autoreleasepool {
                let writer = try DashLegacyStoreSQLite.Connection(url, writable: true)
                try writer.execute("PRAGMA journal_mode=WAL")
                try writer.execute("PRAGMA wal_autocheckpoint=0")
                let reader = try DashLegacyStoreSQLite.Connection(url, writable: false)
                try reader.execute("BEGIN")
                try reader.query("SELECT ZNAME FROM ZPERSISTENTWALLET") { _ in }
                try writer.execute("UPDATE ZPERSISTENTWALLET SET ZNAME='checkpointed'")
                XCTAssertThrowsError(try DashLegacyStoreSQLite.checkpoint(url))
                try reader.execute("ROLLBACK")
            }
            try DashLegacyStoreSQLite.checkpoint(url)
            let connection = try DashLegacyStoreSQLite.Connection(url, writable: false)
            var mode = ""
            try connection.query("PRAGMA journal_mode") {
                mode = String(cString: sqlite3_column_text($0, 0))
            }
            XCTAssertEqual(mode, "delete")
            XCTAssertFalse(FileManager.default.fileExists(atPath: url.path + "-wal"))
        }
    }

    func testChangedSourceIsNotOverwrittenAndRetryPreservesNewWrite() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, source in
                if phase == .afterSnapshot {
                    try DashLegacyStoreSQLite.Connection(source, writable: true)
                        .execute("UPDATE ZPERSISTENTWALLET SET ZNAME='new concurrent value'")
                }
            })))
            let migrated = try open(url)
            try verifyRows(migrated.mainContext, walletName: "new concurrent value")
        }
    }

    func testFailedValidationAndPrecommitErrorLeaveOriginalUnchanged() throws {
        try withStore { url in
            let original = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, candidate in
                if phase == .afterMigration {
                    try DashLegacyStoreSQLite.Connection(candidate, writable: true)
                        .execute("DELETE FROM ZPERSISTENTINDEX")
                }
            })))
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            XCTAssertTrue(try operationDirectories(url).isEmpty, "Uncommitted copies must not accumulate on retries")
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, candidate in
                if phase == .afterMigration {
                    try DashLegacyStoreSQLite.Connection(candidate, writable: true)
                        .execute("UPDATE ZPERSISTENTINDEX SET ZDOCUMENTTYPE=NULL")
                }
            })), "A same-count relationship change must fail validation")
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .writeLocked { throw Injected.stop }
            })))
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            let recovered = try open(url)
            try verifyRows(recovered.mainContext)
        }
    }

    func testAfterCommitInterruptionRecoversWithoutRepeatingMigration() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .afterCommit { throw Injected.stop }
            })))
            let marker = DashLegacySchemaBridge.backupDirectory(for: url).appendingPathComponent("active.json")
            XCTAssertTrue(FileManager.default.fileExists(atPath: marker.path))
            let recovered = try open(url, hooks: .init(visit: { _, _ in XCTFail("Committed candidate should only recover") }))
            try verifyRows(recovered.mainContext)
            XCTAssertFalse(FileManager.default.fileExists(atPath: marker.path))
            XCTAssertEqual(try operationDirectories(url).count, 1)
        }
    }

    func testPendingRecoveryWithMissingOriginalRequiresDeliberateRecovery() throws {
        for removeBackup in [false, true] {
            try withStore { url in
                XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                    if phase == .beforeInstall { throw Injected.stop }
                })))
                let root = DashLegacySchemaBridge.backupDirectory(for: url)
                let marker = root.appendingPathComponent("active.json")
                let journal = try Data(contentsOf: marker)
                let displaced = url.deletingLastPathComponent().appendingPathComponent("displaced.store")
                try FileManager.default.moveItem(at: url, to: displaced)
                if removeBackup {
                    for directory in try operationDirectories(url) { try FileManager.default.removeItem(at: directory) }
                }
                XCTAssertThrowsError(try open(url)) { error in
                    XCTAssertTrue(error.localizedDescription.contains("Restore the original database from a verified backup"))
                    XCTAssertTrue(error.localizedDescription.contains(root.path))
                }
                XCTAssertFalse(FileManager.default.fileExists(atPath: url.path), "Never resurrect a possibly intentional reset")
                XCTAssertEqual(try Data(contentsOf: marker), journal, "Preserve recovery evidence even when copies are absent")
                XCTAssertTrue(FileManager.default.fileExists(atPath: displaced.path))
                // Deliberate restoration of the authoritative source unblocks
                // normal recovery; the bridge can recreate missing scratch files.
                try FileManager.default.moveItem(at: displaced, to: url)
                let recovered = try open(url)
                try verifyRows(recovered.mainContext)
            }
        }
    }

    func testRecoveryWorksWithoutScratchFilesBeforeAndAfterCommit() throws {
        for phase: DashLegacySchemaBridge.Phase in [.beforeInstall, .afterCommit] {
            try withStore { url in
                XCTAssertThrowsError(try open(url, hooks: .init(visit: { current, _ in
                    if current == phase { throw Injected.stop }
                })))
                for directory in try operationDirectories(url) {
                    try FileManager.default.removeItem(at: directory)
                }
                let recovered = try open(url)
                try verifyRows(recovered.mainContext)
                XCTAssertFalse(FileManager.default.fileExists(atPath:
                    DashLegacySchemaBridge.backupDirectory(for: url).appendingPathComponent("active.json").path))
            }
        }
    }

    func testMissingCandidateCannotHideChangedInstalledRows() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .afterCommit { throw Injected.stop }
            })))
            for directory in try operationDirectories(url) {
                try FileManager.default.removeItem(at: directory)
            }
            try DashLegacyStoreSQLite.Connection(url, writable: true)
                .execute("DELETE FROM ZPERSISTENTINDEX")
            XCTAssertThrowsError(try open(url))
            XCTAssertTrue(FileManager.default.fileExists(atPath:
                DashLegacySchemaBridge.backupDirectory(for: url).appendingPathComponent("active.json").path))
        }
    }

    func testFailedOrdinaryOpenDoesNotReclaimRetainedBackup() throws {
        try withStore { url in
            try autoreleasepool { _ = try open(url) }
            let directories = try operationDirectories(url)
            XCTAssertEqual(directories.count, 1)
            try Data("unreadable store".utf8).write(to: url)
            XCTAssertThrowsError(try open(url))
            XCTAssertEqual(try operationDirectories(url), directories)
        }
    }

    func testCorruptionExternalStorageAndNewerUnknownVersionDoNotBridge() throws {
        try withStore { url in
            try Data("not a database".utf8).write(to: url)
            let bytes = try Data(contentsOf: url)
            XCTAssertThrowsError(try open(url))
            XCTAssertEqual(try Data(contentsOf: url), bytes)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
        try withStore { url in
            let support = url.deletingLastPathComponent().appendingPathComponent("." + url.lastPathComponent + "_SUPPORT")
            try FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
            let original = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try open(url))
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
        try withStore { url in
            var metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
            metadata[NSStoreModelVersionIdentifiersKey] = ["2.0.0"]
            try NSPersistentStoreCoordinator.setMetadata(metadata, type: .sqlite, at: url)
            let copy = url.deletingLastPathComponent().appendingPathComponent("newer-original.store")
            try DashLegacyStoreSQLite.copy(from: url, to: copy)
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { _, _ in XCTFail("Newer unknown store must not bridge") })))
            // The ordinary SwiftData path may switch journal modes before
            // rejecting an unknown version; application values must remain intact.
            try DashLegacyStoreSQLite.validatePreservation(from: copy, to: url)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
    }

    func testUnknownEntityAndCandidateExternalStorageAreRejected() throws {
        try withStore { url in
            var metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
            var hashes = try XCTUnwrap(metadata[NSStoreModelVersionHashesKey] as? [String: Data])
            hashes["UnknownEntity"] = Data(repeating: 1, count: 32)
            metadata[NSStoreModelVersionHashesKey] = hashes
            try NSPersistentStoreCoordinator.setMetadata(metadata, type: .sqlite, at: url)
            XCTAssertThrowsError(try open(url))
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
        try withStore { url in
            let original = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, candidate in
                if phase == .afterMigration {
                    let support = candidate.deletingLastPathComponent().appendingPathComponent("." + candidate.lastPathComponent + "_SUPPORT")
                    try FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
                }
            })))
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), original)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
    }

    func testSkippingV3UsesFixedBridgeThenRegisteredCustomFutureStage() throws {
        try withStore { url in
            let schema = Schema(versionedSchema: BridgeFutureV4.self)
            var checkedV3 = false
            let configuration = ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            XCTAssertThrowsError(try DashLegacySchemaBridge.open(
                configuration: configuration, schema: schema, plan: BridgeFuturePlan.self,
                hooks: .init(visit: { phase, candidate in
                    if phase == .afterMigration {
                        _ = try DashSchemaFixtureSupport.describeStore(at: candidate, version: Schema.Version(3, 0, 0))
                        checkedV3 = true
                    }
                    if phase == .afterCommit { throw Injected.stop }
                })))
            XCTAssertTrue(checkedV3)
            let recovered = try DashLegacySchemaBridge.open(
                configuration: configuration, schema: schema, plan: BridgeFuturePlan.self)
            try verifyRows(recovered.mainContext, walletName: "explicit future transformation")
            XCTAssertEqual(try recovered.mainContext.fetchCount(FetchDescriptor<BridgeFutureMarker>()), 0)
        }
    }

    func testValidatorRejectsColumnTypeChangesAndPreservesTypedValues() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let original = directory.appendingPathComponent("source.store")
        let candidate = directory.appendingPathComponent("candidate.store")
        let source = try DashLegacyStoreSQLite.Connection(original, writable: true, create: true)
        try source.execute("CREATE TABLE Z_PRIMARYKEY(Z_ENT INTEGER,Z_NAME TEXT)")
        try source.execute("INSERT INTO Z_PRIMARYKEY VALUES(1,'PersistentWallet')")
        try source.execute("CREATE TABLE ZPERSISTENTWALLET(Z_PK INTEGER,Z_ENT INTEGER,Z_OPT INTEGER,V BLOB,T TEXT,N INTEGER)")
        try source.execute("INSERT INTO ZPERSISTENTWALLET VALUES(1,1,1,X'000100',CAST(X'610062' AS TEXT),9223372036854775807)")
        try DashLegacyStoreSQLite.copy(from: original, to: candidate)
        try DashLegacyStoreSQLite.validatePreservation(from: original, to: candidate)
        let copy = try DashLegacyStoreSQLite.Connection(candidate, writable: true)
        try copy.execute("UPDATE ZPERSISTENTWALLET SET T='a'")
        XCTAssertThrowsError(try DashLegacyStoreSQLite.validatePreservation(from: original, to: candidate), "Embedded NUL bytes must be compared")
        try copy.execute("DROP TABLE ZPERSISTENTWALLET")
        try copy.execute("CREATE TABLE ZPERSISTENTWALLET(Z_PK INTEGER,Z_ENT INTEGER,Z_OPT INTEGER,V TEXT,T TEXT,N INTEGER)")
        try copy.execute("INSERT INTO ZPERSISTENTWALLET VALUES(1,1,1,X'000100',CAST(X'610062' AS TEXT),9223372036854775807)")
        XCTAssertThrowsError(try DashLegacyStoreSQLite.validatePreservation(from: original, to: candidate), "Declared storage type changes must be rejected even when cells agree")
        try copy.execute("ALTER TABLE ZPERSISTENTWALLET DROP COLUMN T")
        XCTAssertThrowsError(try DashLegacyStoreSQLite.validatePreservation(from: original, to: candidate), "Removed original columns must fail validation")
    }

    func testLargeHistoricalStoreMigratesAsynchronouslyWhileMainActorRemainsResponsive() async throws {
        let (directory, url) = try makeStore()
        defer { try? FileManager.default.removeItem(at: directory) }
        // Synthetic storage load only: these payloads are not broadcastable
        // transactions or a production wallet. Populate the pinned old layout.
        try autoreleasepool {
            let connection = try DashLegacyStoreSQLite.Connection(url, writable: true)
            try connection.execute("""
                WITH RECURSIVE rows(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM rows WHERE n<10000)
                INSERT INTO ZPERSISTENTTRANSACTION
                (Z_PK,Z_ENT,Z_OPT,ZBLOCKHEIGHT,ZBLOCKPOSITION,ZBLOCKTIMESTAMP,ZCONTEXT,ZDIRECTION,
                 ZFIRSTSEEN,ZHASBLOCKPOSITION,ZNETAMOUNT,ZPROVIDERCOLLATERALVOUT,ZTRANSACTIONTYPEKIND,
                 ZCREATEDAT,ZLASTUPDATED,ZLABEL,ZTRANSACTIONTYPE,ZTRANSACTIONDATA,ZTXID)
                SELECT n,(SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME='PersistentTransaction'),1,
                       0,0,0,0,0,n,0,0,0,255,0,0,'synthetic','Standard',zeroblob(8192),
                       CAST(printf('%032d',n) AS BLOB) FROM rows;
                UPDATE Z_PRIMARYKEY SET Z_MAX=10000 WHERE Z_NAME='PersistentTransaction';
                """)
        }
        let bytes = (try FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber)?.int64Value ?? 0
        let heartbeat = Task { @MainActor in
            var ticks = 0
            var longestGap = 0.0
            var previous = Date.timeIntervalSinceReferenceDate
            while !Task.isCancelled {
                // Nonthrowing tick: cancellation must not produce a secondary
                // CancellationError while reporting an actual migration failure.
                await withCheckedContinuation { continuation in
                    DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(10)) {
                        continuation.resume()
                    }
                }
                guard !Task.isCancelled else { break }
                let now = Date.timeIntervalSinceReferenceDate
                longestGap = max(longestGap, now - previous)
                previous = now
                ticks += 1
            }
            return (ticks, longestGap)
        }
        defer { heartbeat.cancel() }
        let started = Date.timeIntervalSinceReferenceDate
        let container: ModelContainer
        do {
            container = try await DashModelContainer.createAsync(url: url)
        } catch {
            heartbeat.cancel()
            _ = await heartbeat.value
            throw error // Preserve the migration failure after draining the tick.
        }
        let elapsed = Date.timeIntervalSinceReferenceDate - started
        heartbeat.cancel()
        let (ticks, longestGap) = await heartbeat.value
        XCTAssertGreaterThan(ticks, 1, "The main actor must keep executing while migration runs")
        XCTAssertEqual(try container.mainContext.fetchCount(FetchDescriptor<PersistentTransaction>()), 10_000)
        try verifyRows(container.mainContext)
        let report = "Legacy migration benchmark: bytes=\(bytes) transactions=10000 seconds=\(elapsed) mainTicks=\(ticks) maxMainGap=\(longestGap)"
        Swift.print(report)
        let attachment = XCTAttachment(string: report)
        attachment.name = "legacy-migration-benchmark"
        attachment.lifetime = .keepAlways
        add(attachment)
        let reopened = try await DashModelContainer.createAsync(url: url)
        XCTAssertEqual(try reopened.mainContext.fetchCount(FetchDescriptor<PersistentTransaction>()), 10_000)
    }

    func testExactPreviousLiveV2RemainsWritableWithoutReinterpretingHistoricalV2() throws {
        try withStore { url in
            try autoreleasepool {
                let schema = Schema(versionedSchema: PreviousLiveV2.self)
                let container = try ModelContainer(for: schema, configurations: [
                    ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
                ])
                try verifyRows(container.mainContext)
            }
            XCTAssertEqual(try DashLegacySchemaBridge.identity(at: url).versions, ["2.0.0"])
            let container = try open(url, hooks: .init(visit: { _, _ in
                XCTFail("An exact previous current shape needs no legacy inferred migration")
            }))
            try verifyRows(container.mainContext)
            let wallet = try XCTUnwrap(container.mainContext.fetch(FetchDescriptor<PersistentWallet>()).first)
            wallet.name = "previous capture remains writable"
            try container.mainContext.save()
        }
    }

    func testUnsupportedBetaV2FailsWithoutMutation() throws {
        try withStore { url in
            try autoreleasepool {
                let schema = Schema(versionedSchema: UnsupportedBetaV2.self)
                _ = try ModelContainer(for: schema, configurations: [
                    ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
                ])
            }
            try DashLegacyStoreSQLite.checkpoint(url)
            let before = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try open(url)) { error in
                XCTAssertTrue(error.localizedDescription.contains("does not match"))
            }
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), before)
            XCTAssertTrue(try operationDirectories(url).isEmpty)
        }
    }

    func testPreviousLiveV2JournalRecoversBeforeChoosingMigrationRoute() throws {
        struct OldJournal: Encodable {
            let formatVersion: Int
            let operation: UUID
            let source: DashLegacySchemaBridge.Identity
            let destination: DashLegacySchemaBridge.Identity
            let destinationData: DashLegacyStoreSQLite.StoreEvidence?
        }
        for format in [1, 2] {
            for installed in [false, true] {
                try withStore { url in
                    let operation = UUID()
                    let root = DashLegacySchemaBridge.backupDirectory(for: url)
                    let directory = root.appendingPathComponent(operation.uuidString)
                    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                    let backup = directory.appendingPathComponent("original.store")
                    let candidate = directory.appendingPathComponent("candidate.store")
                    try DashLegacyStoreSQLite.copy(from: url, to: backup)
                    try DashLegacyStoreSQLite.copy(from: url, to: candidate)
                    let source = try DashLegacySchemaBridge.identity(at: url)
                    try autoreleasepool {
                        let schema = Schema(versionedSchema: PreviousLiveV2.self)
                        _ = try ModelContainer(for: schema, configurations: [
                            ModelConfiguration(schema: schema, url: candidate, cloudKitDatabase: .none)
                        ])
                    }
                    try DashLegacyStoreSQLite.checkpoint(candidate)
                    try DashLegacyStoreSQLite.validatePreservation(from: backup, to: candidate)
                    let journal = OldJournal(formatVersion: format, operation: operation, source: source,
                        destination: try DashLegacySchemaBridge.identity(at: candidate),
                        destinationData: format == 2 ? try DashLegacyStoreSQLite.evidence(at: candidate) : nil)
                    if installed { try DashLegacyStoreSQLite.copy(from: candidate, to: url) }
                    let marker = root.appendingPathComponent("active.json")
                    try JSONEncoder().encode(journal).write(to: marker)
                    let container = try open(url)
                    try verifyRows(container.mainContext)
                    XCTAssertFalse(FileManager.default.fileExists(atPath: marker.path))
                }
            }
        }
    }
}
