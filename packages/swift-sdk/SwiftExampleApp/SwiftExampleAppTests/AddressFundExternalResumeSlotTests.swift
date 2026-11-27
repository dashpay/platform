import XCTest
@testable import SwiftExampleApp

/// An external resume pays its lock's recorded recipients, not a picked
/// account or address, so it starts without either and its slot is the
/// wallet and the outpoint alone: a wallet with no unused address can still
/// finish it, and a second tap on the same lock reaches the same controller.
@MainActor
final class AddressFundExternalResumeSlotTests: XCTestCase {

    private let walletId = Data(repeating: 0xA2, count: 32)
    private let outPoint = String(repeating: "ab", count: 32) + ":0"

    private func startExternalResume(
        on coordinator: AddressFundFromAssetLockCoordinator
    ) -> AddressFundFromAssetLockController {
        coordinator.startFunding(
            walletId: walletId,
            platformAccountIndex: nil,
            recipientHash: nil,
            recipientType: nil,
            assetLockOutPoint: outPoint
        ) {
            try await Task.sleep(nanoseconds: 5_000_000_000)
            return nil
        }
    }

    func testAnExternalResumeStartsWithoutAPickedAccountOrAddress() {
        let coordinator = AddressFundFromAssetLockCoordinator()
        let controller = startExternalResume(on: coordinator)

        XCTAssertEqual(controller.phase, .inFlight)
        XCTAssertNil(controller.platformAccountIndex)
        XCTAssertNil(controller.recipientHash)
        XCTAssertNil(controller.recipientType)
        XCTAssertEqual(controller.assetLockOutPoint, outPoint)
    }

    func testASecondExternalResumeOfTheSameLockReachesTheSameController() {
        let coordinator = AddressFundFromAssetLockCoordinator()
        let first = startExternalResume(on: coordinator)
        let second = startExternalResume(on: coordinator)

        XCTAssertTrue(first === second)
        XCTAssertEqual(coordinator.activeControllers().count, 1)
    }

    func testDismissingAnExternalResumeByWalletAndOutPointRemovesIt() {
        let coordinator = AddressFundFromAssetLockCoordinator()
        _ = startExternalResume(on: coordinator)

        coordinator.dismiss(
            walletId: walletId,
            platformAccountIndex: nil,
            recipientHash: nil,
            assetLockOutPoint: outPoint
        )

        XCTAssertTrue(coordinator.activeControllers().isEmpty)
    }
}
