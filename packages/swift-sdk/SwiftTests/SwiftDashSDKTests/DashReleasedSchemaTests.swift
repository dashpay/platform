import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

struct DashReleasedSchemaFixture: Sendable {
    let version: any VersionedSchema.Type
    let resourceName: String
}

/// Publication adds archival snapshots, never additional runtime stages. These
/// checks deliberately fail if development changed a published version in place:
/// move the old version onto its snapshot, add a live version and a migration.
final class DashReleasedSchemaTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        _ = DashModelContainer.schema
    }

    private func source(_ fixture: DashReleasedSchemaFixture) throws -> URL {
        try XCTUnwrap(Bundle.module.url(
            forResource: fixture.resourceName, withExtension: "store",
            subdirectory: "Fixtures/SchemaStores/releases"))
    }

    @MainActor
    func testPublishedSnapshotsAndRuntimeVersionsMatchCapturedStores() throws {
        for fixture in DashReleasedSchemaRegistry.fixtures {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: directory) }
            let version = fixture.version.versionIdentifier
            let expected = try DashSchemaFixtureSupport.describeStore(at: source(fixture), version: version)
            let registered = try XCTUnwrap(DashMigrationPlan.schemas.first {
                $0.versionIdentifier == version
            }, "A published version is missing from the runtime migration plan")
            for (name, type) in [("snapshot", fixture.version), ("runtime", registered)] {
                let url = directory.appendingPathComponent("\(name).store")
                try autoreleasepool {
                    let schema = Schema(versionedSchema: type)
                    _ = try ModelContainer(for: schema, configurations: [
                        ModelConfiguration(name, schema: schema, url: url, cloudKitDatabase: .none)
                    ])
                }
                XCTAssertEqual(
                    try DashSchemaFixtureSupport.describeStore(at: url, version: version), expected,
                    "\(name) changed published \(expected.schema_version); retire it onto its snapshot and introduce a new live version")
            }
        }
    }

    @MainActor
    func testPublishedStoresMigrateAndRemainWritableThroughLiveTypes() throws {
        for fixture in DashReleasedSchemaRegistry.fixtures {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: directory) }
            let url = directory.appendingPathComponent("migrated.store")
            try FileManager.default.copyItem(at: source(fixture), to: url)
            let container = try DashModelContainer.create(url: url)
            let context = container.mainContext
            let wallet = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentWallet>()).first)
            XCTAssertEqual(wallet.walletId, Data(repeating: 0x31, count: 32))
            XCTAssertEqual(wallet.name, "fixture wallet")
            XCTAssertEqual(wallet.accounts.count, 1)
            XCTAssertEqual(wallet.accounts.first?.coreAddresses.first?.address, "yFixtureAddress")
            let txo = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentTxo>()).first)
            XCTAssertEqual(txo.amount, 1_000)
            XCTAssertEqual(txo.transaction?.txid, Data(repeating: 0x34, count: 32))
            XCTAssertEqual(txo.spendingTransaction?.txid, Data(repeating: 0x32, count: 32))
            XCTAssertEqual(txo.account?.wallet.walletId, wallet.walletId)
            XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentPendingInput>()), 1)
            XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentTrackedMasternode>()), 1)
            XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentKeyword>()).first?.keyword, "preserved")
            XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentIdentity>()).first?.balance, 5)
            let lock = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentAssetLock>()).first)
            XCTAssertEqual(lock.amountDuffs, 100_000)
            XCTAssertEqual(lock.recipientIsExternal, true)
            XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentTransaction>()), 2)
            wallet.name = "migrated and writable"
            try context.save()
            XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentWallet>()).first?.name, "migrated and writable")
            let fresh = directory.appendingPathComponent("fresh.store")
            _ = try DashModelContainer.create(url: fresh)
            XCTAssertTrue(Set(try DashSchemaFixtureSupport.indexes(at: fresh))
                .isSubset(of: Set(try DashSchemaFixtureSupport.indexes(at: url))))
        }
    }

    @MainActor
    func testRuntimePlanHasNoDuplicateModelChecksums() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        var checksums = Set<String>()
        for type in DashMigrationPlan.schemas {
            let version = type.versionIdentifier
            let url = directory.appendingPathComponent(DashSchemaFixtureSupport.version(version) + ".store")
            let schema = Schema(versionedSchema: type)
            _ = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let metadata = try DashSchemaFixtureSupport.describeStore(at: url, version: version)
            XCTAssertTrue(checksums.insert(metadata.model_checksum).inserted,
                          "A snapshot with unchanged shape must not become an additional runtime migration stage")
        }
    }
}
