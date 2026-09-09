import CoreData
import Foundation
import SQLite3
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Migration coverage from two directions: source stores built in this
/// process from each registered version, and stores that OLDER BUILDS
/// actually wrote.
///
/// The fixture stores under `Fixtures/SchemaStores/` were written by the
/// builds that shipped each version: `dash-v1` to `dash-v3` by a build of
/// the persistence sources as of commit 5f58417079 — the last state before
/// V4, the state the frozen copies under `FrozenSchemas/` are generated
/// from — through that build's own `DashSchemaV1` / `DashSchemaV2` /
/// `DashSchemaV3`, and `dash-v4` by the build that registered V4, through
/// `DashModelContainer.create`. They pin the frozen copies as the pre-V4
/// build defined them, not what the original V1 release wrote (see the
/// `DashSchemaV1` doc for why those stores are expected to fail open and
/// be rebuilt). Each carries a wallet, an account, a core address, two
/// transactions, a TXO linked to both, a pending input, an identity, a
/// keyword, an asset lock and (from V2) a tracked masternode — enough to
/// exercise every relationship in the wallet graph; the rows are the ones
/// `testWriteTheLiveSchemaFixtureStore` writes.
///
/// The live version has a fixture too, so a change to a live model's
/// shape fails the hash test against that version's own store — the
/// failure a store in the field would otherwise report as Cocoa error
/// 134504. Changing the live shape before it ships is legitimate; doing
/// so means regenerating the live fixture on purpose, with
/// `testWriteTheLiveSchemaFixtureStore`, in the same change.
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
        Fixture(
            name: "dash-v4", version: DashSchemaV4.self,
            hasTrackedMasternode: true, assetLockRecipientIsExternal: true),
    ]

    /// Every schema version that has ever shipped, oldest first, as
    /// `major.minor.patch`. APPEND-ONLY: a version that shipped wrote stores
    /// that exist in the field, so it can never be removed from, reordered
    /// in, or replaced in the migration plan, and the plan is checked
    /// against this list rather than the other way round. Adding a version
    /// to the plan is shipping it: append it here in the same change and
    /// give it a fixture store in `fixtures`, written by that build with
    /// `testWriteTheLiveSchemaFixtureStore`. Every entry has a fixture,
    /// the live one included.
    private static let shippedVersions = ["1.0.0", "2.0.0", "3.0.0", "4.0.0"]

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
    ///
    /// This test is THE authority on whether a freeze is complete in every
    /// respect the entity hash covers: stored properties, their types,
    /// optionality and defaults, relationships and their inverses, and
    /// `#Unique` constraints. The generator's `--check`
    /// (`scripts/freeze_schema_models.py`) only proves the committed frozen
    /// files are the generator's byte-for-byte output; it does not, and
    /// must not try to, decide whether the `FREEZES` table covers every
    /// relationship target and stored value type. A static scan of Swift
    /// source cannot: it misses whatever syntax it does not understand, and
    /// it flags references SwiftData does not hash at all (a struct stored
    /// directly on a model is part of the entity hash; an array of structs
    /// nested inside it is not), so it fails silently in both directions.
    /// Only building the schema and reading the hash SwiftData computes,
    /// against a store a shipping build wrote, answers the question, and
    /// that is what this does: an omitted relationship target fails here as
    /// soon as the live target has changed shape, an omitted stored value
    /// type fails here on the change that would have broken the store, and
    /// a version registering the wrong entity set fails on membership.
    ///
    /// What the hash does NOT cover is `#Index`: Core Data leaves indexes
    /// out of entity version hashes, so an index that drifted in a frozen
    /// copy, or one a migration never created, passes here. That is what
    /// `testFixturesAndMigratedStoresCarryTheIndexesFreshStoresHave` is for.
    ///
    /// Its reach is exactly the fixtures: it guards a version only once a
    /// store written by a build that shipped that version is committed
    /// under `Fixtures/SchemaStores/` and listed in `fixtures`. So the first
    /// thing checked is that `fixtures` lists every version in
    /// `shippedVersions`, the live one included, once each: cutting a new
    /// schema version fails this test until its fixture is committed, and
    /// changing a live model's shape fails it against the live fixture
    /// until that fixture is deliberately rewritten. The expectation comes
    /// from the append-only list, not from the migration plan, so a plan
    /// that dropped a version cannot shrink it
    /// (`testShippedSchemaVersionsStayInTheMigrationPlan`).
    func testFrozenVersionsBuiltAfterTheLiveSchemaHashLikeTheStoresTheyShipped() throws {
        XCTAssertEqual(
            Self.fixtures.map { Self.describe($0.version.versionIdentifier) },
            Self.shippedVersions,
            "every shipped schema version, the live one included, needs a fixture store "
                + "written by the build that shipped it, listed once in `fixtures`; without "
                + "one its shape is unguarded")

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

    private static func describe(_ version: Schema.Version) -> String {
        "\(version.major).\(version.minor).\(version.patch)"
    }

    /// The migration plan must list exactly the versions that ever shipped,
    /// in the order they shipped, with nothing removed, reordered or
    /// replaced: a store written by any of them is still in the field and
    /// must be recognised. The expectation is the append-only
    /// `shippedVersions`, never the plan itself, so editing the plan cannot
    /// move the goalposts; a version can only enter the plan by being
    /// appended to that list in the same change.
    ///
    /// Versions are compared by identifier, not by enum: two enums both
    /// declaring `4.0.0` are indistinguishable here, so the identifiers in
    /// the list must be unique, and whether the enum behind an identifier
    /// still has the shape that shipped is decided by that version's
    /// fixture in the hash test.
    func testShippedSchemaVersionsStayInTheMigrationPlan() {
        XCTAssertEqual(
            Set(Self.shippedVersions).count, Self.shippedVersions.count,
            "a version identifier can ship once")
        XCTAssertEqual(
            DashMigrationPlan.schemas.map { Self.describe($0.versionIdentifier) },
            Self.shippedVersions,
            "the migration plan must list exactly the shipped versions, oldest first; a new "
                + "version is appended to `shippedVersions` in the same change, and nothing "
                + "that shipped is ever removed, reordered or replaced")
    }

    /// The schema the app opens stores with must be the last version of
    /// the migration plan. Both are declared separately, and SwiftData
    /// accepts a plan whose tail is newer than the schema it is asked to
    /// migrate to, so a version appended to the plan but not made the
    /// container's schema would leave the app writing the older shape while
    /// every migration test targets it.
    func testTheLiveSchemaIsTheMigrationPlansLastVersion() throws {
        let last = try XCTUnwrap(DashMigrationPlan.schemas.last)
        XCTAssertEqual(
            Self.describe(DashModelContainer.schema.version),
            Self.describe(last.versionIdentifier),
            "DashModelContainer.schema must be built from the migration plan's last version")
    }

    /// Writes the live version's fixture store — the rows every fixture
    /// carries, through `DashModelContainer.create`, so the file is what
    /// this build ships. Skipped unless `DASH_SCHEMA_FIXTURE_OUTPUT` names a
    /// directory to write into; run it on purpose when the live shape
    /// changes before shipping, or when a new version is cut:
    ///
    ///     DASH_SCHEMA_FIXTURE_OUTPUT=/some/dir swift test \
    ///       --filter DashModelMigrationTests/testWriteTheLiveSchemaFixtureStore
    ///
    /// then move `dash-vN.store` into `Fixtures/SchemaStores/`. A retired
    /// version's fixture is never rewritten: only the build that shipped it
    /// could write it.
    @MainActor
    func testWriteTheLiveSchemaFixtureStore() throws {
        guard let output = ProcessInfo.processInfo.environment["DASH_SCHEMA_FIXTURE_OUTPUT"]
        else {
            throw XCTSkip("set DASH_SCHEMA_FIXTURE_OUTPUT to write the live schema's fixture")
        }
        let version = try XCTUnwrap(DashMigrationPlan.schemas.last).versionIdentifier
        let url = URL(fileURLWithPath: output, isDirectory: true)
            .appendingPathComponent("dash-v\(version.major).store")
        XCTAssertFalse(
            FileManager.default.fileExists(atPath: url.path), "\(url.path) already exists")

        var container: ModelContainer? = try DashModelContainer.create(url: url)
        let context = try XCTUnwrap(container?.mainContext)
        let wallet = PersistentWallet(
            walletId: Self.fixtureWalletId, network: .testnet, name: "fixture wallet",
            syncedHeight: 120)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "standard")
        context.insert(account)
        let address = PersistentCoreAddress(
            address: "yFixtureAddress", poolTypeTag: 0, addressIndex: 0, derivationPath: "m/0")
        address.account = account
        context.insert(address)
        let funding = PersistentTransaction(
            txid: Self.fixtureFundingTxid, transactionData: Data([3, 0]), context: 2,
            blockHeight: 100)
        let spend = PersistentTransaction(
            txid: Self.fixtureSpendTxid, transactionData: Data([3, 0]), context: 2,
            blockHeight: 110)
        context.insert(funding)
        context.insert(spend)
        account.involvedTransactions = [funding, spend]
        let txo = PersistentTxo(
            transaction: funding, vout: 0, amount: 1_000, address: "yFixtureAddress",
            height: 100)
        txo.walletId = Self.fixtureWalletId
        txo.isSpent = true
        txo.spendingTransaction = spend
        txo.coreAddress = address
        txo.account = account
        context.insert(txo)
        context.insert(PersistentPendingInput(
            outpoint: Data(repeating: 0x11, count: 36), inputIndex: 0,
            spendingTxid: Self.fixtureSpendTxid, spendingTransaction: spend,
            walletId: Self.fixtureWalletId))
        let identity = PersistentIdentity(
            identityId: Self.fixtureIdentityId, balance: 5, network: .testnet)
        identity.wallet = wallet
        context.insert(identity)
        context.insert(PersistentKeyword(keyword: "preserved", contractId: "contract"))
        let lock = PersistentAssetLock(
            outPointHex: String(repeating: "ab", count: 32) + ":0",
            walletId: Self.fixtureWalletId, transactionBytes: Data([1, 2, 3]),
            fundingTypeRaw: 4, identityIndexRaw: -1, amountDuffs: 100_000, statusRaw: 4)
        lock.recipientIsExternal = true
        context.insert(lock)
        context.insert(PersistentTrackedMasternode(
            networkRaw: Network.testnet.rawValue, proTxHash: Data(repeating: 7, count: 32),
            label: "fixture", addedAt: 1, snapshotJSON: "{}"))
        try context.save()
        container = nil

        // A fixture has to be one self-contained file that opens read-only
        // from any directory, so the write-ahead log is folded back in and
        // the store left in rollback-journal mode, which also removes the
        // -wal and -shm sidecars.
        var database: OpaquePointer?
        XCTAssertEqual(
            sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READWRITE, nil), SQLITE_OK)
        XCTAssertEqual(sqlite3_exec(database, "PRAGMA journal_mode=DELETE", nil, nil, nil), SQLITE_OK)
        sqlite3_close(database)
        XCTAssertFalse(
            FileManager.default.fileExists(atPath: url.path + "-wal"),
            "the store still has a write-ahead log")
    }

    /// The SQLite indexes of a store, one line per index: table, name and
    /// the statement that created it. Auto-indexes SQLite makes for its
    /// own constraints have no statement and are listed as such.
    private static func indexes(at url: URL) throws -> Set<String> {
        var database: OpaquePointer?
        guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            sqlite3_close(database)
            struct StoreUnreadable: Error {}
            XCTFail("\(url.lastPathComponent): could not be opened as SQLite")
            throw StoreUnreadable()
        }
        defer { sqlite3_close(database) }
        var statement: OpaquePointer?
        let query = "SELECT tbl_name, name, sql FROM sqlite_master WHERE type = 'index'"
        XCTAssertEqual(sqlite3_prepare_v2(database, query, -1, &statement, nil), SQLITE_OK)
        defer { sqlite3_finalize(statement) }
        var rows = Set<String>()
        var step = sqlite3_step(statement)
        while step == SQLITE_ROW {
            let table = String(cString: sqlite3_column_text(statement, 0))
            let name = String(cString: sqlite3_column_text(statement, 1))
            let sql = sqlite3_column_text(statement, 2).map { String(cString: $0) } ?? "(auto)"
            rows.insert("\(table) \(name): \(sql)")
            step = sqlite3_step(statement)
        }
        // Anything but SQLITE_DONE (busy, corrupt, I/O, memory) means the
        // listing is partial, and a partial listing must not be compared.
        guard step == SQLITE_DONE else {
            struct PartialListing: Error {}
            XCTFail("\(url.lastPathComponent): index listing stopped with sqlite result \(step)")
            throw PartialListing()
        }
        return rows
    }

    /// The check the entity hash cannot give. Core Data leaves `#Index` out
    /// of version hashes, so the test above stays green when a frozen
    /// copy's index differs from what shipped, and a lightweight migration
    /// whose only change is an index can complete without creating it.
    /// Both show up in SQLite, so that is where they are checked:
    ///
    ///   - a fixture, as written, has exactly the indexes a store built
    ///     fresh from its frozen version has (the frozen `#Index` is what
    ///     shipped);
    ///   - after migrating through `DashModelContainer.create`, a fixture
    ///     has every index a store built fresh at the live version has (the
    ///     migration created what the live model declares).
    ///
    /// The second is a superset check, not equality: a migrated store can
    /// keep an index from an earlier layout, or one the live model no
    /// longer declares. That is not a compatibility problem, but it is not
    /// free either (the index takes storage and is maintained on every
    /// write), and this check does not look for it.
    @MainActor
    func testFixturesAndMigratedStoresCarryTheIndexesFreshStoresHave() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let freshLive = directory.appendingPathComponent("live.store")
        _ = try DashModelContainer.create(url: freshLive)
        let liveIndexes = try Self.indexes(at: freshLive)
        XCTAssertFalse(liveIndexes.isEmpty)

        for fixture in Self.fixtures {
            let (fixtureDirectory, url) = try copyFixture(fixture)
            defer { try? FileManager.default.removeItem(at: fixtureDirectory) }

            let freshAtVersion = fixtureDirectory.appendingPathComponent("fresh.store")
            let schema = Schema(versionedSchema: fixture.version)
            _ = try ModelContainer(
                for: schema,
                configurations: [
                    ModelConfiguration(
                        "DashSchemaIndexes", schema: schema, url: freshAtVersion,
                        allowsSave: true, cloudKitDatabase: .none)
                ])
            XCTAssertEqual(
                try Self.indexes(at: url).symmetricDifference(try Self.indexes(at: freshAtVersion)),
                [],
                "\(fixture.name): the frozen version's indexes differ from what shipped")

            _ = try DashModelContainer.create(url: url)
            XCTAssertEqual(
                liveIndexes.subtracting(try Self.indexes(at: url)), [],
                "\(fixture.name): indexes a fresh live store has that the migration did not create")
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
