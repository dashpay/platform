import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// A rebind request is built from the native account snapshot, which also
/// lists the tip accounts Rust binds itself. Only ordinary accounts may go
/// back into the request, or more than 64 tip accounts would exceed the bind
/// request cap.
@MainActor
final class ShieldedRebindRequestTests: XCTestCase {
    private func tipAccounts(_ count: UInt32) throws -> [UInt32] {
        try (0..<count).map { try PlatformWalletManager.shieldedTipAccountIndex(identityIndex: $0) }
    }

    func testDropsTipAccountsFromASnapshotLargerThanTheRequestCap() throws {
        let snapshot = [UInt32(0)] + (try tipAccounts(70))
        XCTAssertGreaterThan(snapshot.count, 64)
        XCTAssertEqual(ShieldedService.ordinaryBindRequest(snapshot), [0])
    }

    func testKeepsEveryOrdinaryAccount() throws {
        let snapshot = [UInt32(0), 1, 2] + (try tipAccounts(3))
        XCTAssertEqual(ShieldedService.ordinaryBindRequest(snapshot), [0, 1, 2])
    }

    func testFallsBackToAccountZero() throws {
        XCTAssertEqual(ShieldedService.ordinaryBindRequest([]), [0])
        XCTAssertEqual(ShieldedService.ordinaryBindRequest(try tipAccounts(5)), [0])
    }
}
