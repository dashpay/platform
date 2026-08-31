import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Tests the pure funding-type + status predicate behind the wallet
/// detail screen's "Pending Platform Top Ups" surface.
///
/// This is the only surface in the app that can recover an orphaned
/// TOP-UP asset lock. The identity-side
/// `IdentitiesContentView.crossWalletResumableLocks` admits only
/// funding types `0...2`, and `3` (IdentityInvitation) is a bearer
/// voucher owned by the reclaim flow — so a lock at funding type `4`
/// or `5` that this filter rejects is unreachable from anywhere in the
/// app, and reads to the user as lost funds.
///
/// Two invariants can silently regress:
///
///   1. Funding type `5` (AssetLockShieldedAddressTopUp) must be
///      admitted. The filter was `fundingTypeRaw == 4` alone, which
///      hid every shielded top-up — stalled or RecoveredFromChain —
///      on every surface. Widening the STATUS predicate did not help:
///      a row excluded on the funding-type axis stays excluded
///      however generous the status axis is.
///   2. The status half must keep excluding the terminal Consumed
///      (`4`) while admitting RecoveredFromChain (`5`) — delegated to
///      `isVisibleAsResumable`, and asserted here because this
///      surface is the one that composes the two axes.
final class PendingPlatformTopUpResumeTests: XCTestCase {

    private struct FakeAssetLockRow: AssetLockResumeRow, Equatable {
        let walletId: Data
        let statusRaw: Int
        var identityIndexRaw: Int32 = 0
        let fundingTypeRaw: Int
    }

    private let wallet = Data(repeating: 0xA1, count: 8)

    private func row(status: Int, fundingType: Int) -> FakeAssetLockRow {
        FakeAssetLockRow(walletId: wallet, statusRaw: status, fundingTypeRaw: fundingType)
    }

    // MARK: - funding-type axis

    /// The regression: a shielded top-up must be admitted, at every
    /// recoverable status.
    func testAdmitsShieldedAddressTopUps() {
        for status in [1, 2, 3, 5] {
            XCTAssertTrue(
                PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                    row(status: status, fundingType: 5)
                ),
                "shielded top-up at status \(status) must be resumable"
            )
        }
    }

    /// Address top-ups are unaffected by the widening.
    func testAdmitsAddressTopUps() {
        for status in [1, 2, 3, 5] {
            XCTAssertTrue(
                PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                    row(status: status, fundingType: 4)
                ),
                "address top-up at status \(status) must be resumable"
            )
        }
    }

    /// Identity-family funding types recover on the identity screens.
    /// Admitting one here would offer a resume flow that submits the
    /// wrong transition against it.
    func testRejectsIdentityFamilyAndUnknownFundingTypes() {
        for fundingType in [0, 1, 2, 3, 6, 99] {
            XCTAssertFalse(
                PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                    row(status: 2, fundingType: fundingType)
                ),
                "fundingTypeRaw \(fundingType) must not reach the top-up surface"
            )
        }
    }

    // MARK: - status axis

    /// Built (0) has not been broadcast; Consumed (4) is the terminal
    /// tombstone `resume_asset_lock` rejects outright. Both stay out on
    /// either funding type.
    func testRejectsBuiltAndConsumedOnBothTopUpTypes() {
        for fundingType in [4, 5] {
            for status in [0, 4] {
                XCTAssertFalse(
                    PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                        row(status: status, fundingType: fundingType)
                    ),
                    "status \(status) on fundingType \(fundingType) must stay hidden"
                )
            }
        }
    }

    /// `4` is excluded by NAME, not by an upper bound: `5` sits above
    /// it numerically and is resumable. A `statusRaw <= 3` bound reads
    /// as equivalent and silently drops every restored lock.
    func testConsumedExclusionIsNotAnUpperBound() {
        XCTAssertFalse(
            PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                row(status: 4, fundingType: 5)
            )
        )
        XCTAssertTrue(
            PendingPlatformFundFromAssetLocksList.isResumableTopUp(
                row(status: 5, fundingType: 5)
            )
        )
    }

    /// RecoveredFromChain carries a real `ChainAssetLockProof`, so it is
    /// as fundable as a ChainLocked (3) lock — the row must offer Resume
    /// rather than the "waiting for finality" indicator.
    func testRecoveredFromChainIsFundableNotWaiting() {
        XCTAssertTrue(row(status: 5, fundingType: 5).canFundIdentity)
        XCTAssertTrue(row(status: 5, fundingType: 4).canFundIdentity)
        XCTAssertFalse(row(status: 1, fundingType: 5).canFundIdentity)
    }
}
