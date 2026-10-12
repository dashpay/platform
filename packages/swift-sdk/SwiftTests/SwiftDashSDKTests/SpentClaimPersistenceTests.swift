import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

@MainActor
final class SpentClaimPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 1, count: 32)

    private func claim(_ claimant: UInt8?) -> SpentClaimRestoreFFI {
        var entry = SpentClaimRestoreFFI()
        withUnsafeMutableBytes(of: &entry.txid) { $0.copyBytes(from: Data(repeating: 2, count: 32)) }
        entry.vout = 0x10203
        entry.has_claimant = claimant != nil
        if let claimant {
            withUnsafeMutableBytes(of: &entry.claimant) { $0.copyBytes(from: Data(repeating: claimant, count: 32)) }
        }
        return entry
    }

    private func persist(_ handler: PlatformWalletPersistenceHandler, claimant: UInt8?) -> Bool {
        var entry = claim(claimant)
        return withUnsafePointer(to: &entry) {
            handler.persistSpentClaims(walletId: walletId, claimed: $0, claimedCount: 1,
                                       released: nil, releasedCount: 0)
        }
    }

    func testShouldPersistOrderedClaimsAndReturnUnknownTombstoneOnLoad() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        let account = PersistentAccount(wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "Standard")
        account.accountExtendedPubKeyBytes = Data([1])
        context.insert(wallet)
        context.insert(account)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(persist(handler, claimant: 3))
        XCTAssertTrue(persist(handler, claimant: 4))
        var released = OutPointFFI()
        released.txid = claim(nil).txid
        released.vout = claim(nil).vout
        var overlapping = claim(5)
        XCTAssertTrue(withUnsafePointer(to: &overlapping) { claimPtr in
            withUnsafePointer(to: &released) {
                handler.persistSpentClaims(walletId: walletId, claimed: claimPtr, claimedCount: 1,
                                           released: $0, releasedCount: 1)
            }
        })
        _ = handler.endChangeset(walletId: walletId, success: true)
        XCTAssertTrue(try ModelContext(container).fetch(FetchDescriptor<PersistentSpentClaim>()).isEmpty)
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(persist(handler, claimant: nil))
        XCTAssertTrue(persist(handler, claimant: nil))
        _ = handler.endChangeset(walletId: walletId, success: true)
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentSpentClaim>())
        XCTAssertEqual(rows.count, 1)
        XCTAssertNil(rows.first?.claimant)
        let restarted = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let loaded = restarted.loadWalletList()
        defer { restarted.loadWalletListFree(entries: loaded.entries.map(UnsafeRawPointer.init)) }
        XCTAssertFalse(loaded.errored)
        XCTAssertEqual(loaded.count, 1)
        let entry = try XCTUnwrap(loaded.entries).pointee
        XCTAssertEqual(entry.spent_claims_count, 1)
        let restored = try XCTUnwrap(entry.spent_claims).pointee
        XCTAssertFalse(restored.has_claimant)
        XCTAssertEqual(restored.vout, 0x10203)
    }

    func testShouldRollbackClaimsAndRejectLegacyStoresWithoutCertifyingPartialDelta() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        wallet.spentClaimsComplete = false
        context.insert(wallet)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(persist(handler, claimant: 3))
        _ = handler.endChangeset(walletId: walletId, success: false)
        XCTAssertTrue(try ModelContext(container).fetch(FetchDescriptor<PersistentSpentClaim>()).isEmpty)
        XCTAssertTrue(handler.persistWalletMetadata(walletId: walletId, network: .testnet,
                                                   walletGroupId: Data(), birthHeight: 0))
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(persist(handler, claimant: 4))
        _ = handler.endChangeset(walletId: walletId, success: true)
        XCTAssertTrue(handler.loadWalletList().errored)
        XCTAssertFalse(try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentWallet>()).first).spentClaimsComplete)
    }
}
