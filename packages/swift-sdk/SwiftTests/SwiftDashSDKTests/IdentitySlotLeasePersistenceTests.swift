import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Invitation-claim slot leases round-trip through SwiftData and the
/// persistence extension's C shims.
final class IdentitySlotLeasePersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let invitation = Data(repeating: 7, count: 32) + Data([3, 0, 0, 0])
    private let otherInvitation = Data(repeating: 8, count: 32) + Data([0, 0, 0, 0])

    private typealias Lease = PlatformWalletPersistenceHandler.IdentitySlotLeaseRow

    private func makeHandler() throws -> (ModelContainer, PlatformWalletPersistenceHandler) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        return (container, PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet))
    }

    private func round(
        _ handler: PlatformWalletPersistenceHandler,
        leases: [Lease] = [],
        released: [Data] = [],
        success: Bool = true
    ) throws {
        handler.beginChangeset(walletId: walletId)
        try handler.persistIdentitySlotLeases(walletId: walletId, leases: leases, released: released)
        XCTAssertEqual(handler.endChangeset(walletId: walletId, success: success), success)
    }

    func testShouldDeclareTheLeaseCapabilityAndWireItsCallbacks() {
        let (_, handler) = try! makeHandler()
        XCTAssertNotEqual(
            handler.makePersistenceCapabilities().bits & PlatformWalletPersistenceCapabilities.identitySlotLeases, 0)
        let ext = handler.makePersistenceCallbacksExtension()
        XCTAssertNotNil(ext.on_persist_identity_slot_leases_fn)
        XCTAssertNotNil(ext.on_load_identity_slot_leases_fn)
        XCTAssertNotNil(ext.on_load_identity_slot_leases_free_fn)
    }

    func testShouldStoreUpdateAndReleaseALeaseWithItsRound() throws {
        let (_, handler) = try makeHandler()
        try round(handler, leases: [
            .init(invitationOutPoint: invitation, identityIndex: 4, phase: 0),
            .init(invitationOutPoint: otherInvitation, identityIndex: 5, phase: 0),
        ])
        XCTAssertEqual(Set(try handler.loadIdentitySlotLeases(walletId: walletId).map(\.identityIndex)), [4, 5])

        // The phase moves on; the other lease is released.
        try round(
            handler, leases: [.init(invitationOutPoint: invitation, identityIndex: 4, phase: 1)],
            released: [otherInvitation])
        XCTAssertEqual(
            try handler.loadIdentitySlotLeases(walletId: walletId),
            [.init(invitationOutPoint: invitation, identityIndex: 4, phase: 1)])

        // A rolled-back round leaves the leases as they were.
        try round(handler, released: [invitation], success: false)
        XCTAssertEqual(try handler.loadIdentitySlotLeases(walletId: walletId).count, 1)
    }

    func testShouldScopeLeasesByWalletAndNetworkAndPurgeThemWithTheWallet() throws {
        let (container, handler) = try makeHandler()
        try round(handler, leases: [.init(invitationOutPoint: invitation, identityIndex: 2, phase: 1)])
        XCTAssertTrue(try handler.loadIdentitySlotLeases(walletId: Data(repeating: 43, count: 32)).isEmpty)
        let mainnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        XCTAssertTrue(try mainnet.loadIdentitySlotLeases(walletId: walletId).isEmpty)

        try handler.deleteWalletData(walletId: walletId)
        XCTAssertEqual(try ModelContext(container).fetchCount(FetchDescriptor<PersistentIdentitySlotLease>()), 0)
    }

    func testShouldHandLeasesToRustThroughTheShims() throws {
        let (_, handler) = try makeHandler()
        let ext = handler.makePersistenceCallbacksExtension()
        let persist = try XCTUnwrap(ext.on_persist_identity_slot_leases_fn)
        let load = try XCTUnwrap(ext.on_load_identity_slot_leases_fn)
        let free = try XCTUnwrap(ext.on_load_identity_slot_leases_free_fn)
        let shimContext = Unmanaged.passUnretained(handler).toOpaque()

        var row = IdentitySlotLeaseFFI()
        Swift.withUnsafeMutableBytes(of: &row.invitation_out_point) { _ = invitation.copyBytes(to: $0) }
        row.identity_index = 9
        row.phase = 1
        handler.beginChangeset(walletId: walletId)
        let stored = walletId.withUnsafeBytes { wallet in
            withUnsafePointer(to: &row) {
                persist(shimContext, wallet.bindMemory(to: UInt8.self).baseAddress, $0, 1, nil, 0)
            }
        }
        XCTAssertEqual(stored, 0)
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        var out: UnsafePointer<IdentitySlotLeaseFFI>?
        var count: UInt = 0
        let loaded = walletId.withUnsafeBytes {
            load(shimContext, $0.bindMemory(to: UInt8.self).baseAddress, &out, &count)
        }
        XCTAssertEqual(loaded, 0)
        XCTAssertEqual(count, 1)
        let back = try XCTUnwrap(out).pointee
        var outPoint = back.invitation_out_point
        XCTAssertEqual(Swift.withUnsafeBytes(of: &outPoint) { Data($0) }, invitation)
        XCTAssertEqual(back.identity_index, 9)
        XCTAssertEqual(back.phase, 1)
        free(shimContext, out, count)
    }
}
