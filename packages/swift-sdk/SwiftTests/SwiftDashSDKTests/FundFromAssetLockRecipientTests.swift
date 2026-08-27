import XCTest

@testable import SwiftDashSDK

/// Pins the two host-side decisions in the asset-lock funding surface:
/// which recipient pays the fee, and which recipient lists are legal.
///
/// The fee-strategy step (`ReduceOutput(index)`) is POSITIONAL, and
/// consensus resolves that position against the transition's outputs
/// map — a Rust `BTreeMap<PlatformAddress, Option<Credits>>` keyed by
/// `PlatformAddress`'s derived `Ord` (P2PKH before P2SH, then hash
/// bytes ascending). The caller's array order has no bearing on it.
/// `ManagedPlatformAddressWallet` therefore sorts recipients into that
/// canonical order before marshalling, so array position and consensus
/// output index are the same number.
///
/// The hazard being guarded: computing the index from an arbitrary
/// caller-supplied order silently re-targets the fee whenever the
/// remainder is not first lexicographically. With a third-party payee
/// in the set — the whole point of `fundFromAssetLockExternal` — that
/// means the payee's explicit amount absorbs the fee instead of the
/// sender's change.
final class FundFromAssetLockRecipientTests: XCTestCase {
    private typealias Recipient = ManagedPlatformAddressWallet.FundFromAssetLockRecipient

    private func recipient(_ tag: UInt8, credits: UInt64?, type: UInt8 = 0) -> Recipient {
        Recipient(
            addressType: type,
            hash: Data(repeating: tag, count: 20),
            credits: credits
        )
    }

    // MARK: - Canonical ordering / fee targeting

    func testRemainderIndexIsLexicographicNotCallerOrder() {
        // Caller lists payees first; Alice's change sorts FIRST.
        let alice = recipient(0x0A, credits: nil)
        let bob = recipient(0xBB, credits: 500)
        let carol = recipient(0xCC, credits: 700)

        let ordered = ManagedPlatformAddressWallet.canonicallyOrderedRecipients([bob, carol, alice])
        XCTAssertEqual(ordered.map(\.hash), [alice.hash, bob.hash, carol.hash])

        let index = ManagedPlatformAddressWallet.remainderStepIndex(in: ordered)
        XCTAssertEqual(
            index, 0,
            "the remainder's lexicographic position is 0, even though the caller listed it last"
        )
        XCTAssertNil(ordered[Int(index)].credits)
    }

    func testRemainderIndexWhenRemainderSortsLast() {
        // The inverse arrangement: caller lists the remainder first, but
        // it sorts last. A naive "position in the caller's array" answer
        // would be 0 and would charge the fee to Alice's payee output.
        let alice = recipient(0x0A, credits: 500)
        let bob = recipient(0xBB, credits: 700)
        let carol = recipient(0xCC, credits: nil)

        let ordered = ManagedPlatformAddressWallet.canonicallyOrderedRecipients([carol, alice, bob])
        let index = ManagedPlatformAddressWallet.remainderStepIndex(in: ordered)
        XCTAssertEqual(index, 2)
        XCTAssertNil(ordered[Int(index)].credits)
    }

    func testMarshalledFeeStrategyTargetsTheRemainderOutput() {
        let alice = recipient(0x0A, credits: nil)
        let bob = recipient(0xBB, credits: 500)
        let carol = recipient(0xCC, credits: 700)

        let request = ManagedPlatformAddressWallet.marshalFundingRequest([bob, carol, alice])

        XCTAssertEqual(request.feeStrategy.count, 1)
        XCTAssertEqual(request.feeStrategy[0].step_type, 1, "1 = ReduceOutput")

        let index = Int(request.feeStrategy[0].index)
        XCTAssertFalse(
            request.addresses[index].has_balance,
            "ReduceOutput must name the remainder output, not an explicit-amount payee"
        )
        // And the marshalled array is in the canonical order the Rust
        // BTreeMap will reproduce, which is what makes the index valid.
        let hashes = request.addresses.map { entry in
            withUnsafeBytes(of: entry.address.hash) { Data($0) }
        }
        XCTAssertEqual(hashes, [alice.hash, bob.hash, carol.hash])
    }

    func testCanonicalOrderPutsP2PKHBeforeP2SH() {
        // `PlatformAddress` is a Rust enum: the variant discriminant
        // orders before the payload. (P2SH is rejected downstream, but
        // the ordering rule is part of the contract being mirrored.)
        let p2sh = recipient(0x01, credits: 500, type: 1)
        let p2pkh = recipient(0xFF, credits: nil, type: 0)

        let ordered = ManagedPlatformAddressWallet.canonicallyOrderedRecipients([p2sh, p2pkh])
        XCTAssertEqual(ordered.map(\.addressType), [0, 1])
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
