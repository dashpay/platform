import CoreData
import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Pins that a real v4.2.0-dev.1 store opens through the path the SDK actually
/// ships, and keeps its Core wallet records.
///
/// The staged `DashMigrationPlan` alone cannot open it: staged migration
/// matches a store by each registered `VersionedSchema`'s checksum, and only
/// `PersistentAssetLock` is frozen so far (`DashSchemaFrozenModels.swift`), so
/// the drifted `PersistentDocumentType` / `PersistentIndex` shapes leave a
/// dev.1 store matching no registered version — Cocoa error 134504. Hosts turn
/// that throw into a launch crash, which is why `DashModelContainer.open` falls
/// back to inferred lightweight migration for exactly that error. These tests
/// drive that production entry point rather than rebuilding a look-alike
/// container, so the fallback — and its limits — cannot regress unnoticed.
@MainActor
final class Dev1StoreUpgradeTests: XCTestCase {
    private var directory: URL!
    private var fixtureSQLite: Data!

    override func setUp() async throws {
        let resourceURL = try XCTUnwrap(
            Bundle.module.url(
                forResource: "DashModel-v4.2.0-dev.1.sqlite",
                withExtension: "zlib",
                subdirectory: "Fixtures"
            )
        )
        let compressed = try Data(contentsOf: resourceURL)
        // This resource is produced with Foundation's `.zlib` compressor.
        // A Python zlib-wrapped stream is not accepted by NSData on iOS.
        fixtureSQLite = try (compressed as NSData).decompressed(using: .zlib) as Data
        XCTAssertEqual(fixtureSQLite.count, 647_168)

        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        // Every test starts from an empty logger and installs its own sink
        // before touching a store, so the log it reads holds only its own
        // lines and nothing buffered by an earlier suite can replay into it.
        SDKLogger.resetForTesting()
        XCTAssertTrue(SDKLogger.installFileSink(at: directory, includeDebug: false))
    }

    override func tearDown() async throws {
        try? FileManager.default.removeItem(at: directory)
    }

    /// A fresh copy of the dev.1 fixture under `name`, as a store configuration.
    private func dev1Configuration(named name: String) throws -> ModelConfiguration {
        let storeURL = directory.appendingPathComponent(name)
        try fixtureSQLite.write(to: storeURL, options: .atomic)
        return configuration(at: storeURL)
    }

    private func configuration(at storeURL: URL) -> ModelConfiguration {
        ModelConfiguration(
            schema: DashModelContainer.schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none
        )
    }

