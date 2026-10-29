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
        try XCTSkipIf(
            DashReleasedSchemaRegistry.fixtures.isEmpty,
            "No App Store schema snapshots have been registered yet")
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
        try XCTSkipIf(
            DashReleasedSchemaRegistry.fixtures.isEmpty,
            "No App Store schema snapshots have been registered yet")
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
            let key = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentPublicKey>()).first)
            XCTAssertEqual(key.keyId, 3)
            XCTAssertEqual(key.totalBudgetCredits, 100_000)
            XCTAssertEqual(key.expiresAtMillis, 1_800_000_000_000)
            XCTAssertEqual(key.identity?.identityId, Data(repeating: 0x35, count: 32))
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

    private enum BaseV4Plan: SchemaMigrationPlan {
        static var schemas: [any VersionedSchema.Type] { [DashSchemaV3.self, DashSchemaV4.self] }
        static var stages: [MigrationStage] {
            [.lightweight(fromVersion: DashSchemaV3.self, toVersion: DashSchemaV4.self)]
        }
    }

    /// V4 → V5 adds the per-wallet accounting entity: migrated rows start
    /// without one and keep their stored amount, and a row written after the
    /// migration survives a reopen and cascades with its transaction.
    @MainActor
    func testShouldMigrateV4ToPerWalletAccountingRows() throws {
        let fixture = try XCTUnwrap(DashReleasedSchemaRegistry.fixtures.first {
            $0.version.versionIdentifier == Schema.Version(3, 0, 0)
        })
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("accounting.store")
        try FileManager.default.copyItem(at: source(fixture), to: url)
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV4.self)
            let container = try ModelContainer(for: schema, migrationPlan: BaseV4Plan.self,
                configurations: [ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)])
            try container.mainContext.save()
        }
        XCTAssertEqual(try DashLegacySchemaBridge.identity(at: url).versions, ["4.0.0"])
        let txid = Data(repeating: 0x32, count: 32)
        let walletId = Data(repeating: 0x07, count: 32)
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let row = try XCTUnwrap(container.mainContext.fetch(FetchDescriptor<PersistentTransaction>())
                .first { $0.txid == txid })
            XCTAssertTrue(row.walletAccounting.isEmpty, "V4 rows carry no per-wallet accounting yet")
            XCTAssertEqual(row.netAmount(for: walletId), row.netAmount)
            let accounting = PersistentTransactionAccounting(
                walletId: walletId, txid: txid, netAmount: -1_234,
                direction: CoreDirectionCode.outgoing, fee: 34, transaction: nil
            )
            container.mainContext.insert(accounting)
            row.walletAccounting.append(accounting)
            try container.mainContext.save()
        }
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let row = try XCTUnwrap(container.mainContext.fetch(FetchDescriptor<PersistentTransaction>())
                .first { $0.txid == txid })
            XCTAssertEqual(row.netAmount(for: walletId), -1_234)
            XCTAssertEqual(row.direction(for: walletId), CoreDirectionCode.outgoing)
            XCTAssertEqual(row.fee(for: walletId), 34)
            XCTAssertEqual(try DashLegacySchemaBridge.identity(at: url).versions, ["5.0.0"])
            container.mainContext.delete(row)
            try container.mainContext.save()
            XCTAssertTrue(try container.mainContext.fetch(FetchDescriptor<PersistentTransactionAccounting>()).isEmpty)
        }
    }

    @MainActor
    func testRuntimePlanHasNoDuplicateModelChecksums() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let plans: [any SchemaMigrationPlan.Type] = [DashMigrationPlan.self, DashAcceptedV1MigrationPlan.self]
        for (planIndex, plan) in plans.enumerated() {
            var checksums = Set<String>()
            for type in plan.schemas {
                let version = type.versionIdentifier
                let url = directory.appendingPathComponent("\(planIndex)-" + DashSchemaFixtureSupport.version(version) + ".store")
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
}
