import CoreData
import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Migration coverage from two directions: source stores built in this
/// process from each registered version, and stores that OLDER BUILDS
/// actually wrote.
///
/// The fixture stores under `Fixtures/SchemaStores/` were written by a build
/// of the persistence sources as of commit 5f58417079 — the last state
/// before V4, the state the frozen copies under `FrozenSchemas/` are
/// generated from — through that build's own `DashSchemaV1` /
/// `DashSchemaV2` / `DashSchemaV3`. They pin the frozen copies as that
/// pre-V4 build defined them, not what the original V1 release wrote (see
/// the `DashSchemaV1` doc for why those stores are expected to fail open
/// and be rebuilt). Each carries a wallet,
/// an account, a core address, two transactions, a TXO linked to both, a
/// pending input, an identity, a keyword, an asset lock and (from V2) a
/// tracked masternode — enough to exercise every relationship in the wallet
/// graph.
///
/// A source store written in this process by `Schema(versionedSchema:)`
/// cannot replace them: SwiftData binds an entity name to the first Swift
/// type that claims it, so such a store carries whatever shape the process
/// had already bound, and a frozen version whose entities had silently
/// rebound to the live shape would round-trip itself and pass vacuously.
/// Only a store from a build that knew nothing of the live shape can tell.
final class DashModelMigrationTests: XCTestCase {
    /// SwiftData binds an entity name to the first Swift type that claims it
    /// in the process, so whether the live schema is built before or after
    /// a frozen version is decided once per process, not per test. Build it
    /// first here, as `DashModelContainer.create` does, so every test below
    /// runs in the order the app would.
    override class func setUp() {
        super.setUp()
        _ = DashModelContainer.schema
    }

    private struct Fixture {
        let name: String
        let version: any VersionedSchema.Type
        let hasTrackedMasternode: Bool
        let assetLockRecipientIsExternal: Bool?
    }

    private static let fixtures: [Fixture] = [
        Fixture(
            name: "dash-v1", version: DashSchemaV1.self,
            hasTrackedMasternode: false, assetLockRecipientIsExternal: nil),
        Fixture(
            name: "dash-v2", version: DashSchemaV2.self,
            hasTrackedMasternode: true, assetLockRecipientIsExternal: nil),
        Fixture(
            name: "dash-v3", version: DashSchemaV3.self,
            hasTrackedMasternode: true, assetLockRecipientIsExternal: true),
    ]

    private static let fixtureWalletId = Data(repeating: 0x31, count: 32)
    private static let fixtureSpendTxid = Data(repeating: 0x32, count: 32)
    private static let fixtureFundingTxid = Data(repeating: 0x34, count: 32)
    private static let fixtureIdentityId = Data(repeating: 0x35, count: 32)

    /// A private, writable copy of a fixture store.
    private func copyFixture(_ fixture: Fixture) throws -> (URL, URL) {
        let source = try XCTUnwrap(
            Bundle.module.url(
                forResource: fixture.name, withExtension: "store",
                subdirectory: "Fixtures/SchemaStores"),
            "missing fixture \(fixture.name).store")
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        let copy = directory.appendingPathComponent("\(fixture.name).store")
        try FileManager.default.copyItem(at: source, to: copy)
        return (directory, copy)
    }

    private static func storeHashes(at url: URL) throws -> (String, [String: Data]) {
        let metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(
            type: .sqlite, at: url)
        let checksum = try XCTUnwrap(
            metadata["NSStoreModelVersionChecksumKey"] as? String,
            "store has no model checksum")
        let hashes = try XCTUnwrap(
            metadata["NSStoreModelVersionHashes"] as? [String: Data],
            "store has no entity hashes")
        return (checksum, hashes)
    }

