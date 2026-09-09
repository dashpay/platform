// SignedCoreTransactionTests.swift
// SwiftDashSDKTests
//
// Unit tests for `SignedCoreTransaction`'s reservation-token ownership.

import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the Swift side of the deferred (BIP70/BIP270) signed-payment
/// flow exported by dashpay/platform#4308.
///
/// What is Swift's to get right is token OWNERSHIP: the native registration
/// mints the reservation token before `SignedCoreTransaction` exists, so the
/// object must release it exactly once — on `close()`, or on `deinit` for a
/// payment that was abandoned without either a broadcast or an explicit close.
/// The lifecycle semantics themselves (atomic consume-before-send, idempotent
/// release, tokens bound to their originating wallet generation) live in Rust
/// and are covered there.
///
/// These tests are hermetic: no wallet, no network. The release action is
/// injected through the internal `releaseToken` seam so the count can be
/// observed without reaching into the process-global registry. Kotlin parity:
/// `SignedCoreTransactionTest`.
final class SignedCoreTransactionTests: XCTestCase {

    /// Records every token handed to the injected release action.
    private final class ReleaseRecorder {
        private(set) var released: [UInt64] = []
        var count: Int { released.count }

        func record(_ token: UInt64) {
            released.append(token)
        }
    }

    private func makePayment(
        token: UInt64 = 0xDEAD_BEEF,
        recorder: ReleaseRecorder
    ) -> SignedCoreTransaction {
        SignedCoreTransaction(
            txidHex: "b0c1d2e3f4a5968778695a4b3c2d1e0fb0c1d2e3f4a5968778695a4b3c2d1e0f",
            rawTxBytes: Data([0x01, 0x02, 0x03, 0x04]),
            feeDuffs: 2_260,
            reservationToken: token,
            releaseToken: { recorder.record($0) }
        )
    }

    func testShouldRoundTripPropertiesThroughInit() {
        let recorder = ReleaseRecorder()
        let payment = makePayment(token: 42, recorder: recorder)

        XCTAssertEqual(
            payment.txidHex,
            "b0c1d2e3f4a5968778695a4b3c2d1e0fb0c1d2e3f4a5968778695a4b3c2d1e0f"
        )
        XCTAssertEqual(payment.rawTxBytes, Data([0x01, 0x02, 0x03, 0x04]))
        XCTAssertEqual(payment.feeDuffs, 2_260)
        XCTAssertEqual(payment.reservationToken, 42)
        XCTAssertEqual(recorder.count, 0, "construction must not release the token")

        payment.close()
    }

    func testShouldReleaseTokenOnceOnCloseAndIgnoreASecondClose() {
        let recorder = ReleaseRecorder()
        let payment = makePayment(token: 7, recorder: recorder)

        payment.close()
        XCTAssertEqual(recorder.released, [7])

        payment.close()
        XCTAssertEqual(recorder.released, [7], "a second close must not release again")
    }

    func testShouldReleaseTokenOnceViaDeinitWhenNeverClosed() {
        let recorder = ReleaseRecorder()
        var payment: SignedCoreTransaction? = makePayment(token: 99, recorder: recorder)
        XCTAssertNotNil(payment)
        XCTAssertEqual(recorder.count, 0, "a live payment must keep its token")

        payment = nil
        XCTAssertEqual(
            recorder.released, [99],
            "an abandoned payment must release its reservation from deinit"
        )
    }

    func testShouldNotDoubleReleaseWhenCloseIsFollowedByDeinit() {
        let recorder = ReleaseRecorder()
        var payment: SignedCoreTransaction? = makePayment(token: 5, recorder: recorder)

        payment?.close()
        XCTAssertEqual(recorder.released, [5])

        payment = nil
        XCTAssertEqual(recorder.released, [5], "deinit must not re-release a closed payment")
    }

    /// Symbol-binding smoke test against the REAL release path: releasing a
    /// token the registry never minted is a silent native no-op, so the default
    /// seam must neither throw nor crash. This is what links
    /// `core_wallet_signed_payment_release`.
    func testShouldTreatReleaseOfAnUnknownTokenAsASilentNoOp() {
        XCTAssertNoThrow(try core_wallet_signed_payment_release(0xFFFF_FFFF_FFF0).check())

        let payment = SignedCoreTransaction(
            txidHex: "00",
            rawTxBytes: Data(),
            feeDuffs: 0,
            reservationToken: 0xFFFF_FFFF_FFF1
        )
        payment.close()
    }
}
