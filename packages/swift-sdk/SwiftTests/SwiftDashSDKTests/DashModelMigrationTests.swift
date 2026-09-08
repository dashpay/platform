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
        // V1 registers the FROZEN component (see `DashSchemaFrozenModels`),
        // so a row written into a V1 container is that type — inserting the
        // live one would materialise as the frozen entity and then fail its
        // cast on read.
        v1Container?.mainContext.insert(DashSchemaV1.PersistentKeyword(
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

        // V2 registers the same frozen copy, so the read side is frozen too.
        let keywords = try migrated.mainContext.fetch(
            FetchDescriptor<DashSchemaV1.PersistentKeyword>())
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

    /// The stage this change adds: a V3 store must migrate to V4 and read
    /// back with the sweep columns backfilled to their "nothing swept yet"
    /// values. V3 registers the frozen component, so the row goes in as the
    /// frozen type and comes out as the live one — which is the whole point
    /// of the freeze: the same entity, one property wider. A pending-input
    /// row rides along so the tombstone index V4 adds is exercised by the
    /// migration too.
    @MainActor
    func testV3StoreMigratesToV4AndBackfillsTheSweepColumns() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let walletId = Data(repeating: 0x5A, count: 32)

        let v3Schema = Schema(versionedSchema: DashSchemaV3.self)
        let v3Configuration = ModelConfiguration(
            "DashSweepMigrationTest",
            schema: v3Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v3Container: ModelContainer? = try ModelContainer(
            for: v3Schema,
            configurations: [v3Configuration])
        v3Container?.mainContext.insert(DashSchemaV1.PersistentWallet(
            walletId: walletId,
            network: .testnet))
        v3Container?.mainContext.insert(DashSchemaV1.PersistentPendingInput(
            outpoint: Data(repeating: 0x11, count: 36),
            inputIndex: 0,
            spendingTxid: Data(repeating: 0x22, count: 32),
            spendingTransaction: nil,
            walletId: walletId))
        // A transaction with one spent output: the two other widened
        // models, so the migration is exercised on every column V4 adds.
        let v3Funding = DashSchemaV1.PersistentTransaction(
            txid: Data(repeating: 0x33, count: 32),
            transactionData: Data([0x03, 0x00]),
            context: 2,
            blockHeight: 100)
        v3Container?.mainContext.insert(v3Funding)
        let v3Coin = DashSchemaV1.PersistentTxo(
            transaction: v3Funding,
            vout: 0,
            amount: 1_000,
            address: "yV3Coin",
            height: 100)
        v3Coin.walletId = walletId
        v3Coin.isSpent = true
        v3Container?.mainContext.insert(v3Coin)
        try v3Container?.mainContext.save()
        v3Container = nil

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

        let wallets = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentWallet>())
        XCTAssertEqual(wallets.count, 1, "the V3 row must survive the migration")
        XCTAssertNil(
            wallets.first?.lastAppliedChainLockHeight,
            "a wallet migrated from V3 has no chainlock boundary yet, so no "
                + "tombstone it later takes can be collected on a fabricated one")
        let pending = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentPendingInput>())
        XCTAssertEqual(pending.count, 1, "the V3 pending row must survive the migration")
        XCTAssertEqual(pending.first?.isSweptTombstone, false, "backfilled as an ordinary claim")
        XCTAssertNil(pending.first?.winnerMinedHeight, "and unstamped")
        let coins = try migrated.mainContext.fetch(FetchDescriptor<PersistentTxo>())
        XCTAssertEqual(coins.count, 1, "the V3 TXO row must survive the migration")
        XCTAssertEqual(coins.first?.isSpent, true, "its spent flag is carried as stored")
        XCTAssertNil(
            coins.first?.supersededByTxid,
            "a coin migrated from V3 was never held by a sweep — the stamp backfills to nil, "
                + "so the release and re-delivery rules see an ordinary spent coin")
        let transactions = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentTransaction>())
        XCTAssertEqual(transactions.map(\.context), [2], "the V3 transaction row survives unchanged")
    }

    /// The whole chain from the oldest registered version, on the models this
    /// change actually widens: a V1 store carrying a wallet, a transaction
    /// and a coin must arrive at V4 with every row intact and the V4 columns
    /// at their backfill values. V1 and V2 register the frozen component,
    /// so the rows go in as frozen types and come out live — the property
    /// the freeze exists to guarantee, pinned here where it matters most.
    @MainActor
    func testV1StoreWithWalletTransactionAndCoinMigratesToV4() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let walletId = Data(repeating: 0x1A, count: 32)
        let txid = Data(repeating: 0x1B, count: 32)

        let v1Schema = Schema(versionedSchema: DashSchemaV1.self)
        let v1Configuration = ModelConfiguration(
            "DashChainMigrationTest",
            schema: v1Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v1Container: ModelContainer? = try ModelContainer(
            for: v1Schema,
            configurations: [v1Configuration])
        v1Container?.mainContext.insert(DashSchemaV1.PersistentWallet(
            walletId: walletId,
            network: .testnet))
        let v1Funding = DashSchemaV1.PersistentTransaction(
            txid: txid,
            transactionData: Data([0x03, 0x00]),
            context: 3,
            blockHeight: 50,
            netAmount: 2_000)
        v1Container?.mainContext.insert(v1Funding)
        let v1Coin = DashSchemaV1.PersistentTxo(
            transaction: v1Funding,
            vout: 1,
            amount: 2_000,
            address: "yV1Coin",
            height: 50)
        v1Coin.walletId = walletId
        v1Container?.mainContext.insert(v1Coin)
        try v1Container?.mainContext.save()
        v1Container = nil

        let v4Schema = Schema(versionedSchema: DashSchemaV4.self)
        let v4Configuration = ModelConfiguration(
            "DashChainMigrationTest",
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
        XCTAssertNil(wallets.first?.lastAppliedChainLockHeight)
        let transactions = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentTransaction>())
        XCTAssertEqual(transactions.map(\.txid), [txid])
        XCTAssertEqual(transactions.first?.context, 3)
        XCTAssertEqual(transactions.first?.netAmount, 2_000)
        let coins = try migrated.mainContext.fetch(FetchDescriptor<PersistentTxo>())
        XCTAssertEqual(coins.count, 1)
        XCTAssertEqual(coins.first?.vout, 1)
        XCTAssertEqual(coins.first?.amount, 2_000)
        XCTAssertEqual(coins.first?.walletId, walletId)
        XCTAssertEqual(coins.first?.isSpent, false)
        XCTAssertNil(coins.first?.supersededByTxid)
        XCTAssertEqual(
            coins.first?.transaction?.txid, txid,
            "the coin's relationship to its funding transaction survives three stages")
    }

    /// What makes the V3 -> V4 stage lightweight: the two versions name the
    /// same entity set, and V4 only widens three of them. Also pins that
    /// `PersistentTransaction` is NOT one of the three — a swept row is
    /// deleted outright, so the transaction entity carries no sweep marker,
    /// and one that came back would silently change V4's checksum.
    func testV3AndV4NameTheSameEntitySet() throws {
        let v3 = Schema(versionedSchema: DashSchemaV3.self)
        let v4 = Schema(versionedSchema: DashSchemaV4.self)
        XCTAssertEqual(
            v3.entities.map(\.name).sorted(),
            v4.entities.map(\.name).sorted())

        let transaction = try XCTUnwrap(v4.entities.first { $0.name == "PersistentTransaction" })
        let frozenTransaction = try XCTUnwrap(v3.entities.first { $0.name == "PersistentTransaction" })
        XCTAssertEqual(
            transaction.attributesByName.keys.sorted(),
            frozenTransaction.attributesByName.keys.sorted(),
            "V4 adds no column to PersistentTransaction")
        let txo = try XCTUnwrap(v4.entities.first { $0.name == "PersistentTxo" })
        XCTAssertNotNil(txo.attributesByName["supersededByTxid"])
        let pendingInput = try XCTUnwrap(v4.entities.first { $0.name == "PersistentPendingInput" })
        XCTAssertNotNil(pendingInput.attributesByName["isSweptTombstone"])
        XCTAssertNotNil(pendingInput.attributesByName["winnerMinedHeight"])
        let wallet = try XCTUnwrap(v4.entities.first { $0.name == "PersistentWallet" })
        XCTAssertNotNil(wallet.attributesByName["lastAppliedChainLockHeight"])

        // And V3's frozen copies do not carry them.
        let frozenTxo = try XCTUnwrap(v3.entities.first { $0.name == "PersistentTxo" })
        XCTAssertNil(frozenTxo.attributesByName["supersededByTxid"])
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
            Schema(versionedSchema: DashSchemaV3.self),
            Schema(versionedSchema: DashSchemaV4.self),
            Schema(versionedSchema: DashSchemaV5.self)
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

extension DashModelMigrationTests {
    @MainActor
    func testV4StoreMigratesToV5PreservingSweepStateAndProfiles() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("sweep-profile.store")
        let oldSchema = Schema(versionedSchema: DashSchemaV4.self)
        let oldConfig = ModelConfiguration(schema: oldSchema, url: storeURL, cloudKitDatabase: .none)
        var oldContainer: ModelContainer? = try ModelContainer(for: oldSchema, configurations: [oldConfig])
        let identityId = Data(repeating: 0x21, count: 32)
        let walletId = Data(repeating: 0x31, count: 32)
        let winnerTxid = Data(repeating: 0x41, count: 32)
        do {
            let context = oldContainer!.mainContext
            let identity = PersistentIdentity(identityId: identityId, isLocal: true, network: .testnet)
            context.insert(identity)
            context.insert(PersistentDashpayProfile(identity: identity, displayName: "Preserved profile"))
            let wallet = PersistentWallet(walletId: walletId, network: .testnet)
            wallet.lastAppliedChainLockHeight = 4321
            context.insert(wallet)
            let pending = PersistentPendingInput(
                outpoint: Data(repeating: 0x11, count: 36), inputIndex: 0,
                spendingTxid: winnerTxid, spendingTransaction: nil, walletId: walletId)
            pending.isSweptTombstone = true
            pending.winnerMinedHeight = 1234
            context.insert(pending)
            let funding = PersistentTransaction(txid: Data(repeating: 0x51, count: 32),
                transactionData: Data([0x03, 0x00]), context: 2, blockHeight: 100)
            context.insert(funding)
            let coin = PersistentTxo(transaction: funding, vout: 0, amount: 1000, address: "yV4Coin", height: 100)
            coin.walletId = walletId
            coin.isSpent = true
            coin.supersededByTxid = winnerTxid
            context.insert(coin)
            try context.save()
        }
        oldContainer = nil

        let schema = Schema(versionedSchema: DashSchemaV5.self)
        let config = ModelConfiguration(schema: schema, url: storeURL, cloudKitDatabase: .none)
        var container: ModelContainer? = try ModelContainer(
            for: schema, migrationPlan: DashMigrationPlan.self, configurations: [config])
        do {
            let context = container!.mainContext
            let profiles = try context.fetch(FetchDescriptor<PersistentDashpayProfile>())
            XCTAssertEqual(profiles.map(\.displayName), ["Preserved profile"])
            XCTAssertEqual(profiles.first?.identity.identityId, identityId)
            XCTAssertNil(profiles.first?.shieldedAddress)
            let wallets = try context.fetch(FetchDescriptor<PersistentWallet>())
            XCTAssertEqual(wallets.map(\.lastAppliedChainLockHeight), [4321])
            let pending = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentPendingInput>()).first)
            XCTAssertTrue(pending.isSweptTombstone)
            XCTAssertEqual(pending.winnerMinedHeight, 1234)
            XCTAssertEqual(pending.spendingTxid, winnerTxid)
            let coin = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentTxo>()).first)
            XCTAssertTrue(coin.isSpent)
            XCTAssertEqual(coin.supersededByTxid, winnerTxid)
            XCTAssertEqual(coin.transaction?.blockHeight, 100)
            try PersistentDashpayPaymentAddresses.replace(in: context,
                networkRaw: Network.testnet.rawValue, ownerIdentityId: identityId, profileIdentityId: identityId,
                core: nil, platform: nil, shielded: Data(repeating: 0x45, count: 43))
            try context.save()
        }
        container = nil
        let reopened = try ModelContainer(for: schema, migrationPlan: DashMigrationPlan.self, configurations: [config])
        let profile = try XCTUnwrap(reopened.mainContext.fetch(FetchDescriptor<PersistentDashpayProfile>()).first)
        XCTAssertEqual(profile.shieldedAddress, Data(repeating: 0x45, count: 43))
    }

    @MainActor
    func testV3ProfileStoreMigratesToPaymentAddressesWithoutLosingProfile() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("profile.store")
        let oldSchema = Schema(versionedSchema: DashSchemaV3.self)
        let oldConfig = ModelConfiguration(schema: oldSchema, url: storeURL, cloudKitDatabase: .none)
        var oldContainer: ModelContainer? = try ModelContainer(for: oldSchema, configurations: [oldConfig])
        let identityId = Data(repeating: 0x21, count: 32)
        do {
            let identity = DashSchemaV1.PersistentIdentity(identityId: identityId, isLocal: true, network: .testnet)
            oldContainer!.mainContext.insert(identity)
            oldContainer!.mainContext.insert(DashSchemaV1.PersistentDashpayProfile(
                identity: identity, displayName: "Preserved profile"))
            try oldContainer!.mainContext.save()
        }
        oldContainer = nil
        let schema = Schema(versionedSchema: DashSchemaV5.self)
        let config = ModelConfiguration(schema: schema, url: storeURL, cloudKitDatabase: .none)
        let container = try ModelContainer(for: schema, migrationPlan: DashMigrationPlan.self, configurations: [config])
        let profiles = try container.mainContext.fetch(FetchDescriptor<PersistentDashpayProfile>())
        XCTAssertEqual(profiles.count, 1)
        XCTAssertEqual(profiles[0].displayName, "Preserved profile")
        XCTAssertNil(profiles[0].shieldedAddress)
        try PersistentDashpayPaymentAddresses.replace(in: container.mainContext,
            networkRaw: Network.testnet.rawValue, ownerIdentityId: identityId, profileIdentityId: identityId,
            core: nil, platform: nil, shielded: Data(repeating: 0x45, count: 43))
        try container.mainContext.save()
        XCTAssertEqual(profiles[0].identity.identityId, identityId)
        XCTAssertEqual(profiles[0].shieldedAddress, Data(repeating: 0x45, count: 43))
    }
}
