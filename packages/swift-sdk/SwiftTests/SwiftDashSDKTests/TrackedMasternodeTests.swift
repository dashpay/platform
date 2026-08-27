import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Tracked-masternode marshalling: the Rust-computed capability gating and
/// the SwiftData row the persistence callbacks write.
final class TrackedMasternodeTests: XCTestCase {
    func testCapabilitiesFollowHeldRoles() {
        XCTAssertEqual(
            MasternodeCapabilities(holding: []),
            MasternodeCapabilities(holding: []))
        let none = MasternodeCapabilities(holding: [])
        XCTAssertFalse(none.canWithdraw)
        XCTAssertFalse(none.canVote)

        let owner = MasternodeCapabilities(holding: [.owner])
        XCTAssertTrue(owner.canWithdraw)
        XCTAssertFalse(owner.canVote)

        let payout = MasternodeCapabilities(holding: [.ownerPayout])
        XCTAssertTrue(payout.canWithdraw, "the payout-address key also withdraws")

        let voting = MasternodeCapabilities(holding: [.voting])
        XCTAssertTrue(voting.canVote)
        XCTAssertFalse(voting.canWithdraw)

        let op = MasternodeCapabilities(holding: [.operator, .platformNode])
        XCTAssertTrue(op.canUpdateService)
        XCTAssertTrue(op.identifiesPlatformNode)
        XCTAssertFalse(op.canWithdraw)
    }

    @MainActor
    func testPersistentTrackedMasternodeUniquenessIsPerNetwork() throws {
        let container = try ModelContainer(
            for: PersistentTrackedMasternode.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true))
        let context = container.mainContext
        let hash = Data(repeating: 7, count: 32)
        context.insert(PersistentTrackedMasternode(
            networkRaw: Network.mainnet.rawValue, proTxHash: hash,
            label: "main", addedAt: 1, snapshotJSON: "{}"))
        context.insert(PersistentTrackedMasternode(
            networkRaw: Network.testnet.rawValue, proTxHash: hash,
            label: "test", addedAt: 2, snapshotJSON: "{}"))
        try context.save()
        let rows = try context.fetch(FetchDescriptor<PersistentTrackedMasternode>())
        XCTAssertEqual(rows.count, 2, "the same proTxHash may be tracked on both networks")
        XCTAssertEqual(Set(rows.compactMap(\.network)), [.mainnet, .testnet])
    }

    @MainActor
    func testTrackedWritesCommitOutsideAndSurviveChangesetRollback() throws {
        let container = try ModelContainer(
            for: PersistentTrackedMasternode.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true))
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        let walletId = Data(repeating: 1, count: 32)
        let hash = Data(repeating: 9, count: 32)
        let row = PlatformWalletPersistenceHandler.TrackedMasternodeRow(
            proTxHash: hash,
            label: "survives",
            addedAt: 42,
            snapshotJSON: #"{"v":1}"#)

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistTrackedMasternodes(
            networkRaw: Network.testnet.rawValue,
            rows: [row]))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))

        var verificationContext = ModelContext(container)
        var persisted = try verificationContext.fetch(
            FetchDescriptor<PersistentTrackedMasternode>())
        XCTAssertEqual(persisted.count, 1)
        XCTAssertEqual(persisted.first?.proTxHash, hash)
        XCTAssertEqual(persisted.first?.label, "survives")

        // Whole-set removal has the same independent durability guarantee.
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistTrackedMasternodes(
            networkRaw: Network.testnet.rawValue,
            rows: []))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))

        verificationContext = ModelContext(container)
        persisted = try verificationContext.fetch(
            FetchDescriptor<PersistentTrackedMasternode>())
        XCTAssertTrue(persisted.isEmpty)
    }

}
