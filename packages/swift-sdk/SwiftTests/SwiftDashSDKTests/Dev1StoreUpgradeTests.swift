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
/// back to inferred lightweight migration. This test drives that production
/// entry point rather than rebuilding a look-alike container, so the fallback
/// cannot regress unnoticed.
@MainActor
final class Dev1StoreUpgradeTests: XCTestCase {
    func testDev1StoreOpensThroughProductionFactoryAndPreservesCoreRows() throws {
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
        let sqlite = try (compressed as NSData).decompressed(using: .zlib) as Data
        XCTAssertEqual(sqlite.count, 647_168)

        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        addTeardownBlock {
            try? FileManager.default.removeItem(at: directory)
        }

        func configuration(named name: String) throws -> ModelConfiguration {
            let storeURL = directory.appendingPathComponent(name)
            try sqlite.write(to: storeURL, options: .atomic)
            return ModelConfiguration(
                schema: DashModelContainer.schema,
                url: storeURL,
                allowsSave: true,
                cloudKitDatabase: .none
            )
        }

        // The staged plan on its own is what crashes hosts today. Assert it on
        // its own copy of the fixture — a failed open must not be what the
        // production path below is then handed — so this test keeps naming the
        // cause once the remaining models are frozen and it starts succeeding.
        XCTAssertThrowsError(
            try ModelContainer(
                for: DashModelContainer.schema,
                migrationPlan: DashMigrationPlan.self,
                configurations: [try configuration(named: "StagedOnly.sqlite")]
            )
        )

        let container = try DashModelContainer.open(
            try configuration(named: "DashModel.sqlite")
        )
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
}
