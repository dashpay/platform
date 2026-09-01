import Foundation
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Pins the store-opening semantics used by DashWallet's
/// `SwiftDashSDKHost.buildModelContainer`: the current schema with inferred
/// lightweight migration and no staged migration plan.
///
/// `DashModelContainer.create` currently supplies `DashMigrationPlan` and
/// rejects the real v4.2.0-dev.1 checksum with Cocoa error 134504 because the
/// historical `PersistentDocumentType` and `PersistentIndex` shapes are not
/// registered as a frozen schema. This test deliberately does not exercise
/// that known-broken factory path; it verifies that the app-compatible path
/// opens the old store and preserves its Core wallet records.
@MainActor
final class Dev1StoreUpgradeTests: XCTestCase {
    func testDev1StoreOpensWithoutStagedPlanAndPreservesCoreRows() throws {
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

        let storeURL = directory.appendingPathComponent("DashModel.sqlite")
        try sqlite.write(to: storeURL, options: .atomic)

        let schema = DashModelContainer.schema
        let configuration = ModelConfiguration(
            schema: schema,
            url: storeURL,
            allowsSave: true,
            cloudKitDatabase: .none
        )
        let container = try ModelContainer(
            for: schema,
            configurations: [configuration]
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
