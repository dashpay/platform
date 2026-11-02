import XCTest
import SwiftData
@testable import SwiftDashSDK

final class IdentityBalanceMetadataSchemaTests: XCTestCase {
    func testFullWidthMetadataSurvivesStoreReopen() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try writeMetadata(url)
        let reopened = try DashModelContainer.create(url: url)
        let context = ModelContext(reopened)
        let rows = try context.fetch(FetchDescriptor<PersistentIdentityBalanceMetadata>())
        XCTAssertEqual(rows.count, 1)
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(UInt64(bitPattern: row.platformHeight), .max)
        XCTAssertEqual(row.coreHeight, .max)
        XCTAssertEqual(UInt64(bitPattern: row.timestampMillis), .max)
        XCTAssertEqual(row.walletId, Data(repeating: 42, count: 32))
        XCTAssertEqual(row.identityId, Data(repeating: 7, count: 32))
    }

    private func writeMetadata(_ url: URL) throws {
        let container = try DashModelContainer.create(url: url)
        let context = ModelContext(container)
        context.insert(PersistentIdentityBalanceMetadata(
            networkRaw: Network.testnet.rawValue, walletId: Data(repeating: 42, count: 32),
            identityId: Data(repeating: 7, count: 32), platformHeight: .max,
            coreHeight: .max, timestampMillis: .max))
        try context.save()
    }

    func testSameIdentityMetadataIsIsolatedByNetworkAndWallet() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        for (network, wallet, height): (UInt32, UInt8, UInt64) in [(0, 42, 1), (1, 42, 2), (1, 43, 3)] {
            context.insert(PersistentIdentityBalanceMetadata(networkRaw: network,
                walletId: Data(repeating: wallet, count: 32), identityId: Data(repeating: 7, count: 32),
                platformHeight: height, coreHeight: 0, timestampMillis: 0))
        }
        try context.save()
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentIdentityBalanceMetadata>())
        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(Set(rows.map { $0.platformHeight }), [1, 2, 3])
    }
}
