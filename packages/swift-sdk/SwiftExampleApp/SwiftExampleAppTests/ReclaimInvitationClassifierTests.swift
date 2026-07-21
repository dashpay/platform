import XCTest
import SwiftDashSDK
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

    func test_typedAlreadyConsumed_classifiedTrue() {
        let error = PlatformWalletError.assetLockAlreadyConsumed("deadbeef:0")
        XCTAssertTrue(ReclaimInvitationSheet.isAlreadyConsumed(error))
    }

    func test_typedNotTracked_classifiedFalse() {
        let error = PlatformWalletError.assetLockNotTracked("deadbeef:0")
        XCTAssertFalse(ReclaimInvitationSheet.isAlreadyConsumed(error))
    }

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

    // MARK: - Terminal-outcome decision (classifyReclaimFailure)

    private struct StubError: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    private static let alreadyConsumed = StubError(
        message: "SDK error: Protocol error: Asset lock transaction "
            + "3ff8e26d02e53f97a5f06b12327f40fc10cb859077e2788362c5d93032850ff0 "
            + "output 0 already completely used"
    )

    /// Already-consumed + our own reclaim was in flight ⇒ explicitly ambiguous,
    /// NEVER `.reclaimed`: the marker only proves a local attempt started a
    /// consume, not that it landed — the invitee can claim between our crash
    /// and the retry, and a Reclaimed recovery would misattribute that claim.
    func test_classify_alreadyConsumed_priorInFlight_isAmbiguous() {
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.alreadyConsumed, hadPriorReclaimInFlight: true),
            .consumedAmbiguous
        )
    }

    /// Already-consumed + no prior reclaim ⇒ the invitee claimed it first (Claimed).
    func test_classify_alreadyConsumed_noPrior_isClaimed() {
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.alreadyConsumed, hadPriorReclaimInFlight: false),
            .claimed
        )
    }

    /// A non-already-consumed failure is `.error` regardless of the marker — the
    /// row is left as-is. This is the safety net behind the S3 marker-placement
    /// fix: a pre-broadcast local failure (never already-consumed, and with the
    /// marker never set) must never be mistaken for a self-reclaim or a claim.
    func test_classify_otherError_isError_regardlessOfMarker() {
        let other = StubError(message: "SDK error: Transport error: connection refused")
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: other, hadPriorReclaimInFlight: true),
            .error
        )
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: other, hadPriorReclaimInFlight: false),
            .error
        )
    }

    private static let lockNotTracked = StubError(
        message: "PlatformWalletError: Asset lock "
            + "3ff8e26d02e53f97a5f06b12327f40fc10cb859077e2788362c5d93032850ff0:0 is not tracked"
    )

    /// A retry after our own crash-interrupted consume can fail LOCALLY
    /// ("…is not tracked"). With the marker set that is consistent with our
    /// consume having landed, but it is NOT on-chain proof — so it resolves to
    /// the explicitly ambiguous `.untrackedAfterOwnAttempt`, never `.reclaimed`.
    func test_classify_lockNotTracked_priorInFlight_isUntrackedAmbiguous() {
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: true),
            .untrackedAfterOwnAttempt
        )
    }

    /// The same local error WITHOUT a prior in-flight marker is `.error` — a first
    /// attempt that hits "not tracked" is a genuine anomaly, not a self-reclaim.
    func test_classify_lockNotTracked_noPrior_isError() {
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: false),
            .error
        )
    }

    // MARK: - Stale-marker clearing (shouldClearInFlightMarker)

    /// A first attempt that set the marker itself and then failed the LOCAL
    /// pre-broadcast "not tracked" guard must clear the marker: the consume
    /// never started, so the marker is demonstrably stale.
    func test_shouldClear_lockNotTracked_noPrior_isTrue() {
        XCTAssertTrue(
            ReclaimInvitationSheet.shouldClearInFlightMarker(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: false)
        )
    }

    /// With a PRIOR in-flight marker the same error is the ambiguous
    /// crash-recovery signal (classified `.untrackedAfterOwnAttempt` above)
    /// — never a clear.
    func test_shouldClear_lockNotTracked_prior_isFalse() {
        XCTAssertFalse(
            ReclaimInvitationSheet.shouldClearInFlightMarker(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: true)
        )
    }

    /// Errors that may have reached the network keep the marker regardless —
    /// a later "already consumed" must stay classified as ambiguous rather
    /// than a provable foreign claim.
    func test_shouldClear_networkishErrors_isFalse() {
        let transport = StubError(message: "SDK error: Transport error: connection refused")
        XCTAssertFalse(
            ReclaimInvitationSheet.shouldClearInFlightMarker(
                error: transport, hadPriorReclaimInFlight: false)
        )
        XCTAssertFalse(
            ReclaimInvitationSheet.shouldClearInFlightMarker(
                error: Self.alreadyConsumed, hadPriorReclaimInFlight: false)
        )
    }

    /// The two-attempt regression the clearing exists for: attempt 1 sets the
    /// marker and fails locally ("not tracked", no prior) → `.error` + clear;
    /// an identical retry therefore STILL sees no prior marker and stays
    /// `.error` — a purely-local failure can never escalate past `.error`.
    /// (Without the clear, the retry would capture `hadPriorReclaimInFlight ==
    /// true` and degrade into the ambiguous outcome for no reason.)
    func test_twoAttempt_localFailure_neverBecomesReclaimed() {
        // Attempt 1: marker freshly set by this attempt (prior == false).
        var persistedMarker = false // durable value BEFORE the attempt
        let attempt1Prior = persistedMarker
        persistedMarker = true // markInFlight()
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: attempt1Prior),
            .error
        )
        if ReclaimInvitationSheet.shouldClearInFlightMarker(
            error: Self.lockNotTracked, hadPriorReclaimInFlight: attempt1Prior
        ) {
            persistedMarker = false // the fix under test
        }

        // Attempt 2 (identical retry): captures the durable marker as prior.
        let attempt2Prior = persistedMarker
        XCTAssertEqual(
            ReclaimInvitationSheet.classifyReclaimFailure(
                error: Self.lockNotTracked, hadPriorReclaimInFlight: attempt2Prior),
            .error,
            "a repeated purely-local failure must stay an error, not become Reclaimed"
        )
    }
}