    private func logLines(event: String) throws -> [String] {
        SDKLogger.flush()
        let log = try String(
            contentsOf: directory.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        return log.split(separator: "\n").map(String.init).filter {
            $0.contains("event=\(event) ")
        }
    }

    private func assertDev1Rows(in container: ModelContainer) throws {
        let context = ModelContext(container)
        let wallets = try context.fetch(FetchDescriptor<PersistentWallet>())
        let accounts = try context.fetch(FetchDescriptor<PersistentAccount>())

        XCTAssertEqual(wallets.count, 1)
        XCTAssertEqual(accounts.count, 1)
        XCTAssertEqual(wallets[0].walletId, Data(repeating: 0xA1, count: 32))
        XCTAssertEqual(wallets[0].birthHeight, 2_400_000)
        XCTAssertEqual(wallets[0].syncedHeight, 2_500_000)
        XCTAssertEqual(accounts[0].accountType, 0)
        XCTAssertEqual(accounts[0].accountIndex, 0)
        XCTAssertEqual(
            accounts[0].accountExtendedPubKeyBytes,
            Data(repeating: 0x02, count: 78)
        )
        XCTAssertEqual(accounts[0].wallet.walletId, wallets[0].walletId)
    }

    func testDev1StoreOpensThroughProductionFactoryAndPreservesCoreRows() throws {
        // The staged plan on its own is what crashes hosts today. Assert it on
        // its own copy of the fixture — a failed open must not be what the
        // production path below is then handed — and assert the precondition
        // the fallback keys on: this store matches no registered version. The
        // pair keeps naming the cause until the remaining models are frozen,
        // at which point the match flips to `true` and the staged attempt
        // starts succeeding.
        let stagedOnly = try dev1Configuration(named: "StagedOnly.sqlite")
        XCTAssertEqual(DashModelContainer.classifyStore(at: stagedOnly.url), .driftedRegisteredVersion)
        XCTAssertThrowsError(
            try ModelContainer(
                for: DashModelContainer.schema,
                migrationPlan: DashMigrationPlan.self,
                configurations: [stagedOnly]
            )
        )

        let container = try DashModelContainer.open(
            try dev1Configuration(named: "DashModel.sqlite")
        )
        try assertDev1Rows(in: container)

        let staged = try logLines(event: "core_store_staged_migration_failed")
        XCTAssertEqual(staged.count, 1, staged.joined(separator: "\n"))
        let result = try XCTUnwrap(try logLines(event: "core_store_open_result").last)
        XCTAssertTrue(result.contains(#"migration_path="inferred_fallback""#), result)
        XCTAssertTrue(result.contains(#"result="success""#), result)
    }

    /// The fallback's "self-heal" claim: once inferred migration has opened a
    /// dev.1 store, it carries the current schema's checksum, so the very next
    /// open must succeed through the staged plan with no fallback at all.
    func testFallbackMigratedStoreReopensThroughStagedPlan() throws {
        let storeURL = directory.appendingPathComponent("DashModel.sqlite")
        try fixtureSQLite.write(to: storeURL, options: .atomic)

        XCTAssertEqual(DashModelContainer.classifyStore(at: storeURL), .driftedRegisteredVersion)
        try autoreleasepool {
            let first = try DashModelContainer.open(configuration(at: storeURL))
            try assertDev1Rows(in: first)
        }
        XCTAssertEqual(try logLines(event: "core_store_staged_migration_failed").count, 1)
        // Inferred migration rewrote the store under the current schema, so
        // it now matches a registered version and the fallback is never
        // needed again.
        XCTAssertEqual(DashModelContainer.classifyStore(at: storeURL), .matchesRegisteredVersion)

        let second = try DashModelContainer.open(configuration(at: storeURL))
        try assertDev1Rows(in: second)

        // Still exactly one staged failure — the reopen did not need the
        // fallback — and the latest result names the staged path.
        XCTAssertEqual(try logLines(event: "core_store_staged_migration_failed").count, 1)
        let results = try logLines(event: "core_store_open_result")
        XCTAssertEqual(results.count, 2, results.joined(separator: "\n"))
        let reopen = try XCTUnwrap(results.last)
        XCTAssertTrue(reopen.contains(#"migration_path="staged""#), reopen)
        XCTAssertTrue(reopen.contains(#"result="success""#), reopen)
        XCTAssertTrue(reopen.contains("store_existed_before_open=true"), reopen)
    }

    /// Any failure other than "unknown model version" must surface untouched:
    /// once a custom `MigrationStage` exists, a failure inside it reopened
    /// without the plan would stamp the current checksum on a store that never
    /// ran that stage, so it could never run later.
    func testNonMigrationOpenFailureIsRethrownWithoutFallback() throws {
        let storeURL = directory.appendingPathComponent("DashModel.sqlite")
        try Data(repeating: 0x5A, count: 4096).write(to: storeURL, options: .atomic)

        // Unreadable metadata is not a version question — and not the typed
        // "newer build" error either, which would send the user to update.
        XCTAssertEqual(DashModelContainer.classifyStore(at: storeURL), .unreadable)
        XCTAssertThrowsError(try DashModelContainer.open(configuration(at: storeURL))) { error in
            XCTAssertNil(error as? DashModelContainerError, "corrupt store must not read as a newer build")
        }

        XCTAssertTrue(try logLines(event: "core_store_staged_migration_failed").isEmpty)
        let result = try XCTUnwrap(try logLines(event: "core_store_open_result").last)
        XCTAssertTrue(result.contains(#"migration_path="staged""#), result)
        XCTAssertTrue(result.contains(#"result="failure""#), result)
        XCTAssertTrue(result.contains(#"store_verdict="unreadable""#), result)
        // The failed store's path is redacted from the error message.
        XCTAssertFalse(result.contains(storeURL.path), result)
    }

    /// A store written by a newer build — here, one with an entity this SDK
    /// does not have — fails the staged open like a drifted store does, but
    /// must NOT be handed to inferred migration: that would open it and drop
    /// the unknown entity's table without a word. The pre-fallback crash was
    /// the safe outcome for a downgrade, and it must stay one.
    func testStoreFromANewerSchemaIsRefusedWithoutFallback() throws {
        let storeURL = directory.appendingPathComponent("DashModel.sqlite")
        try autoreleasepool {
            let newer = Schema(DashModelContainer.modelTypes + [FutureOnlyModel.self])
            let configuration = ModelConfiguration(
                schema: newer,
                url: storeURL,
                allowsSave: true,
                cloudKitDatabase: .none
            )
            let container = try ModelContainer(for: newer, configurations: [configuration])
            let context = ModelContext(container)
            context.insert(FutureOnlyModel(marker: 7))
            try context.save()
        }

        guard case .newerThanRegistered(let reason) = DashModelContainer.classifyStore(at: storeURL)
        else {
            return XCTFail("a store with an unknown entity must classify as newer")
        }
        XCTAssertTrue(reason.contains("FutureOnlyModel"), reason)

        XCTAssertThrowsError(try DashModelContainer.open(configuration(at: storeURL))) { error in
            guard case DashModelContainerError.storeFromNewerBuild(let reason) = error else {
                return XCTFail("a newer store must surface as the typed error, got \(error)")
            }
            XCTAssertTrue(reason.contains("FutureOnlyModel"), reason)
        }
        XCTAssertTrue(try logLines(event: "core_store_staged_migration_failed").isEmpty)
        let result = try XCTUnwrap(try logLines(event: "core_store_open_result").last)
        XCTAssertTrue(result.contains(#"migration_path="staged""#), result)
        XCTAssertTrue(result.contains(#"result="failure""#), result)
        XCTAssertTrue(result.contains("store_verdict=\"newer_than_registered:"), result)
        // And the store is untouched: still newer, still refused.
        guard case .newerThanRegistered = DashModelContainer.classifyStore(at: storeURL) else {
            return XCTFail("a refused open must not rewrite the store")
        }
    }
    /// The attribute-only disagreement: a store whose
    /// `PersistentWalletManagerMetadata` has one attribute the live model does
    /// not, keeping V3's version identifier. Its identifier is registered and
    /// every entity name is known, so only the per-entity comparison can tell
    /// it from the pinned drift — and must, because inferred migration would
    /// drop the attribute's values without a word.
    ///
    /// It is refused as `unplaceable`, NOT as a newer build: the same shape
    /// arises the day an attribute is added to any unfrozen live model, where
    /// every existing store is the OLDER one. So SwiftData's own error comes
    /// through and the host never offers a reset off the back of it.
    func testStoreWithAnAttributeOnlyNewerEntityIsRefusedWithoutFallback() throws {
        let storeURL = directory.appendingPathComponent("DashModel.sqlite")
        try autoreleasepool {
            let newer = Schema(versionedSchema: AttributeOnlyNewerSchema.self)
            let configuration = ModelConfiguration(
                schema: newer,
                url: storeURL,
                allowsSave: true,
                cloudKitDatabase: .none
            )
            let container = try ModelContainer(for: newer, configurations: [configuration])
            let context = ModelContext(container)
            context.insert(AttributeOnlyNewerSchema.PersistentWalletManagerMetadata(
                networkRaw: 1,
                futureAttribute: 42
            ))
            try context.save()
        }

        XCTAssertEqual(
            DashModelContainer.classifyStore(at: storeURL),
            .unplaceable(reason: "unexpected_entity_drift=PersistentWalletManagerMetadata")
        )
        XCTAssertThrowsError(try DashModelContainer.open(configuration(at: storeURL))) { error in
            XCTAssertNil(
                error as? DashModelContainerError,
                "a disagreement with no direction must not claim a newer build: \(error)"
            )
        }
        XCTAssertTrue(try logLines(event: "core_store_staged_migration_failed").isEmpty)
        let result = try XCTUnwrap(try logLines(event: "core_store_open_result").last)
        XCTAssertTrue(result.contains(#"result="failure""#), result)
        XCTAssertTrue(
            result.contains("store_verdict=\"unplaceable:unexpected_entity_drift=PersistentWalletManagerMetadata\""),
            result
        )
    }

    /// `knownDriftedEntityHashes` must be exactly what the fixture shows, no
    /// wider and byte for byte: every entity it names disagrees with V1's
    /// model for this store, none disagrees that it does not name, and each
    /// listed hash is the one the store carries. When a shape gets frozen,
    /// this is the test that says to shrink the table.
    func testKnownDriftedEntityHashesArePinnedToTheFixture() throws {
        let storeURL = try dev1Configuration(named: "Pin.sqlite").url
        let metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(
            ofType: NSSQLiteStoreType, at: storeURL, options: nil
        )
        let storeHashes = try XCTUnwrap(metadata[NSStoreModelVersionHashesKey] as? [String: Data])
        let identifiers = try XCTUnwrap(metadata[NSStoreModelVersionIdentifiersKey] as? [String])
        XCTAssertEqual(identifiers, [DashSchemaV1.versionIdentifier.description])

        let v1 = try XCTUnwrap(NSManagedObjectModel.makeManagedObjectModel(for: DashSchemaV1.models))
        let disagreeing = Set(storeHashes.compactMap { name, hash in
            v1.entityVersionHashesByName[name] == hash ? nil : name
        })
        let known = DashModelContainer.knownDriftedEntityHashes
        XCTAssertEqual(
            disagreeing, Set(known.keys),
            "fixture drifts on \(disagreeing.sorted()); the table must name exactly those"
        )
        for (name, hash) in known {
            XCTAssertEqual(storeHashes[name], hash, "\(name): the pinned hash must be the store's")
        }
    }

    /// The decision on plain values: which stores the fallback may answer.
    func testStoreSchemaVerdictOnPlainValues() {
        let a = Data([1]), b = Data([2])
        let current: Set<String> = ["PersistentWallet", "PersistentDocumentType", "PersistentIndex"]
        let v1 = DashModelContainer.RegisteredVersionHashes(
            identifier: "1.0.0",
            entityHashes: ["PersistentWallet": a, "PersistentDocumentType": a, "PersistentIndex": a]
        )
        func verdict(
            _ store: [String: Data],
            identifiers: [String] = ["1.0.0"],
            matches: Bool = false
        ) -> DashModelContainer.StoreSchemaVerdict {
            DashModelContainer.storeSchemaVerdict(
                matchesRegisteredVersion: matches,
                storeEntityHashes: store,
                storeVersionIdentifiers: identifiers,
                registered: [v1],
                currentEntities: current
            )
        }

        // A compatible store is never inspected further.
        XCTAssertEqual(verdict(["PersistentWallet": b], matches: true), .matchesRegisteredVersion)
        // Drift confined to the known entities WITH their known shapes may be
        // migrated; the same entities with any other shape may not.
        let knownDocumentType = DashModelContainer.knownDriftedEntityHashes["PersistentDocumentType"]!
        let knownIndex = DashModelContainer.knownDriftedEntityHashes["PersistentIndex"]!
        XCTAssertEqual(
            verdict(["PersistentWallet": a, "PersistentDocumentType": knownDocumentType, "PersistentIndex": knownIndex]),
            .driftedRegisteredVersion
        )
        XCTAssertEqual(
            verdict(["PersistentWallet": a, "PersistentDocumentType": knownDocumentType, "PersistentIndex": a]),
            .driftedRegisteredVersion,
            "one drifted entity with its known shape, the other untouched"
        )
        XCTAssertEqual(
            verdict(["PersistentWallet": a, "PersistentDocumentType": b, "PersistentIndex": knownIndex]),
            .unplaceable(reason: "unexpected_entity_drift=PersistentDocumentType"),
            "a known entity with an unknown shape is not the pinned drift — and not evidence of direction"
        )
        // The attribute-only downgrade: same names, kept identifier, but the
        // disagreement is on an entity that is not known to have drifted.
        XCTAssertEqual(
            verdict(["PersistentWallet": b, "PersistentDocumentType": a, "PersistentIndex": a]),
            .unplaceable(reason: "unexpected_entity_drift=PersistentWallet")
        )
        // Mixed: known drift plus one unexpected entity still refuses.
        XCTAssertEqual(
            verdict(["PersistentWallet": b, "PersistentDocumentType": knownDocumentType, "PersistentIndex": a]),
            .unplaceable(reason: "unexpected_entity_drift=PersistentWallet")
        )
        XCTAssertEqual(
            DashModelContainer.storeSchemaVerdict(
                matchesRegisteredVersion: false,
                storeEntityHashes: ["PersistentWallet": a],
                storeVersionIdentifiers: ["1.0.0"],
                registered: [],
                currentEntities: current
            ),
            .unplaceable(reason: "no_registered_models"),
            "no model built from any schema is a fact about this build, not the store"
        )
        XCTAssertEqual(
            verdict(["PersistentWallet": a], identifiers: ["9.0.0"]),
            .unplaceable(reason: "unregistered_version_identifier=9.0.0"),
            "an identifier we do not register may be pre-V1 or de-registered, not only future"
        )
        // Unplaceable, NOT a newer build: `open` must rethrow SwiftData's own
        // error for these rather than tell the user their wallet came from a
        // newer app and offer a reset.
        XCTAssertEqual(
            verdict(["PersistentWallet": a], identifiers: []),
            .unplaceable(reason: "no_version_identifier")
        )
        XCTAssertEqual(
            verdict([:]),
            .unplaceable(reason: "no_entity_hashes"),
            "no hashes means nothing was compared; drift may not be claimed"
        )
        XCTAssertEqual(
            verdict(["PersistentWallet": a]),
            .unplaceable(reason: "no_entity_disagreement"),
            "every hash the store carries agrees and it still is not compatible — "
                + "it differs by something these hashes do not describe, not by the pinned drift"
        )
        XCTAssertEqual(
            verdict(["PersistentWallet": a, "FutureOnlyModel": a]),
            .newerThanRegistered(reason: "unknown_entities=FutureOnlyModel")
        )
    }
}

/// An entity no registered SDK schema has — what a store written by a future
/// build looks like to this one.
@Model
final class FutureOnlyModel {
    var marker: Int

    init(marker: Int) {
        self.marker = marker
    }
}

/// V3 exactly as a newer build would write it: the same version identifier
/// and the same entity set, with one attribute added to a relationship-free
/// entity. Nested so the clone shares the live entity's name (SwiftData
/// derives it from the unqualified type name) without touching the live type.
enum AttributeOnlyNewerSchema: VersionedSchema {
    static var versionIdentifier: Schema.Version { DashSchemaV3.versionIdentifier }

    static var models: [any PersistentModel.Type] {
        DashModelContainer.modelTypes.filter {
            ObjectIdentifier($0) != ObjectIdentifier(SwiftDashSDK.PersistentWalletManagerMetadata.self)
        } + [PersistentWalletManagerMetadata.self]
    }

    @Model
    final class PersistentWalletManagerMetadata {
        @Attribute(.unique) var networkRaw: UInt32
        var combinedSyncHeight: UInt32
        var combinedSyncBlockHash: Data?
        var walletCount: Int
        var createdAt: Date
        var lastUpdated: Date
        /// The one thing this build has that the SDK's model does not.
        var futureAttribute: Int

        init(networkRaw: UInt32, futureAttribute: Int) {
            self.networkRaw = networkRaw
            self.combinedSyncHeight = 0
            self.combinedSyncBlockHash = nil
            self.walletCount = 0
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.futureAttribute = futureAttribute
        }
    }
}
