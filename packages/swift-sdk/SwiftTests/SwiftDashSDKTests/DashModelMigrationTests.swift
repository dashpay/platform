import CoreData
import Foundation
import SQLite3
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// The accepted V1 baseline remains byte-for-byte unchanged. Actual App Store
/// releases captured by the pipeline are additionally checked by
/// DashReleasedSchemaTests. A development version is not a released version.
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
        Fixture(name: "historical-v2", version: DashSchemaV2.self,
                hasTrackedMasternode: true, assetLockRecipientIsExternal: nil),
    ]

    private static var acceptedBaselineVersions: [Schema.Version] { [Schema.Version(1, 0, 0)] }

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

    @MainActor
    func testMigrationDiagnosticsReachExportFileWithoutVerboseLogging() throws {
        let fixture = try XCTUnwrap(Self.fixtures.first { $0.name == "historical-v2" })
        let (directory, url) = try copyFixture(fixture)
        defer { try? FileManager.default.removeItem(at: directory) }
        let session = directory.appendingPathComponent("logs", isDirectory: true)
        // This is the same file sink setting installed by the default low preset.
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))
        // The sink is process-wide: detach it before the directory disappears.
        defer { SDKLogger.removeFileSink() }
        let sourceChecksum = try Self.storeHashes(at: url).0

        try autoreleasepool { _ = try DashModelContainer.create(url: url) }
        try autoreleasepool { _ = try DashModelContainer.create(url: url) }
        SDKLogger.flush()
        let log = try String(contentsOf: session.appendingPathComponent("swift/run.log"), encoding: .utf8)
        XCTAssertTrue(log.contains("event=store_open_started"))
        XCTAssertTrue(log.contains("route=\"historical-v2-to-v3\""))
        XCTAssertTrue(log.contains("source_version=\"2.0.0\""))
        XCTAssertTrue(log.contains("source_checksum=\"\(sourceChecksum)\""))
        XCTAssertTrue(log.contains("target_version=\"3.0.0\""))
        XCTAssertTrue(log.contains("route=\"labelled-current-v3\""))
        XCTAssertEqual(log.components(separatedBy: "event=store_open_succeeded").count - 1, 2)
        XCTAssertFalse(log.contains("event=store_open_failed"))
        XCTAssertFalse(log.contains(directory.path))
        XCTAssertFalse(log.contains("fixture wallet"))

        let freshURL = directory.appendingPathComponent("fresh.store")
        try autoreleasepool { _ = try DashModelContainer.create(url: freshURL) }
        // A malformed store must produce a safe error diagnostic, without its contents.
        let invalidURL = directory.appendingPathComponent("invalid.store")
        try Data("private-invalid-store-content".utf8).write(to: invalidURL)
        XCTAssertThrowsError(try DashModelContainer.create(url: invalidURL))
        SDKLogger.flush()
        let updatedLog = try String(contentsOf: session.appendingPathComponent("swift/run.log"), encoding: .utf8)
        XCTAssertTrue(updatedLog.contains("route=\"new-store\""))
        let failure = try XCTUnwrap(updatedLog.split(separator: "\n").first { $0.contains("event=store_open_failed") })
        XCTAssertTrue(failure.contains("target_version=\"3.0.0\""))
        XCTAssertTrue(failure.contains("error_code="))
        XCTAssertFalse(updatedLog.contains(directory.path))
        XCTAssertFalse(updatedLog.contains("private-invalid-store-content"))
    }

    /// Opening a current store must not depend on the temporary-store probe
    /// that resolves a frozen schema's identity: the plan is the default
    /// either way. Labels whose route is undecidable without that probe
    /// (accepted V1 versus the bridge, historical V2 versus a beta layout)
    /// keep failing closed, leaving the store untouched.
    func testCurrentV3RouteNeverDependsOnTheSchemaIdentityProbe() throws {
        struct ProbeUnavailable: Error {}
        let failingProbe: (any VersionedSchema.Type) throws -> DashLegacySchemaBridge.Identity = { _ in
            throw ProbeUnavailable()
        }
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let current = directory.appendingPathComponent("current.store")
        try autoreleasepool { _ = try DashModelContainer.create(url: current) }
        XCTAssertEqual(try DashLegacySchemaBridge.identity(at: current).versions, ["3.0.0"])
        let plan = try DashModelContainer.migrationPlan(
            at: current, defaultPlan: DashMigrationPlan.self, identity: failingProbe)
        XCTAssertTrue(ObjectIdentifier(plan) == ObjectIdentifier(DashMigrationPlan.self))

        for fixture in Self.fixtures {
            let (fixtureDirectory, url) = try copyFixture(fixture)
            defer { try? FileManager.default.removeItem(at: fixtureDirectory) }
            let before = try DashLegacyStoreSQLite.rawDigest(url)
            XCTAssertThrowsError(try DashModelContainer.migrationPlan(
                at: url, defaultPlan: DashMigrationPlan.self, identity: failingProbe), fixture.name) { error in
                XCTAssertTrue(error is ProbeUnavailable, fixture.name)
            }
            XCTAssertEqual(try DashLegacyStoreSQLite.rawDigest(url), before, fixture.name)
        }
    }

    func testSchemaIdentityIsComputedOncePerProcessAndFailuresAreNotRemembered() throws {
        struct ProbeUnavailable: Error {}
        let cache = DashLegacySchemaBridge.SchemaIdentityCache()
        let identity = DashLegacySchemaBridge.Identity(
            versions: ["3.0.0"], checksum: "checksum", hashes: ["PersistentWallet": Data([1])])
        var computations = 0
        XCTAssertThrowsError(try cache.identity(for: DashSchemaV3.self) {
            computations += 1
            throw ProbeUnavailable()
        })
        XCTAssertEqual(try cache.identity(for: DashSchemaV3.self) { computations += 1; return identity }, identity)
        XCTAssertEqual(try cache.identity(for: DashSchemaV3.self) {
            computations += 1
            throw ProbeUnavailable()
        }, identity)
        XCTAssertEqual(computations, 2)
        XCTAssertThrowsError(try cache.identity(for: DashSchemaV2.self) {
            computations += 1
            throw ProbeUnavailable()
        })
        XCTAssertEqual(computations, 3)
        XCTAssertEqual(try DashLegacySchemaBridge.identity(for: DashSchemaV3.self),
                       try DashLegacySchemaBridge.identity(for: DashSchemaV3.self))
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

    /// The baseline fixture opens through `DashModelContainer.create`'s exact
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

    /// Keep the accepted V1 hashes stable even after the live graph has been
    /// built. A source-store round trip alone can miss accidentally live
    /// relationships and inline value types; compare the existing fixture.
    /// Captured App Store versions are covered by DashReleasedSchemaTests.
    func testFrozenVersionsBuiltAfterTheLiveSchemaHashLikeTheStoresTheyShipped() throws {
        XCTAssertEqual(
            Self.fixtures.map { $0.version.versionIdentifier },
            Self.acceptedBaselineVersions + [Schema.Version(2, 0, 0)],
            "the accepted baseline must retain its existing fixture")

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

    func testMigrationPlanContainsBaselinePublishedAndLiveVersionsInOrder() {
        let publishedVersions = DashReleasedSchemaRegistry.fixtures.map {
            $0.version.versionIdentifier
        }
        let expected = Set(
            [Schema.Version(2, 0, 0)] + publishedVersions + [DashModelContainer.schema.version]
        ).sorted()
        XCTAssertEqual(
            DashMigrationPlan.schemas.map { $0.versionIdentifier }, expected,
            "The primary plan retains historical V2 and published versions; accepted V1 has a separate non-destructive route")
    }

    func testMigrationStagesConnectAdjacentRegisteredSchemas() {
        let plans: [any SchemaMigrationPlan.Type] = [DashMigrationPlan.self, DashAcceptedV1MigrationPlan.self]
        for plan in plans {
            let schemas = plan.schemas
            let stages = plan.stages
            XCTAssertEqual(stages.count, schemas.count - 1)
            for (stage, adjacent) in zip(stages, zip(schemas, schemas.dropFirst())) {
                switch stage {
                case .lightweight(let from, let to), .custom(let from, let to, _, _):
                    XCTAssertEqual(ObjectIdentifier(from), ObjectIdentifier(adjacent.0))
                    XCTAssertEqual(ObjectIdentifier(to), ObjectIdentifier(adjacent.1))
                @unknown default:
                    XCTFail("Unsupported migration stage")
                }
            }
        }
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
        let registeredModels = last.models.map { ObjectIdentifier($0) }
        let liveModels = DashModelContainer.modelTypes.map { ObjectIdentifier($0) }
        XCTAssertEqual(Set(registeredModels).count, registeredModels.count)
        XCTAssertEqual(registeredModels.count, liveModels.count)
        XCTAssertEqual(
            Set(registeredModels), Set(liveModels),
            "The last version must register the live model types used by SDK callers")
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

    func testV3AddsTrackedMasternodesAndBalanceMetadataToTheBaselineEntitySet() {
        XCTAssertEqual(
            Set(Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name))
                .subtracting(Schema(versionedSchema: DashSchemaV1.self).entities.map(\.name)),
            ["PersistentTrackedMasternode", "PersistentIdentityBalanceMetadata"])
    }

    @MainActor
    func testV1BalanceGetsNoWatermarkUntilOneIsPersistedInLiveV3() throws {
        let (directory, url) = try copyFixture(Self.fixtures[0])
        defer { try? FileManager.default.removeItem(at: directory) }
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let context = container.mainContext
            XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentIdentity>()).first?.balance, 5)
            XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentIdentityBalanceMetadata>()), 0)
            context.insert(PersistentIdentityBalanceMetadata(
                networkRaw: Network.testnet.rawValue, walletId: Self.fixtureWalletId,
                identityId: Self.fixtureIdentityId, platformHeight: 42, coreHeight: 7,
                timestampMillis: 123))
            try context.save()
        }
        let reopened = try DashModelContainer.create(url: url)
        let metadata = try XCTUnwrap(reopened.mainContext.fetch(
            FetchDescriptor<PersistentIdentityBalanceMetadata>()).first)
        XCTAssertEqual(metadata.identityId, Self.fixtureIdentityId)
        XCTAssertEqual(metadata.walletId, Self.fixtureWalletId)
        XCTAssertEqual(metadata.platformHeight, 42)
        XCTAssertEqual(metadata.coreHeight, 7)
        XCTAssertEqual(metadata.timestampMillis, 123)
        XCTAssertEqual(try reopened.mainContext.fetch(FetchDescriptor<PersistentIdentity>()).first?.balance, 5)
    }

    /// The accepted V1 graph predates key limits. Its keys must arrive in
    /// live V3 unlimited, then accept limits through the public accessors.
    @MainActor
    func testV1StoreMigratesToV3AndBackfillsTheKeyLimitColumns() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let identityId = "FixtureIdentityBase58"

        let v1Schema = Schema(versionedSchema: DashSchemaV1.self)
        let v1Configuration = ModelConfiguration(
            "DashKeyLimitsMigrationTest",
            schema: v1Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v1Container: ModelContainer? = try ModelContainer(
            for: v1Schema,
            configurations: [v1Configuration])
        v1Container?.mainContext.insert(DashSchemaV1.PersistentPublicKey(
            keyId: 3,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x02, count: 33),
            identityId: identityId))
        try v1Container?.mainContext.save()
        v1Container = nil

        let v2Schema = Schema(versionedSchema: DashSchemaV3.self)
        let v2Configuration = ModelConfiguration(
            "DashKeyLimitsMigrationTest",
            schema: v2Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        let migrated = try ModelContainer(
            for: v2Schema,
            migrationPlan: DashAcceptedV1MigrationPlan.self,
            configurations: [v2Configuration])

        let keys = try migrated.mainContext.fetch(FetchDescriptor<PersistentPublicKey>())
        XCTAssertEqual(keys.count, 1, "the V1 key row must survive the migration")
        let key = try XCTUnwrap(keys.first)
        XCTAssertEqual(key.keyId, 3)
        XCTAssertEqual(key.identityId, identityId)
        XCTAssertEqual(key.publicKeyData, Data(repeating: 0x02, count: 33))
        XCTAssertNil(key.totalBudget, "a key migrated from V1 carries no budget")
        XCTAssertNil(key.expiresAt, "nor an expiry; together, that is a version 0 key")
        XCTAssertFalse(key.hasLimits)

        // And both new columns are writable on the migrated row, through the
        // unsigned accessors the rest of the SDK reads them with.
        key.totalBudgetCredits = 1_000
        key.expiresAtMillis = 1_800_000_000_000
        try migrated.mainContext.save()
        let reread = try XCTUnwrap(
            migrated.mainContext.fetch(FetchDescriptor<PersistentPublicKey>()).first)
        XCTAssertEqual(reread.totalBudgetCredits, 1_000)
        XCTAssertEqual(reread.expiresAtMillis, 1_800_000_000_000)
        XCTAssertTrue(reread.hasLimits)
    }

    /// Legacy contract bounds keep their inferred variant after V1 -> V3,
    /// and the new discriminator can then represent contract groups.
    @MainActor
    func testV1StoreMigratesToV3AndBackfillsTheContractBoundsKind() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let storeURL = directory.appendingPathComponent("dash.store")

        let contractId = Data(repeating: 0x7C, count: 32)

        let v1Schema = Schema(versionedSchema: DashSchemaV1.self)
        let v1Configuration = ModelConfiguration(
            "DashContractBoundsKindMigrationTest",
            schema: v1Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        var v1Container: ModelContainer? = try ModelContainer(
            for: v1Schema,
            configurations: [v1Configuration])
        v1Container?.mainContext.insert(DashSchemaV1.PersistentPublicKey(
            keyId: 3,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x02, count: 33),
            contractBounds: [contractId],
            contractBoundsDocumentTypeName: "contactRequest",
            identityId: "legacyDocTypeKey"))
        v1Container?.mainContext.insert(DashSchemaV1.PersistentPublicKey(
            keyId: 4,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: Data(repeating: 0x03, count: 33),
            contractBounds: [contractId],
            identityId: "legacyContractKey"))
        try v1Container?.mainContext.save()
        v1Container = nil

        let v2Schema = Schema(versionedSchema: DashSchemaV3.self)
        let v2Configuration = ModelConfiguration(
            "DashContractBoundsKindMigrationTest",
            schema: v2Schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none)
        let migrated = try ModelContainer(
            for: v2Schema,
            migrationPlan: DashAcceptedV1MigrationPlan.self,
            configurations: [v2Configuration])

        let rows = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentPublicKey>(sortBy: [SortDescriptor(\.keyId)]))
        XCTAssertEqual(rows.map(\.keyId), [3, 4], "both V1 rows survive the migration")
        XCTAssertEqual(
            rows.map(\.contractBoundsKind), [nil, nil],
            "a row written before the column backfills NULL")
        XCTAssertEqual(rows[0].contractBounds?.first, contractId)
        XCTAssertEqual(
            rows[0].effectiveContractBoundsKind, 2,
            "a legacy row with a doc-type name still infers SingleContractDocumentType")
        XCTAssertEqual(
            rows[1].effectiveContractBoundsKind, 1,
            "a legacy row with a bare id still infers SingleContract")

        // And the new column is writable on the migrated row.
        rows[1].contractBoundsKind = 3
        try migrated.mainContext.save()
        let reread = try migrated.mainContext.fetch(
            FetchDescriptor<PersistentPublicKey>(sortBy: [SortDescriptor(\.keyId)]))
        XCTAssertEqual(reread[1].effectiveContractBoundsKind, 3)
    }

    func testV3AddsKeyColumnsWithoutChangingTheFrozenBaseline() throws {
        let baseline = Schema(versionedSchema: DashSchemaV1.self)
        let live = Schema(versionedSchema: DashSchemaV3.self)
        let oldKey = try XCTUnwrap(baseline.entities.first { $0.name == "PersistentPublicKey" })
        let newKey = try XCTUnwrap(live.entities.first { $0.name == "PersistentPublicKey" })
        for column in ["totalBudget", "expiresAt", "contractBoundsKind"] {
            XCTAssertNil(oldKey.attributesByName[column])
            XCTAssertNotNil(newKey.attributesByName[column])
        }
    }

    /// Synthetic historical evidence generated from the reconstructed source;
    /// this test never accesses a device database or keychain.
    @MainActor
    func testCaptureHistoricalV2Fixture() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("historical-v2.store")
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV2.self)
            let container = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let context = container.mainContext
            let wallet = DashSchemaV2.PersistentWallet(
                walletId: Data(repeating: 0x31, count: 32), network: .testnet, name: "fixture wallet",
                syncedHeight: 120)
            context.insert(wallet)
            let account = DashSchemaV2.PersistentAccount(
                wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "standard")
            context.insert(account)
            let address = DashSchemaV2.PersistentCoreAddress(
                address: "yFixtureAddress", poolTypeTag: 0, addressIndex: 0, derivationPath: "m/0")
            address.account = account
            context.insert(address)
            let funding = DashSchemaV2.PersistentTransaction(
                txid: Data(repeating: 0x34, count: 32), transactionData: Data([3, 0]), context: 2,
                blockHeight: 100)
            let spend = DashSchemaV2.PersistentTransaction(
                txid: Data(repeating: 0x32, count: 32), transactionData: Data([3, 0]), context: 2,
                blockHeight: 110)
            context.insert(funding)
            context.insert(spend)
            account.involvedTransactions = [funding, spend]
            let txo = DashSchemaV2.PersistentTxo(
                transaction: funding, vout: 0, amount: 1_000, address: "yFixtureAddress",
                height: 100)
            txo.walletId = Data(repeating: 0x31, count: 32)
            txo.isSpent = true
            txo.spendingTransaction = spend
            txo.coreAddress = address
            txo.account = account
            context.insert(txo)
            context.insert(DashSchemaV2.PersistentPendingInput(
                outpoint: Data(repeating: 0x11, count: 36), inputIndex: 0,
                spendingTxid: Data(repeating: 0x32, count: 32), spendingTransaction: spend,
                walletId: Data(repeating: 0x31, count: 32)))
            let identity = DashSchemaV2.PersistentIdentity(
                identityId: Data(repeating: 0x35, count: 32), balance: 5, network: .testnet)
            identity.wallet = wallet
            context.insert(identity)
            let key = DashSchemaV2.PersistentPublicKey(
                keyId: 3, purpose: .authentication, securityLevel: .high,
                keyType: .ecdsaSecp256k1, publicKeyData: Data(repeating: 0x02, count: 33),
                identityId: identity.identityIdString)
            key.identity = identity
            context.insert(key)
            context.insert(DashSchemaV2.PersistentKeyword(keyword: "preserved", contractId: "contract"))
            let lock = DashSchemaV2.PersistentAssetLock(
                outPointHex: String(repeating: "ab", count: 32) + ":0",
                walletId: Data(repeating: 0x31, count: 32), transactionBytes: Data([1, 2, 3]),
                fundingTypeRaw: 4, identityIndexRaw: -1, amountDuffs: 100_000, statusRaw: 4)
            context.insert(lock)
            context.insert(DashSchemaV2.PersistentTrackedMasternode(
                networkRaw: Network.testnet.rawValue, proTxHash: Data(repeating: 7, count: 32),
                label: "fixture", addedAt: 1, snapshotJSON: "{}"))
            try context.save()
        }
        try DashLegacyStoreSQLite.checkpoint(url)
        let metadata = try DashSchemaFixtureSupport.describeStore(at: url, version: Schema.Version(2, 0, 0))
        XCTAssertEqual(metadata.entity_hashes.count, 35)
        XCTAssertEqual(metadata.model_checksum, "RrRj/iNbS9izgLQvNb2APed4iwaR7pftEE2+4tea7K8=")
        let attachment = XCTAttachment(contentsOfFile: url)
        attachment.name = "historical-v2.store"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor
    func testAcceptedV1PreservesAllThirteenFieldsMissingFromHistoricalV2() throws {
        let (directory, url) = try copyFixture(Self.fixtures[0])
        defer { try? FileManager.default.removeItem(at: directory) }
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV1.self)
            let container = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let context = container.mainContext
            let type = DashSchemaV1.PersistentDocumentType(
                contractId: Data([41]), name: "retained", schemaJSON: Data(), propertiesJSON: Data())
            type.indexOnly = true
            let index = DashSchemaV1.PersistentIndex(
                contractId: Data([41]), documentTypeName: "retained", name: "retained", properties: ["name"])
            index.documentType = type
            index.countable = "countableAllowingOffset"
            index.summable = "amount"
            index.averageable = "amount"
            index.terminal = "$ownerId"
            index.timeRangeJSON = Data("{\"on\":\"createdAt\"}".utf8)
            index.rangeCountable = true
            index.rangeSummable = true
            index.rangeAverageable = true
            index.rankedCountable = true
            index.rankedSummable = true
            index.rankedAverageable = true
            index.preallocated = true
            context.insert(type)
            context.insert(index)
            try context.save()
        }
        let before = directory.appendingPathComponent("before.store")
        try DashLegacyStoreSQLite.copy(from: url, to: before)
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let context = container.mainContext
            let type = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentDocumentType>()).first)
            let index = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentIndex>()).first)
            XCTAssertTrue(type.indexOnly)
            XCTAssertEqual(index.countable, "countableAllowingOffset")
            XCTAssertEqual(index.summable, "amount")
            XCTAssertEqual(index.averageable, "amount")
            XCTAssertEqual(index.terminal, "$ownerId")
            XCTAssertEqual(index.timeRangeJSON, Data("{\"on\":\"createdAt\"}".utf8))
            XCTAssertEqual([index.rangeCountable, index.rangeSummable, index.rangeAverageable,
                index.rankedCountable, index.rankedSummable, index.rankedAverageable, index.preallocated],
                Array(repeating: true, count: 7))
            XCTAssertEqual(index.documentType?.persistentModelID, type.persistentModelID)
            index.name = "writable"
            try context.save()
            index.name = "retained"
            try context.save()
        }
        try DashLegacyStoreSQLite.validatePreservation(from: before, to: url)
    }

    /// Local diagnostic only. The input never enters test resources, attachments
    /// or logs, and all migration/writes happen on a disposable copy.
    @MainActor
    func testPrivateHistoricalStoreMigrationWhenExplicitlyProvided() throws {
        let path = ProcessInfo.processInfo.environment["DASH_PRIVATE_MIGRATION_STORE"]
        try XCTSkipIf(path == nil, "No private migration diagnostic input was explicitly provided")
        let source = URL(fileURLWithPath: try XCTUnwrap(path))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true,
                                                attributes: [.posixPermissions: 0o700])
        defer { try? FileManager.default.removeItem(at: directory) }
        let before = directory.appendingPathComponent("before.store")
        let migrated = directory.appendingPathComponent("migrated.store")
        try DashLegacyStoreSQLite.copy(from: source, to: before)
        try DashLegacyStoreSQLite.copy(from: before, to: migrated)
        try autoreleasepool { _ = try DashModelContainer.create(url: migrated) }
        try DashLegacyStoreSQLite.validatePreservation(from: before, to: migrated)
        let walletId: Data = try autoreleasepool {
            let container = try DashModelContainer.create(url: migrated)
            let context = container.mainContext
            let wallet = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentWallet>()).first)
            wallet.name = "temporary migration write verification"
            try context.save()
            return wallet.walletId
        }
        try autoreleasepool {
            let container = try DashModelContainer.create(url: migrated)
            let wallets = try container.mainContext.fetch(FetchDescriptor<PersistentWallet>())
            XCTAssertTrue(wallets.contains {
                $0.walletId == walletId && $0.name == "temporary migration write verification"
            }, "A write to the diagnostic copy must survive reopening")
        }
    }

    @MainActor
    func testHistoricalV2AddsDefaultsAndPreservesDocumentRelationships() throws {
        let (directory, url) = try copyFixture(Self.fixtures[1])
        defer { try? FileManager.default.removeItem(at: directory) }
        try autoreleasepool {
            let schema = Schema(versionedSchema: DashSchemaV2.self)
            let container = try ModelContainer(for: schema, configurations: [
                ModelConfiguration(schema: schema, url: url, cloudKitDatabase: .none)
            ])
            let type = DashSchemaV2.PersistentDocumentType(
                contractId: Data([42]), name: "historical", schemaJSON: Data([123, 125]), propertiesJSON: Data([123, 125]))
            let index = DashSchemaV2.PersistentIndex(
                contractId: Data([42]), documentTypeName: "historical", name: "original", properties: ["name"])
            index.documentType = type
            container.mainContext.insert(type)
            container.mainContext.insert(index)
            try container.mainContext.save()
        }
        let before = directory.appendingPathComponent("before.store")
        try DashLegacyStoreSQLite.copy(from: url, to: before)
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let context = container.mainContext
            let type = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentDocumentType>()).first)
            let index = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentIndex>()).first)
            XCTAssertFalse(type.indexOnly)
            XCTAssertNil(index.countable)
            XCTAssertNil(index.summable)
            XCTAssertNil(index.averageable)
            XCTAssertNil(index.terminal)
            XCTAssertNil(index.timeRangeJSON)
            XCTAssertEqual([index.rangeCountable, index.rangeSummable, index.rangeAverageable,
                index.rankedCountable, index.rankedSummable, index.rankedAverageable, index.preallocated],
                Array(repeating: false, count: 7))
            XCTAssertEqual(index.documentType?.persistentModelID, type.persistentModelID)
            XCTAssertEqual(type.indices?.count, 1)
            XCTAssertEqual(index.properties, ["name"])
        }
        try DashLegacyStoreSQLite.validatePreservation(from: before, to: url)
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let index = try XCTUnwrap(container.mainContext.fetch(FetchDescriptor<PersistentIndex>()).first)
            index.preallocated = true
            try container.mainContext.save()
        }
        let reopened = try DashModelContainer.create(url: url)
        XCTAssertTrue(try XCTUnwrap(reopened.mainContext.fetch(FetchDescriptor<PersistentIndex>()).first).preallocated)
    }
}
