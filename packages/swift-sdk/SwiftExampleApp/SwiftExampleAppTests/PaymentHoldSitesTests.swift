import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// The payment screens that keep their last failure: after one that holds
/// its slot (`.maybeSent`, `.lockHeld`, no class) they refuse a fresh
/// payment, after a `.notSent` one they allow it, and an error that is not
/// an SDK error never clears the hold.
final class PaymentHoldSitesTests: XCTestCase {
    private static let held = AssetLockBuiltOutPoint(
        rawBytes: Data(repeating: 7, count: 32) + Data([1, 0, 0, 0])
    )!

    /// The failing class of each row, and whether a fresh payment is allowed after it.
    private static let table: [(PlatformPaymentClass?, Bool)] = [
        (.maybeSent, false),
        (PlatformPaymentClass(delivery: .lockHeld(held), isRetryable: false), false),
        (nil, false),
        (PlatformPaymentClass(delivery: .notSent, isRetryable: true), true),
        (PlatformPaymentClass(delivery: .notSent, isRetryable: false), true),
    ]

    private static func error(_ payment: PlatformPaymentClass?) -> Error {
        PlatformWalletError.walletOperation("refused", payment: payment)
    }

    func testCreateInvitationHoldsAfterAFailureThatKeepsItsSlot() {
        for (payment, permits) in Self.table {
            let kept = CreateInvitationSheet.failureAfterCreate(nil, error: Self.error(payment))
            XCTAssertEqual(kept?.invitationVoucher, true)
            XCTAssertEqual(PaymentFailure.permitsFreshPayment(after: kept), permits, "\(String(describing: payment))")
        }
        // A held voucher's text sends the user to reclaim it.
        let voucher = CreateInvitationSheet.failureAfterCreate(nil, error: Self.error(Self.table[1].0))
        XCTAssertTrue(voucher?.message.contains("Sent invitations") == true)
        // An error that is not an SDK error keeps the hold.
        let hold = CreateInvitationSheet.failureAfterCreate(nil, error: Self.error(.maybeSent))
        XCTAssertEqual(CreateInvitationSheet.failureAfterCreate(hold, error: CocoaError(.fileNoSuchFile)), hold)
    }

    @MainActor
    func testTopUpAddressFromAssetLockHoldsAfterAFailureThatKeepsItsSlot() {
        for (payment, permits) in Self.table {
            let vm = TopUpAddressFromAssetLockViewModel()
            vm.applyTopUpFailure(Self.error(payment))
            XCTAssertEqual(vm.permitsTopUp, permits, "\(String(describing: payment))")
            XCTAssertNotNil(vm.errorMessage)
        }
        let vm = TopUpAddressFromAssetLockViewModel()
        vm.applyTopUpFailure(Self.error(.maybeSent))
        // An error that is not an SDK error, and a form reset, keep the hold.
        vm.applyTopUpFailure(CocoaError(.fileNoSuchFile))
        vm.reset()
        XCTAssertFalse(vm.permitsTopUp)
        // Only the user's confirmed release clears it.
        vm.releasePaymentHold()
        XCTAssertTrue(vm.permitsTopUp)
    }
}
