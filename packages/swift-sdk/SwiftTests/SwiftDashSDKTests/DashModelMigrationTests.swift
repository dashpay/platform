import CoreData
import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Migration coverage against stores that OLDER BUILDS actually wrote.
///
/// The fixture stores under `Fixtures/SchemaStores/` were produced by the
/// SDK's own `DashSchemaV1` / `DashSchemaV2` / `DashSchemaV3` definitions at
/// commit 96a103375e (v4.2.0-dev.8), before the wallet transaction graph
/// gained its sweep columns. Each carries a wallet, an account, a core
/// address, two transactions, a TXO linked to both, a pending input, an
/// identity, a keyword, an asset lock and (from V2) a tracked masternode —
/// enough to exercise every relationship in the wallet graph.
///
/// A source store written IN this process by `Schema(versionedSchema:)`
/// would not do: SwiftData binds an entity name to the first Swift type
/// that claims it, so such a store carries whatever shape the process had
/// already bound, and a frozen version whose entities had silently rebound
/// to the live shape would round-trip itself and pass vacuously. Only a
/// store from a build that knew nothing of the live shape can tell.
final class DashModelMigrationTests: XCTestCase {
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

    private static let walletId = Data(repeating: 0x31, count: 32)
    private static let spendTxid = Data(repeating: 0x32, count: 32)
    private static let fundingTxid = Data(repeating: 0x34, count: 32)
    private static let identityId = Data(repeating: 0x35, count: 32)

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
            XCTAssertEqual(wallets.map(\.walletId), [Self.walletId], fixture.name)
            let wallet = try XCTUnwrap(wallets.first)
            XCTAssertEqual(wallet.name, "fixture wallet", fixture.name)
            XCTAssertEqual(wallet.syncedHeight, 120, fixture.name)
            XCTAssertNil(wallet.lastAppliedChainLockHeight, fixture.name)
            XCTAssertEqual(wallet.accounts.count, 1, fixture.name)
            XCTAssertEqual(wallet.identities.map(\.identityId), [Self.identityId], fixture.name)

            let accounts = try context.fetch(FetchDescriptor<PersistentAccount>())
            let account = try XCTUnwrap(accounts.first, fixture.name)
            XCTAssertEqual(accounts.count, 1, fixture.name)
            XCTAssertEqual(account.wallet.walletId, Self.walletId, fixture.name)
            XCTAssertEqual(account.coreAddresses.map(\.address), ["yFixtureAddress"], fixture.name)
            XCTAssertEqual(
                Set(account.involvedTransactions.map(\.txid)),
                [Self.spendTxid, Self.fundingTxid], fixture.name)

            let transactions = try context.fetch(FetchDescriptor<PersistentTransaction>())
            XCTAssertEqual(transactions.count, 2, fixture.name)
            for transaction in transactions {
                XCTAssertFalse(transaction.isGloballySwept, fixture.name)
            }
            let funding = try XCTUnwrap(
                transactions.first { $0.txid == Self.fundingTxid }, fixture.name)
            let spend = try XCTUnwrap(
                transactions.first { $0.txid == Self.spendTxid }, fixture.name)
            XCTAssertEqual(funding.outputs.count, 1, fixture.name)
            XCTAssertEqual(spend.inputs.count, 1, fixture.name)
            XCTAssertEqual(spend.pendingInputs.count, 1, fixture.name)

            let txos = try context.fetch(FetchDescriptor<PersistentTxo>())
            XCTAssertEqual(txos.count, 1, fixture.name)
            let txo = try XCTUnwrap(txos.first)
            XCTAssertEqual(txo.amount, 1_000, fixture.name)
            XCTAssertEqual(txo.transaction?.txid, Self.fundingTxid, fixture.name)
            XCTAssertEqual(txo.spendingTransaction?.txid, Self.spendTxid, fixture.name)
            XCTAssertEqual(txo.coreAddress?.address, "yFixtureAddress", fixture.name)
            XCTAssertEqual(txo.account?.accountIndex, 0, fixture.name)
            XCTAssertNil(txo.supersededByTxid, fixture.name)

            let pendingInputs = try context.fetch(FetchDescriptor<PersistentPendingInput>())
            XCTAssertEqual(pendingInputs.count, 1, fixture.name)
            let pending = try XCTUnwrap(pendingInputs.first)
            XCTAssertEqual(pending.spendingTxid, Self.spendTxid, fixture.name)
            XCTAssertEqual(pending.spendingTransaction?.txid, Self.spendTxid, fixture.name)
            XCTAssertFalse(pending.isSweptTombstone, fixture.name)
            XCTAssertNil(pending.winnerMinedHeight, fixture.name)

            let identities = try context.fetch(FetchDescriptor<PersistentIdentity>())
            XCTAssertEqual(identities.map(\.identityId), [Self.identityId], fixture.name)
            XCTAssertEqual(identities.first?.balance, 5, fixture.name)
            XCTAssertEqual(identities.first?.wallet?.walletId, Self.walletId, fixture.name)

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

    /// Each frozen version, built AFTER the live schema (the order
    /// `DashModelContainer.create` uses), still hashes every entity exactly
    /// as the build that shipped it did. This is the property the previous
    /// partial freeze lacked: a frozen wallet whose live `PersistentAccount`
    /// pointed back at the live wallet type rebound "PersistentWallet" to the
    /// live shape and moved V1–V3's checksums the moment the live schema
    /// was built.
    func testFrozenVersionsBuiltAfterTheLiveSchemaHashLikeTheStoresTheyShipped() throws {
        _ = DashModelContainer.schema

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

    /// Guards the frozen models' entity identity. SwiftData must derive each
    /// nested type's entity name from the unqualified type name so migration
    /// stages add columns to existing tables instead of dropping and creating
    /// unrelated entities.
    func testFrozenModelsKeepTheLiveEntityNames() throws {
        let live = Set(Schema(versionedSchema: DashSchemaV4.self).entities.map(\.name))
        for version in [DashSchemaV1.self, DashSchemaV2.self, DashSchemaV3.self]
            as [any VersionedSchema.Type]
        {
            let frozen = Set(Schema(versionedSchema: version).entities.map(\.name))
            XCTAssertTrue(
                frozen.isSubset(of: live),
                "\(version): frozen entities not known to the live schema: "
                    + "\(frozen.subtracting(live).sorted())")
        }

        // Successive versions alter shapes, not entity membership, except
        // for the one addition V2 makes.
        XCTAssertEqual(
            Set(Schema(versionedSchema: DashSchemaV2.self).entities.map(\.name))
                .subtracting(Schema(versionedSchema: DashSchemaV1.self).entities.map(\.name)),
            ["PersistentTrackedMasternode"])
        XCTAssertEqual(
            Schema(versionedSchema: DashSchemaV2.self).entities.map(\.name).sorted(),
            Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name).sorted())
        XCTAssertEqual(
            Schema(versionedSchema: DashSchemaV3.self).entities.map(\.name).sorted(),
            Schema(versionedSchema: DashSchemaV4.self).entities.map(\.name).sorted())
    }
}
