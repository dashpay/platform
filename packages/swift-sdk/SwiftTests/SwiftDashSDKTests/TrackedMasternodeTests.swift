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
}