    /// Every fixture opens through `DashModelContainer.create`'s exact
    /// order — live schema built first, then the migration plan — and its
    /// rows come back through the live types with the relationships intact
    /// and the new columns at their migration defaults.
    @MainActor
    func testStoresWrittenByOlderBuildsMigrateThroughTheContainerFactory() throws {
        for fixture in Self.fixtures {
            let (directory, url) = try copyFixture(fixture)
            defer { try? FileManager.default.removeItem(at: directory) }

            let container: ModelContainer
            do {
                container = try DashModelContainer.create(url: url)
            } catch {
                XCTFail("\(fixture.name): migration failed to open: \(error)")
                continue
            }
            let context = container.mainContext

            let wallets = try context.fetch(FetchDescriptor<PersistentWallet>())
            XCTAssertEqual(wallets.map(\.walletId), [Self.fixtureWalletId], fixture.name)
            let wallet = try XCTUnwrap(wallets.first)
            XCTAssertEqual(wallet.name, "fixture wallet", fixture.name)
            XCTAssertEqual(wallet.syncedHeight, 120, fixture.name)
            XCTAssertNil(wallet.lastAppliedChainLockHeight, fixture.name)
            XCTAssertEqual(wallet.accounts.count, 1, fixture.name)
            XCTAssertEqual(
                wallet.identities.map(\.identityId), [Self.fixtureIdentityId], fixture.name)

            let accounts = try context.fetch(FetchDescriptor<PersistentAccount>())
            let account = try XCTUnwrap(accounts.first, fixture.name)
            XCTAssertEqual(accounts.count, 1, fixture.name)
            XCTAssertEqual(account.wallet.walletId, Self.fixtureWalletId, fixture.name)
            XCTAssertEqual(account.coreAddresses.map(\.address), ["yFixtureAddress"], fixture.name)
            XCTAssertEqual(
                Set(account.involvedTransactions.map(\.txid)),
                [Self.fixtureSpendTxid, Self.fixtureFundingTxid], fixture.name)

            let transactions = try context.fetch(FetchDescriptor<PersistentTransaction>())
            XCTAssertEqual(transactions.count, 2, fixture.name)
            let funding = try XCTUnwrap(
                transactions.first { $0.txid == Self.fixtureFundingTxid }, fixture.name)
            let spend = try XCTUnwrap(
                transactions.first { $0.txid == Self.fixtureSpendTxid }, fixture.name)
            XCTAssertEqual(funding.outputs.count, 1, fixture.name)
            XCTAssertEqual(spend.inputs.count, 1, fixture.name)
            XCTAssertEqual(spend.pendingInputs.count, 1, fixture.name)

            let txos = try context.fetch(FetchDescriptor<PersistentTxo>())
            XCTAssertEqual(txos.count, 1, fixture.name)
            let txo = try XCTUnwrap(txos.first)
            XCTAssertEqual(txo.amount, 1_000, fixture.name)
            XCTAssertEqual(txo.transaction?.txid, Self.fixtureFundingTxid, fixture.name)
            XCTAssertEqual(txo.spendingTransaction?.txid, Self.fixtureSpendTxid, fixture.name)
            XCTAssertEqual(txo.coreAddress?.address, "yFixtureAddress", fixture.name)
            XCTAssertEqual(txo.account?.accountIndex, 0, fixture.name)
            XCTAssertNil(txo.supersededByTxid, fixture.name)

            let pendingInputs = try context.fetch(FetchDescriptor<PersistentPendingInput>())
            XCTAssertEqual(pendingInputs.count, 1, fixture.name)
            let pending = try XCTUnwrap(pendingInputs.first)
            XCTAssertEqual(pending.spendingTxid, Self.fixtureSpendTxid, fixture.name)
            XCTAssertEqual(pending.spendingTransaction?.txid, Self.fixtureSpendTxid, fixture.name)
            XCTAssertFalse(pending.isSweptTombstone, fixture.name)
            XCTAssertNil(pending.winnerMinedHeight, fixture.name)

            let identities = try context.fetch(FetchDescriptor<PersistentIdentity>())
            XCTAssertEqual(identities.map(\.identityId), [Self.fixtureIdentityId], fixture.name)
            XCTAssertEqual(identities.first?.balance, 5, fixture.name)
            XCTAssertEqual(identities.first?.wallet?.walletId, Self.fixtureWalletId, fixture.name)

            let keywords = try context.fetch(FetchDescriptor<PersistentKeyword>())
            XCTAssertEqual(keywords.map(\.keyword), ["preserved"], fixture.name)

            let locks = try context.fetch(FetchDescriptor<PersistentAssetLock>())
            XCTAssertEqual(locks.count, 1, fixture.name)
            XCTAssertEqual(locks.first?.amountDuffs, 100_000, fixture.name)
            XCTAssertEqual(
                locks.first?.recipientIsExternal, fixture.assetLockRecipientIsExternal,
                fixture.name)

            let tracked = try context.fetchCount(FetchDescriptor<PersistentTrackedMasternode>())
            XCTAssertEqual(tracked, fixture.hasTrackedMasternode ? 1 : 0, fixture.name)

            // The migrated store is writable through the new columns.
            wallet.lastAppliedChainLockHeight = 130
            txo.supersededByTxid = Data(repeating: 0x36, count: 32)
            try context.save()
            XCTAssertEqual(
                try context.fetch(FetchDescriptor<PersistentWallet>()).first?
                    .lastAppliedChainLockHeight,
                130, fixture.name)
        }
    }

