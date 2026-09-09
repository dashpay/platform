import XCTest
@testable import SwiftDashSDK

/// End-to-end coverage of the deferred (BIP70/BIP270) finalize / broadcast /
/// release C ABI against the funded local devnet.
///
/// This is the layer neither sibling suite reaches. `SignedCoreTransactionTests`
/// runs entirely above the boundary (it injects the release action and never
/// calls the native finalizer), and the Rust `signed_payment_registry` tests run
/// entirely below it (they never marshal anything through the C ABI). What only
/// these tests can catch: the eleven-argument out-parameter marshalling of
/// `core_wallet_signed_payment_finalize`, ownership of the returned txid string,
/// the copy of the borrowed bytes view before its backing `FFICoreTransaction`
/// is freed, the error-code mapping of the deferred-token failures onto the
/// typed `PlatformWalletError` arms, and the consume-once / release-frees-inputs
/// behaviour of a reservation actually held by a live wallet.
final class DeferredSignedPaymentIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 {
        UInt64(fundingDash * 1e8)
    }

    private let paymentAmount: UInt64 = 100_000 // 0.001 DASH

    func testBuildBroadcastConsumesTokenExactlyOnce() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "deferred-alice")
        let bob = try await env.makeTestWallet(name: "deferred-bob")

        let aliceAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: aliceAddress, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let bobAddress = try bob.getCoreWallet().nextReceiveAddress()
        let network = try alice.getCoreWallet().network()
        let payment = try alice.getPlatformWallet().buildSignedPayment(
            recipients: [(address: bobAddress, amountDuffs: paymentAmount)],
            network: network
        )

        // Every field below crossed the boundary through a separate out
        // parameter, so each one is its own marshalling assertion.
        XCTAssertNotEqual(payment.reservationToken, 0, "finalize must mint a reservation token")
        XCTAssertGreaterThan(payment.feeDuffs, 0, "a signed build always charges a fee")
        XCTAssertFalse(
            payment.rawTxBytes.isEmpty,
            "the borrowed bytes view must be copied out before out_tx is freed"
        )
        XCTAssertEqual(payment.txidHex.count, 64, "txid must be 32 bytes of hex")
        let hexDigits = Set("0123456789abcdef")
        XCTAssertTrue(
            payment.txidHex.allSatisfy { hexDigits.contains($0) },
            "txid must be lowercase hex, got \(payment.txidHex)"
        )

        let beforeTxids = try await readTxids()
        let txid = try alice.getPlatformWallet().broadcastSigned(payment)
        // Rust computed `txidHex` from the signed bytes at finalize time and the
        // broadcast reports what it actually sent; that these agree is what
        // proves the merchant server was handed the bytes that got broadcast.
        XCTAssertEqual(txid, payment.txidHex, "broadcast txid must match the one finalize returned")

        guard let newTxid = try await waitForNewTxid(notIn: beforeTxids) else {
            XCTFail("deferred payment PersistentTransaction row never appeared")
            return
        }
        _ = try await env.mine(1, including: newTxid)

        try await Wait.until(
            "bob credited \(paymentAmount) duffs by the deferred payment",
            timeout: 60,
            pollInterval: 0.01
        ) {
            try bob.getPlatformWallet().balance().spendable == paymentAmount
        }

        // The token is consumed atomically before the send, so the second
        // broadcast is refused rather than sending again. Proves code 35 maps to
        // the typed arm across the ABI, not just in the hermetic mirror.
        XCTAssertThrowsError(
            try alice.getPlatformWallet().broadcastSigned(token: payment.reservationToken)
        ) { error in
            guard case PlatformWalletError.reservationTokenConsumed = error else {
                XCTFail("re-broadcast must throw .reservationTokenConsumed, got \(error)")
                return
            }
        }

        // Releasing a consumed token is a silent native no-op, so a defensive
        // release on the merchant-acked path must not throw.
        XCTAssertNoThrow(try alice.getPlatformWallet().releaseReservation(payment))
    }

    func testReleaseFreesTheReservedInputs() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "deferred-release-alice")

        let aliceAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: aliceAddress, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let network = try alice.getCoreWallet().network()
        // Nothing here is ever broadcast, so the destination is immaterial;
        // paying alice's own receive address keeps this to one funded wallet.
        let destination = aliceAddress

        let held = try alice.getPlatformWallet().buildSignedPayment(
            recipients: [(address: destination, amountDuffs: paymentAmount)],
            network: network
        )
        XCTAssertNotEqual(held.reservationToken, 0, "finalize must mint a reservation token")

        // The single funding UTXO is now reserved, so a second build has nothing
        // to select. Coin selection reports that as either "not enough funds" or
        // "no selectable inputs" depending on which check gives up first — both
        // mean the same thing here, so accept either rather than pinning the
        // selector's internal ordering.
        XCTAssertThrowsError(
            try alice.getPlatformWallet().buildSignedPayment(
                recipients: [(address: destination, amountDuffs: paymentAmount)],
                network: network
            )
        ) { error in
            guard let walletError = error as? PlatformWalletError else {
                XCTFail("build against a fully reserved wallet threw \(error)")
                return
            }
            switch walletError {
            case .coreInsufficientFunds, .noSelectableInputs:
                break
            default:
                XCTFail("build against a fully reserved wallet threw \(walletError)")
            }
        }

        // The abandoned / merchant-nacked arm: returns the inputs to spendable.
        try alice.getPlatformWallet().releaseReservation(held)

        let rebuilt = try alice.getPlatformWallet().buildSignedPayment(
            recipients: [(address: destination, amountDuffs: paymentAmount)],
            network: network
        )
        XCTAssertNotEqual(
            rebuilt.reservationToken, 0,
            "the released UTXO must be selectable again"
        )
        XCTAssertNotEqual(
            rebuilt.reservationToken, held.reservationToken,
            "tokens are process-unique and never reused"
        )
        // Leave no outstanding reservation behind for tearDown's resetState.
        try alice.getPlatformWallet().releaseReservation(rebuilt)

        // A released token is as unusable as a consumed one.
        XCTAssertThrowsError(
            try alice.getPlatformWallet().broadcastSigned(token: held.reservationToken)
        ) { error in
            guard case PlatformWalletError.reservationTokenConsumed = error else {
                XCTFail("broadcasting a released token must throw .reservationTokenConsumed, got \(error)")
                return
            }
        }
    }
}
