// Reproduction recipe, copied into a disposable SDK by historical_schema_fixture.py.
// This file and the historical model graph are not compiled into the shipping SDK.
import Foundation
import SQLite3
import SwiftData
import XCTest

@testable import SwiftDashSDK

@MainActor
final class DashHistoricalFixtureCaptureTests: XCTestCase {
    func testCaptureHistoricalSourceFixture() throws {
        // Match the original audit's model-registration order.
        _ = DashModelContainer.schema
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = directory.appendingPathComponent("historical.store")
        try autoreleasepool {
            // The historical iOS app used an unversioned Schema with no migration plan.
            let schema = Schema(DashSchemaSnapshotV1.models)
            let configuration = ModelConfiguration(
                schema: schema, url: store, allowsSave: true, cloudKitDatabase: .none)
            let container = try ModelContainer(for: schema, configurations: [configuration])
            let wallet = DashSchemaSnapshotV1.PersistentWallet(
                walletId: Data(repeating: 0x61, count: 32), network: .testnet,
                name: "historical audit wallet")
            let contractId = Data(repeating: 0x62, count: 32)
            let contract = DashSchemaSnapshotV1.PersistentDataContract(
                id: contractId, name: "historical audit contract",
                serializedContract: Data("{}".utf8), network: .testnet)
            let type = DashSchemaSnapshotV1.PersistentDocumentType(
                contractId: contractId, name: "audit",
                schemaJSON: Data("{}".utf8), propertiesJSON: Data("{}".utf8))
            let index = DashSchemaSnapshotV1.PersistentIndex(
                contractId: contractId, documentTypeName: "audit", name: "byName", properties: ["name"])
            container.mainContext.insert(wallet)
            container.mainContext.insert(contract)
            container.mainContext.insert(type)
            container.mainContext.insert(index)
            type.dataContract = contract
            index.documentType = type
            try container.mainContext.save()
        }
        try checkpoint(store)
        let captured = try DashSchemaFixtureSupport.describeStore(at: store, version: Schema.Version(1, 0, 0))
        let provenanceURL = try XCTUnwrap(Bundle.module.url(
            forResource: "manifest", withExtension: "json",
            subdirectory: "Fixtures/SchemaStores/legacy-fd8d8d13e5"))
        struct Provenance: Decodable { let schema: DashSchemaFixtureSupport.Description }
        let provenance = try JSONDecoder().decode(Provenance.self, from: Data(contentsOf: provenanceURL))
        XCTAssertEqual(captured, provenance.schema, "Recreated model hashes and indexes must match the committed fixture")
        let attachment = XCTAttachment(contentsOfFile: store)
        attachment.name = "legacy-fd8d8d13e5.store"
        attachment.lifetime = .keepAlways
        add(attachment)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .prettyPrinted]
        let metadata = XCTAttachment(data: try encoder.encode(captured), uniformTypeIdentifier: "public.json")
        metadata.name = "legacy-fd8d8d13e5.schema.json"
        metadata.lifetime = .keepAlways
        add(metadata)
    }

    private func checkpoint(_ url: URL) throws {
        var database: OpaquePointer?
        guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READWRITE, nil) == SQLITE_OK else {
            sqlite3_close(database)
            throw NSError(domain: "HistoricalFixtureSQLite", code: 1)
        }
        defer { sqlite3_close(database) }
        guard sqlite3_wal_checkpoint_v2(database, nil, SQLITE_CHECKPOINT_TRUNCATE, nil, nil) == SQLITE_OK,
              sqlite3_exec(database, "PRAGMA journal_mode=DELETE", nil, nil, nil) == SQLITE_OK,
              !FileManager.default.fileExists(atPath: url.path + "-wal"),
              !FileManager.default.fileExists(atPath: url.path + "-shm") else {
            throw NSError(domain: "HistoricalFixtureSQLite", code: 2)
        }
    }
}
