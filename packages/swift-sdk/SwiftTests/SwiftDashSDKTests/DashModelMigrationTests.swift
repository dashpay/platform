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

    /// Guards the frozen models' entity identity. SwiftData must derive each
    /// nested type's entity name from the unqualified type name so migration
    /// stages add columns to existing tables instead of dropping and creating
    /// unrelated entities.
    func testFrozenModelsKeepTheLiveEntityNames() throws {
        for schema in [
            Schema(versionedSchema: DashSchemaV1.self),
            Schema(versionedSchema: DashSchemaV2.self),
            Schema(versionedSchema: DashSchemaV3.self),
            Schema(versionedSchema: DashSchemaV4.self)
        ] {
            let names = schema.entities.map(\.name)
            for expected in [
                "PersistentAssetLock",
                "PersistentWallet",
                "PersistentTransaction",
                "PersistentTxo",
                "PersistentPendingInput"
            ] {
                XCTAssertTrue(
                    names.contains(expected),
                    "expected an entity named \(expected), got \(names.sorted())")
            }
        }

        // Successive versions alter shapes, not entity membership.
        XCTAssertEqual(
            Schema(versionedSchema: DashSchemaV2.self).entities.map(\.name).sorted(),
            Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name).sorted())
        XCTAssertEqual(
            Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name).sorted(),
            Schema(versionedSchema: DashSchemaV4.self).entities.map(\.name).sorted())

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

    /// A source store must be constructed from the exact historical model
    /// types. Using today's live classes here would create today's checksum
    /// and let an accidentally mutated historical schema pass unnoticed.
    @MainActor
    func testV2WalletGraphStoreMigratesToV4AndBackfillsSweepFields() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let walletId = Data(repeating: 0x31, count: 32)
        let txid = Data(repeating: 0x32, count: 32)
        let v2Schema = Schema(versionedSchema: DashSchemaV2.self)
        let v2Configuration = ModelConfiguration(
            "DashSweepMigrationTest",
            schema: v2Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v2Container: ModelContainer? = try ModelContainer(
            for: v2Schema,
            configurations: [v2Configuration])

        let legacyWallet = DashSchemaV1.PersistentWallet(
            walletId: walletId,
            network: .testnet,
            name: "legacy wallet")
        let legacyTransaction = DashSchemaV1.PersistentTransaction(
            txid: txid,
            transactionData: Data([1, 2, 3]),
            direction: 1,
            netAmount: -1_000)
        let legacyTxo = DashSchemaV1.PersistentTxo(
            transaction: legacyTransaction,
            vout: 0,
            amount: 1_000,
            address: "yLegacyAddress")
        legacyTxo.walletId = walletId
        let legacyPendingInput = DashSchemaV1.PersistentPendingInput(
            outpoint: Data(repeating: 0x33, count: 36),
            inputIndex: 0,
            spendingTxid: txid,
            spendingTransaction: legacyTransaction,
            walletId: walletId)
        v2Container?.mainContext.insert(legacyWallet)
        v2Container?.mainContext.insert(legacyTransaction)
        v2Container?.mainContext.insert(legacyTxo)
        v2Container?.mainContext.insert(legacyPendingInput)
        try v2Container?.mainContext.save()
        v2Container = nil

        let v4Schema = Schema(versionedSchema: DashSchemaV4.self)
        let v4Configuration = ModelConfiguration(
            "DashSweepMigrationTest",
            schema: v4Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        let migrated = try ModelContainer(
            for: v4Schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [v4Configuration])

        let wallets = try migrated.mainContext.fetch(FetchDescriptor<PersistentWallet>())
        XCTAssertEqual(wallets.map(\.walletId), [walletId])
        XCTAssertNil(try XCTUnwrap(wallets.first).lastAppliedChainLockHeight)

        let transactions = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentTransaction>())
        XCTAssertEqual(transactions.map(\.txid), [txid])
        XCTAssertFalse(try XCTUnwrap(transactions.first).isGloballySwept)

        let txos = try migrated.mainContext.fetch(FetchDescriptor<PersistentTxo>())
        XCTAssertEqual(txos.count, 1)
        XCTAssertNil(try XCTUnwrap(txos.first).supersededByTxid)

        let pendingInputs = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentPendingInput>())
        XCTAssertEqual(pendingInputs.count, 1)
        let pending = try XCTUnwrap(pendingInputs.first)
        XCTAssertFalse(pending.isSweptTombstone)
        XCTAssertNil(pending.winnerMinedHeight)
    }
}