    /// Each frozen version, built after the live schema (the order
    /// `DashModelContainer.create` uses, established process-wide in
    /// `setUp`), still hashes every entity exactly as the build that shipped
    /// it did. A partial freeze cannot give this: a frozen wallet reached
    /// from a live `PersistentAccount.wallet` is rebound to the live
    /// wallet's shape the moment the live schema is built first, and the
    /// released checksum moves with it.
    func testFrozenVersionsBuiltAfterTheLiveSchemaHashLikeTheStoresTheyShipped() throws {
        for fixture in Self.fixtures {
            let (directory, url) = try copyFixture(fixture)
            defer { try? FileManager.default.removeItem(at: directory) }
            let (shippedChecksum, shippedHashes) = try Self.storeHashes(at: url)

            let scratch = directory.appendingPathComponent("scratch.store")
            let schema = Schema(versionedSchema: fixture.version)
            let configuration = ModelConfiguration(
                "DashSchemaScratch", schema: schema, url: scratch, allowsSave: true,
                cloudKitDatabase: .none)
            _ = try ModelContainer(for: schema, configurations: [configuration])
            let (builtChecksum, builtHashes) = try Self.storeHashes(at: scratch)

            let drifted = shippedHashes.keys.filter { shippedHashes[$0] != builtHashes[$0] }
                .sorted()
            XCTAssertEqual(
                drifted, [],
                "\(fixture.name): entities whose frozen shape no longer matches the shipped store")
            XCTAssertEqual(
                Set(builtHashes.keys), Set(shippedHashes.keys),
                "\(fixture.name): entity membership differs from the shipped store")
            XCTAssertEqual(builtChecksum, shippedChecksum, "\(fixture.name): checksum")
        }
    }

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
        // V1 registers the frozen graph (see `FrozenSchemas/`),
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

        // V2 registers the frozen `PersistentTrackedMasternode`, so the row
        // written into a V2 container is that type too.
        migrated.mainContext.insert(DashSchemaV2.PersistentTrackedMasternode(
            networkRaw: Network.testnet.rawValue,
            proTxHash: Data(repeating: 7, count: 32),
            label: "new in V2",
            addedAt: 1,
            snapshotJSON: "{}"))
        try migrated.mainContext.save()
        XCTAssertEqual(
            try migrated.mainContext.fetchCount(
                FetchDescriptor<DashSchemaV2.PersistentTrackedMasternode>()),
            1)
    }

    /// The stage this change adds: a V3 store must migrate to V4 and read
    /// back with the sweep columns backfilled to their "nothing swept yet"
    /// values. V3 registers the frozen graph, so the row goes in as the
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
    /// at their backfill values. V1 and V2 register the frozen graph,
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
            Schema(versionedSchema: DashSchemaV4.self)
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

        // V3 registers the frozen `DashSchemaV3.PersistentAssetLock`, the
        // shape that gained the column, so the read side is that type.
        let locks = try migrated.mainContext.fetch(
            FetchDescriptor<DashSchemaV3.PersistentAssetLock>())
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
            try migrated.mainContext.fetch(FetchDescriptor<DashSchemaV3.PersistentAssetLock>())
                .first?.recipientIsExternal,
            true)
    }
}
