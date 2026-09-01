import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

final class DashModelMigrationTests: XCTestCase {
    @MainActor
    func testV1StoreMigratesToV2AndAcceptsTrackedMasternodes() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let v1Schema = Schema(versionedSchema: DashSchemaV1.self)
        let v1Configuration = ModelConfiguration(
            "DashMigrationTest",
            schema: v1Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v1Container: ModelContainer? = try ModelContainer(
            for: v1Schema,
            configurations: [v1Configuration])
        v1Container?.mainContext.insert(PersistentKeyword(
            keyword: "preserved",
            contractId: "contract"))
        try v1Container?.mainContext.save()
        v1Container = nil

        let v2Schema = Schema(versionedSchema: DashSchemaV2.self)
        let v2Configuration = ModelConfiguration(
            "DashMigrationTest",
            schema: v2Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        let migrated = try ModelContainer(
            for: v2Schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [v2Configuration])

        let keywords = try migrated.mainContext.fetch(FetchDescriptor<PersistentKeyword>())
        XCTAssertEqual(keywords.map(\.keyword), ["preserved"])

        migrated.mainContext.insert(PersistentTrackedMasternode(
            networkRaw: Network.testnet.rawValue,
            proTxHash: Data(repeating: 7, count: 32),
            label: "new in V2",
            addedAt: 1,
            snapshotJSON: "{}"))
        try migrated.mainContext.save()
        XCTAssertEqual(
            try migrated.mainContext.fetchCount(
                FetchDescriptor<PersistentTrackedMasternode>()),
            1)
    }

    /// Guards the freeze itself: `DashSchemaV1.PersistentAssetLock` only
    /// keeps V1/V2 stores openable if SwiftData names its entity
    /// "PersistentAssetLock" — i.e. from the UNQUALIFIED type name. If a
    /// future SwiftData release qualified nested types instead, the frozen
    /// copy would silently register a *different* entity and the V2 -> V3
    /// stage would become a drop+create rather than an add-column, so this
    /// has to fail loudly rather than in the field.
    func testFrozenAssetLockKeepsTheLiveEntityName() throws {
        for schema in [
            Schema(versionedSchema: DashSchemaV1.self),
            Schema(versionedSchema: DashSchemaV2.self),
            Schema(versionedSchema: DashSchemaV3.self)
        ] {
            let names = schema.entities.map(\.name)
            XCTAssertTrue(
                names.contains("PersistentAssetLock"),
                "expected an entity named PersistentAssetLock, got \(names.sorted())")
        }

        // V2 and V3 differ ONLY in that one entity's shape, never in which
        // entities exist — that is what makes the stage lightweight.
        XCTAssertEqual(
            Schema(versionedSchema: DashSchemaV2.self).entities.map(\.name).sorted(),
            Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name).sorted())

        // V1 -> V2 remains exactly "add PersistentTrackedMasternode".
        XCTAssertEqual(
            Set(Schema(versionedSchema: DashSchemaV2.self).entities.map(\.name))
                .subtracting(Schema(versionedSchema: DashSchemaV1.self).entities.map(\.name)),
            ["PersistentTrackedMasternode"])
    }

    /// The regression this whole freeze exists for: a store written by the
    /// schema-V2 definition of `PersistentAssetLock` (no `recipientIsExternal`)
    /// must still open once the live model has grown that property.
    ///
    /// The source store is created from `DashSchemaV2`, which references the
    /// frozen `DashSchemaV1.PersistentAssetLock` — not the live type — so
    /// this exercises the real cross-version path rather than trivially
    /// round-tripping today's model.
    @MainActor
    func testV2AssetLockStoreMigratesToV3AndBackfillsRecipientIsExternal() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let outPointHex = String(repeating: "ab", count: 32) + ":0"
        let walletId = Data(repeating: 3, count: 32)

        let v2Schema = Schema(versionedSchema: DashSchemaV2.self)
        let v2Configuration = ModelConfiguration(
            "DashAssetLockMigrationTest",
            schema: v2Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v2Container: ModelContainer? = try ModelContainer(
            for: v2Schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [v2Configuration])
        let legacyRow = DashSchemaV1.PersistentAssetLock(
            outPointHex: outPointHex,
            walletId: walletId,
            transactionBytes: Data([1, 2, 3]),
            fundingTypeRaw: 4,
            identityIndexRaw: -1,
            accountIndexRaw: 0,
            amountDuffs: 100_000,
            statusRaw: 4)
        legacyRow.recipientPlatformAddressHash = Data(repeating: 9, count: 20)
        legacyRow.recipientPlatformAddressType = 0
        v2Container?.mainContext.insert(legacyRow)
        try v2Container?.mainContext.save()
        v2Container = nil

        // Reopen exactly the way `DashModelContainer.create` does.
        let v3Schema = Schema(versionedSchema: DashSchemaV3.self)
        let v3Configuration = ModelConfiguration(
            "DashAssetLockMigrationTest",
            schema: v3Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        let migrated = try ModelContainer(
            for: v3Schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [v3Configuration])

        let locks = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentAssetLock>())
        XCTAssertEqual(locks.count, 1)
        let lock = try XCTUnwrap(locks.first)
        XCTAssertEqual(lock.outPointHex, outPointHex)
        XCTAssertEqual(lock.walletId, walletId)
        XCTAssertEqual(lock.recipientPlatformAddressHash, Data(repeating: 9, count: 20))
        XCTAssertEqual(lock.recipientPlatformAddressType, 0)
        // Backfilled NULL — the documented "treat as own" signal.
        XCTAssertNil(lock.recipientIsExternal)

        // And the new column is writable on the migrated row.
        lock.recipientIsExternal = true
        try migrated.mainContext.save()
        XCTAssertEqual(
            try migrated.mainContext.fetch(FetchDescriptor<PersistentAssetLock>())
                .first?.recipientIsExternal,
            true)
    }
}
