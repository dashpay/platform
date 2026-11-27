import XCTest
@testable import SwiftExampleApp

/// A Platform-address funding (fresh, owned resume or external resume)
/// completes without a balance when its result carries no update for the
/// picked address, instead of reporting a false 0: reconciliation can
/// return an empty changeset when sync already applied the balance.
@MainActor
final class AddressFundReportedBalanceTests: XCTestCase {

    private let picked = Data(repeating: 0x11, count: 20)
    private let other = Data(repeating: 0x22, count: 20)

    func testTheReportedBalanceIsThePickedAddressUpdate() {
        XCTAssertEqual(
            AddressFundFromAssetLockController.reportedBalance(
                of: picked,
                in: [(other, 7), (picked, 5_000)]
            ),
            5_000
        )
    }

    func testNoUpdateForThePickedAddressReportsNoBalanceNotZero() {
        XCTAssertNil(AddressFundFromAssetLockController.reportedBalance(of: picked, in: []))
        XCTAssertNil(AddressFundFromAssetLockController.reportedBalance(of: picked, in: [(other, 7)]))
    }

    func testAFundingWithoutABalanceCompletesWithoutOne() async {
        let controller = AddressFundFromAssetLockController(
            walletId: Data(repeating: 0xA1, count: 32),
            platformAccountIndex: 0,
            recipientHash: picked,
            recipientType: 0
        )
        controller.submit { nil }
        for _ in 0..<100 where controller.phase == .inFlight {
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTAssertEqual(controller.phase, .completed(newBalance: nil))
    }
}
