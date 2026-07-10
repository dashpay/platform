import XCTest
@testable import SwiftExampleApp

/// Pins `ReclaimInvitationSheet.isAlreadyConsumed(message:)` — the classifier
/// that decides whether a failed reclaim is the benign "voucher already claimed"
/// case (flip the row to Claimed, show a neutral message) versus a real error
/// (surface it).
///
/// The SDK surfaces a consensus error as
/// `"SDK error: Protocol error: <consensus Display verbatim>"`, so the match is
/// keyed on the exact canonical Display of
/// `IdentityAssetLockTransactionOutPointAlreadyConsumedError` —
/// "…already completely used". The critical safety property is the **absence**
/// of false positives: a different asset-lock failure (notably the
/// not-enough-credits error, which shares the "Asset lock transaction …" prefix)
/// must NOT be misclassified as already-consumed, or the UI would wrongly flip a
/// still-live invitation to Claimed.
final class ReclaimInvitationClassifierTests: XCTestCase {

    /// The real already-consumed rejection, as surfaced to Swift.
    func test_alreadyConsumedDisplay_classifiedTrue() {
        let message = "SDK error: Protocol error: Asset lock transaction "
            + "3ff8e26d02e53f97a5f06b12327f40fc10cb859077e2788362c5d93032850ff0 "
            + "output 0 already completely used"
        XCTAssertTrue(ReclaimInvitationSheet.isAlreadyConsumed(message: message))
    }

    /// Case-insensitive: consensus Display wording can be re-cased upstream.
    func test_alreadyConsumed_caseInsensitive() {
        XCTAssertTrue(
            ReclaimInvitationSheet.isAlreadyConsumed(message: "ALREADY COMPLETELY USED")
        )
    }

    /// The not-enough-credits error shares the "Asset lock transaction …" prefix
    /// but is a DIFFERENT failure (the voucher is still live). Must be false —
    /// this is the false-positive the narrowed classifier exists to prevent.
    func test_notEnoughCreditsError_classifiedFalse() {
        let message = "SDK error: Protocol error: Asset lock transaction "
            + "3ff8e26d02e53f97a5f06b12327f40fc10cb859077e2788362c5d93032850ff0 "
            + "output 0 only has 50000000 credits left out of 50000000 initial "
            + "credits on the asset lock but needs 50500000 credits to start processing"
        XCTAssertFalse(ReclaimInvitationSheet.isAlreadyConsumed(message: message))
    }

    /// An unrelated transport failure must not be swallowed as "already claimed".
    func test_networkError_classifiedFalse() {
        XCTAssertFalse(
            ReclaimInvitationSheet.isAlreadyConsumed(
                message: "SDK error: Transport error: connection refused"
            )
        )
    }

    /// The broad phrases dropped from the classifier must NOT match on their own
    /// — they never appear in the real Display and would widen false positives.
    func test_droppedBroadPhrases_classifiedFalse() {
        XCTAssertFalse(ReclaimInvitationSheet.isAlreadyConsumed(message: "already consumed"))
        XCTAssertFalse(ReclaimInvitationSheet.isAlreadyConsumed(message: "AlreadyConsumed"))
    }
}
