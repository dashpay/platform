import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for the Sent-invitations persistence bridge
/// (`PlatformWalletPersistenceHandler.persistInvitations`).
///
/// Pins the **T1 seam** (spec §4): the upsert stores `outPointHex` in the
/// `encodeOutPoint` display form, and a later removal derives the identical
/// string from the same 36-byte raw outpoint — so a create→removal round-trip
/// actually deletes the row. This seam is latent in v1 (reclaim, the only
/// removal emitter, isn't shipped yet), so it's exercised here before it ships:
/// if the upsert keyed `outPointHex` on anything other than the `encodeOutPoint`
/// form, the removal would silently match nothing and orphan the row.
@MainActor
final class InvitationPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 0x01, count: 32)
    // 36-byte outpoint: 32-byte txid ‖ 4-byte little-endian vout (= 0).
    private let rawOutPoint = Data(repeating: 0xAB, count: 32) + Data([0, 0, 0, 0])

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    private func snapshot(statusRaw: Int) -> PlatformWalletPersistenceHandler.InvitationEntrySnapshot {
        .init(
            outPointHex: PersistentAssetLock.encodeOutPoint(rawBytes: rawOutPoint),
            rawOutPoint: rawOutPoint,
            fundingIndexRaw: 3,
            amountDuffs: 50_000,
            expiryUnix: 1_800_000_000,
            createdAtSecs: 1_700_000_000,
            hasInviter: true,
            statusRaw: statusRaw
        )
    }

    private func fetchRows(_ container: ModelContainer) throws -> [PersistentInvitation] {
        try ModelContext(container).fetch(FetchDescriptor<PersistentInvitation>())
    }

    /// Create inserts one row (fields mapped, `walletId` set), a re-upsert of the
    /// same outpoint updates in place (no duplicate), and a removal keyed on the
    /// raw outpoint deletes it — the T1 seam. Each round is bracketed by
    /// `beginChangeset`/`endChangeset(success:)` exactly like the FFI store round.
    func testUpsertThenStatusChangeThenRemovalRoundTrips() throws {
        let (handler, container) = try makeHandler()
        let expectedHex = PersistentAssetLock.encodeOutPoint(rawBytes: rawOutPoint)

        // 1. Create.
        handler.beginChangeset(walletId: walletId)
        handler.persistInvitations(walletId: walletId, upserts: [snapshot(statusRaw: 0)], removed: [])
        _ = handler.endChangeset(walletId: walletId, success: true)

        var rows = try fetchRows(container)
        XCTAssertEqual(rows.count, 1, "one invitation row after create")
        XCTAssertEqual(rows.first?.outPointHex, expectedHex)
        XCTAssertEqual(rows.first?.walletId, walletId, "walletId set on insert (the @Query filter)")
        XCTAssertEqual(rows.first?.statusRaw, 0)
        XCTAssertEqual(rows.first?.amountDuffs, 50_000)
        XCTAssertEqual(rows.first?.fundingIndexRaw, 3)
        XCTAssertEqual(rows.first?.hasInviter, true)
        XCTAssertEqual(rows.first?.rawOutPoint, rawOutPoint, "raw outpoint stored for reclaim (no reverse-decode)")

        // 2. Status change → upsert in place, no duplicate row.
        handler.beginChangeset(walletId: walletId)
        handler.persistInvitations(walletId: walletId, upserts: [snapshot(statusRaw: 1)], removed: [])
        _ = handler.endChangeset(walletId: walletId, success: true)

        rows = try fetchRows(container)
        XCTAssertEqual(rows.count, 1, "re-upsert must update in place, not duplicate")
        XCTAssertEqual(rows.first?.statusRaw, 1, "status updated in place")

        // 3. Removal keyed on the raw 36-byte outpoint deletes the row. The T1
        //    seam: `persistInvitations` derives the removal key via
        //    `encodeOutPoint(rawBytes:)`, which must equal the upsert's stored
        //    `outPointHex` for the delete to match.
        handler.beginChangeset(walletId: walletId)
        handler.persistInvitations(walletId: walletId, upserts: [], removed: [rawOutPoint])
        _ = handler.endChangeset(walletId: walletId, success: true)

        rows = try fetchRows(container)
        XCTAssertEqual(
            rows.count, 0,
            "removal via encodeOutPoint(rawBytes:) must match the upsert key and delete the row"
        )
    }
}
