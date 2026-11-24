import XCTest
@testable import SwiftExampleApp

/// Display metadata for shielded activity kinds (`ShieldedActivityKindDisplay`),
/// covering the fee-only spends Rust records as kinds 13
/// (PrivateDocumentCreate) and 14 (PrivateContactRequest).
final class ShieldedActivityKindDisplayTests: XCTestCase {

    // MARK: - Labels

    func testShouldLabelAPrivateDocumentCreateAsPrivateDocumentEntry() {
        XCTAssertEqual(ShieldedActivityKindDisplay.label(13), "Private Document Entry")
    }

    func testShouldLabelAPrivateContactRequestByName() {
        XCTAssertEqual(ShieldedActivityKindDisplay.label(14), "Private Contact Request")
    }

    func testShouldKeepTheGenericLabelForTheResidualKindAndAnUnknownTag() {
        XCTAssertEqual(ShieldedActivityKindDisplay.label(7), "Shielded Spend")
        XCTAssertEqual(ShieldedActivityKindDisplay.label(15), "Shielded Spend")
    }

    // MARK: - Classification

    func testShouldShowTheRestoreFootnoteOnlyForTheResidualKind() {
        XCTAssertTrue(ShieldedActivityKindDisplay.isUnclassifiedResidual(7))
        XCTAssertFalse(ShieldedActivityKindDisplay.isUnclassifiedResidual(13))
        XCTAssertFalse(ShieldedActivityKindDisplay.isUnclassifiedResidual(14))
    }

    func testShouldShowAFeeOnlySpendsAmountInCredits() {
        XCTAssertFalse(ShieldedActivityKindDisplay.amountIsTokens(13))
        XCTAssertFalse(ShieldedActivityKindDisplay.amountIsTokens(14))
    }

    func testShouldTreatOnlyTheTwoPrivateKindsAsFeeOnlySpends() {
        XCTAssertTrue(ShieldedActivityKindDisplay.isFeeOnlySpend(13))
        XCTAssertTrue(ShieldedActivityKindDisplay.isFeeOnlySpend(14))
        XCTAssertFalse(ShieldedActivityKindDisplay.isFeeOnlySpend(7))
        XCTAssertFalse(ShieldedActivityKindDisplay.isFeeOnlySpend(12))
        XCTAssertFalse(ShieldedActivityKindDisplay.isFeeOnlySpend(15))
    }

    func testShouldGiveTheFeeOnlySpendsTheirOwnIcons() {
        XCTAssertEqual(ShieldedActivityKindDisplay.icon(13), "doc.badge.plus")
        XCTAssertEqual(ShieldedActivityKindDisplay.icon(14), "person.badge.plus")
    }

    // MARK: - Fee-only spend note

    func testShouldSayTheFeeLeftThePoolOnceConfirmed() {
        XCTAssertEqual(
            ShieldedActivityKindDisplay.feeOnlySpendNote(status: 1),
            "The amount is the fee, which is all that left the shielded pool."
        )
    }

    func testShouldSayTheFeeWasNotChargedWhenFailed() {
        XCTAssertEqual(
            ShieldedActivityKindDisplay.feeOnlySpendNote(status: 2),
            "It did not go through; the fee shown was not charged."
        )
    }

    func testShouldSayNotConfirmedYetWhilePendingOrUnknown() {
        let expected = "Not confirmed yet: a shielded sync confirms it once it lands. The amount is the fee, which is all that leaves the shielded pool."
        XCTAssertEqual(ShieldedActivityKindDisplay.feeOnlySpendNote(status: 0), expected)
        XCTAssertEqual(ShieldedActivityKindDisplay.feeOnlySpendNote(status: 3), expected)
    }
}
