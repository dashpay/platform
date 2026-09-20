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
private enum BridgeFutureV3: VersionedSchema {
    static var versionIdentifier: Schema.Version { Schema.Version(3, 0, 0) }
    static var models: [any PersistentModel.Type] { DashSchemaV2.models + [BridgeFutureMarker.self] }
}
private enum BridgeFuturePlan: SchemaMigrationPlan {
    static var schemas: [any VersionedSchema.Type] { [DashSchemaV1.self, DashSchemaV2.self, BridgeFutureV3.self] }
    static var stages: [MigrationStage] {
        [.lightweight(fromVersion: DashSchemaV1.self, toVersion: DashSchemaV2.self),
         .custom(fromVersion: DashSchemaV2.self, toVersion: BridgeFutureV3.self,
                 willMigrate: { context in
                     for wallet in try context.fetch(FetchDescriptor<PersistentWallet>()) {
                         wallet.name = "explicit future transformation"
                     }
                     try context.save()
                 }, didMigrate: nil)]
    }
}

@MainActor
final class DashLegacySchemaMigrationTests: XCTestCase {
    private enum Injected: Error { case stop }

    private func withStore(baseline: Bool = false, _ body: (URL) throws -> Void) throws {
        let source = try XCTUnwrap(Bundle.module.url(
            forResource: baseline ? "dash-v1" : "fixture", withExtension: "store",
            subdirectory: baseline ? "Fixtures/SchemaStores" : "Fixtures/SchemaStores/legacy-fd8d8d13e5"))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("DashModel.store")
        try FileManager.default.copyItem(at: source, to: url)
        try autoreleasepool { try body(url) }
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
            let current = try open(url) // Current V2.
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

    func testPendingRecoveryWithMissingOriginalNeverCreatesEmptyDatabase() throws {
        try withStore { url in
            XCTAssertThrowsError(try open(url, hooks: .init(visit: { phase, _ in
                if phase == .beforeInstall { throw Injected.stop }
            })))
            let displaced = url.deletingLastPathComponent().appendingPathComponent("displaced.store")
            try FileManager.default.moveItem(at: url, to: displaced)
            XCTAssertThrowsError(try open(url))
            XCTAssertFalse(FileManager.default.fileExists(atPath: url.path))
            XCTAssertTrue(FileManager.default.fileExists(atPath: displaced.path))
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

    func testSkippingV2UsesFixedBridgeThenRegisteredCustomFutureStage() throws {
        try withStore { url in
            let schema = Schema(versionedSchema: BridgeFutureV3.self)
            var checkedV2 = false
            let configuration = ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            XCTAssertThrowsError(try DashLegacySchemaBridge.open(
                configuration: configuration, schema: schema, plan: BridgeFuturePlan.self,
                hooks: .init(visit: { phase, candidate in
                    if phase == .afterMigration {
                        _ = try DashSchemaFixtureSupport.describeStore(at: candidate, version: Schema.Version(2, 0, 0))
                        checkedV2 = true
                    }
                    if phase == .afterCommit { throw Injected.stop }
                })))
            XCTAssertTrue(checkedV2)
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
}
