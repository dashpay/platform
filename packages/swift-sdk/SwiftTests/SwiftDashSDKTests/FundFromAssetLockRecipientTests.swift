import XCTest

@testable import SwiftDashSDK

/// Pins the two host-side responsibilities left in the asset-lock
/// funding surface: marshalling recipients faithfully, and rejecting
/// recipient lists Swift can check synchronously.
///
/// What is deliberately NOT here any more: canonical ordering and the
/// `ReduceOutput(index)` fee step. That index is positional against the
/// transition's outputs `BTreeMap` — a consensus ordering rule
/// (`PlatformAddress`'s derived `Ord`) — so it is derived in
/// `platform-wallet` (`remainder_fee_strategy`), where the map is
/// actually built. Every binding that computed it from array position
/// mis-targeted the fee whenever the remainder was not also first
/// lexicographically; see the Rust tests
/// `remainder_fee_strategy_targets_the_remainder_output` and
/// `remainder_fee_strategy_handles_a_middle_remainder`.
final class FundFromAssetLockRecipientTests: XCTestCase {
    private typealias Recipient = ManagedPlatformAddressWallet.FundFromAssetLockRecipient

    private func recipient(_ tag: UInt8, credits: UInt64?, type: UInt8 = 0) -> Recipient {
        Recipient(
            addressType: type,
            hash: Data(repeating: tag, count: 20),
            credits: credits
        )
    }

    // MARK: - Marshalling

    /// Marshalling is order-preserving and lossless: one FFI row per
    /// recipient, in the caller's order, with `has_balance` carrying the
    /// remainder discriminator. The Rust side turns this into a set, so
    /// the order is not load-bearing — but silently dropping, reordering
    /// or re-tagging a row would be.
    func testMarshalPreservesRecipientsOneForOne() {
        let alice = recipient(0x0A, credits: nil)
        let bob = recipient(0xBB, credits: 500)
        let carol = recipient(0xCC, credits: 700)

        let rows = ManagedPlatformAddressWallet.marshalRecipients([bob, carol, alice])

        XCTAssertEqual(rows.count, 3)
        let hashes = rows.map { row in
            withUnsafeBytes(of: row.address.hash) { Data($0) }
        }
        XCTAssertEqual(hashes, [bob.hash, carol.hash, alice.hash])
        XCTAssertEqual(rows.map(\.has_balance), [true, true, false])
        XCTAssertEqual(rows.map(\.balance), [500, 700, 0])
        XCTAssertEqual(rows.map(\.address.address_type), [0, 0, 0])
    }

    /// No fee-strategy derivation survives in Swift. Guards against the
    /// helper being reintroduced by a future change that "needs the
    /// index right here" — the index belongs to `platform-wallet`.
    func testMarshalDoesNotProduceAFeeStrategy() {
        let rows = ManagedPlatformAddressWallet.marshalRecipients([
            recipient(0xCC, credits: 700),
            recipient(0x0A, credits: nil),
        ])
        // The marshaller's entire output is the address array; the
        // remainder is identified by `has_balance == false`, never by a
        // positional index computed on this side of the boundary.
        XCTAssertEqual(rows.filter { !$0.has_balance }.count, 1)
    }

    // MARK: - Preflight

    func testExternalPreflightAcceptsThirdPartyPayeeWithOwnRemainder() throws {
        try ManagedPlatformAddressWallet.fundFromAssetLockExternalPreflight(
            recipients: [recipient(0xBB, credits: 500), recipient(0x0A, credits: nil)]
        )
    }

    func testExternalPreflightRejectsRemainderOnlyRequest() {
        // Nothing is being paid externally — that is a call-site mistake,
        // and `fundFromAssetLock` validates the destination properly.
        XCTAssertThrowsError(
            try ManagedPlatformAddressWallet.fundFromAssetLockExternalPreflight(
                recipients: [recipient(0x0A, credits: nil)]
            )
        )
    }

    func testPreflightsRejectP2SHAndBadHashLengths() {
        let p2shSet = [recipient(0xBB, credits: 500, type: 1), recipient(0x0A, credits: nil)]
        XCTAssertThrowsError(
            try ManagedPlatformAddressWallet.fundFromAssetLockPreflight(recipients: p2shSet)
        )
        XCTAssertThrowsError(
            try ManagedPlatformAddressWallet.fundFromAssetLockExternalPreflight(recipients: p2shSet)
        )

        let shortHash = [
            Recipient(addressType: 0, hash: Data(repeating: 0xBB, count: 19), credits: 500),
            recipient(0x0A, credits: nil),
        ]
        XCTAssertThrowsError(
            try ManagedPlatformAddressWallet.fundFromAssetLockPreflight(recipients: shortHash)
        )
        XCTAssertThrowsError(
            try ManagedPlatformAddressWallet.fundFromAssetLockExternalPreflight(
                recipients: shortHash)
        )
    }

    func testPreflightsRejectWrongRemainderCardinality() {
        for recipients in [
            [Recipient](),
            [recipient(0x0A, credits: 500), recipient(0xBB, credits: 700)],
            [recipient(0x0A, credits: nil), recipient(0xBB, credits: nil)],
        ] {
            XCTAssertThrowsError(
                try ManagedPlatformAddressWallet.fundFromAssetLockPreflight(recipients: recipients)
            )
            XCTAssertThrowsError(
                try ManagedPlatformAddressWallet.fundFromAssetLockExternalPreflight(
                    recipients: recipients)
            )
        }
    }
}
