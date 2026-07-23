//
//  PlatformPaymentAccountSelectionTests.swift
//  SwiftExampleAppTests
//
//  Unit coverage for `PlatformPaymentAccountSelection.choose(...)` — the
//  pure logic that picks WHICH key-class-0 Platform Payment account funds
//  a platform → platform transfer. The Rust Auto selector spends inputs
//  within a single account, so the chosen account must cover the full
//  amount + fee on its own; the helper must never hand back an account
//  that can't, and must pick the largest covering account when several do.
//

import XCTest
@testable import SwiftExampleApp

final class PlatformPaymentAccountSelectionTests: XCTestCase {

    private typealias Selection = PlatformPaymentAccountSelection
    private typealias Candidate = PlatformPaymentAccountSelection.Candidate

    // MARK: - Covering picks

    func testSingleCoveringAccountIsChosen() {
        let candidates = [Candidate(accountIndex: 0, balance: 1_000)]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 500, fee: 100),
            .covering(accountIndex: 0)
        )
    }

    /// With several covering accounts, prefer the largest balance.
    func testPrefersLargestCoveringAccount() {
        let candidates = [
            Candidate(accountIndex: 0, balance: 1_000),
            Candidate(accountIndex: 1, balance: 5_000),
            Candidate(accountIndex: 2, balance: 2_000),
        ]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 800, fee: 100),
            .covering(accountIndex: 1)
        )
    }

    /// Equal-balance covering accounts tie-break on the smaller index, so
    /// the pick is deterministic regardless of input order.
    func testCoveringTieBreaksOnSmallerIndex() {
        let candidates = [
            Candidate(accountIndex: 3, balance: 1_000),
            Candidate(accountIndex: 1, balance: 1_000),
            Candidate(accountIndex: 2, balance: 1_000),
        ]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 500, fee: 100),
            .covering(accountIndex: 1)
        )
    }

    /// The account must cover amount + FEE, not just the amount: an
    /// account that holds the amount but not the fee is NOT covering.
    func testFeeIsIncludedInCoverageRequirement() {
        let candidates = [
            Candidate(accountIndex: 0, balance: 500), // exactly the amount
            Candidate(accountIndex: 1, balance: 650), // amount + fee
        ]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 500, fee: 100),
            .covering(accountIndex: 1)
        )
    }

    /// Exact coverage (balance == amount + fee) qualifies.
    func testExactCoverageQualifies() {
        let candidates = [Candidate(accountIndex: 7, balance: 600)]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 500, fee: 100),
            .covering(accountIndex: 7)
        )
    }

    // MARK: - Insufficient (the core CodeRabbit bug)

    /// The aggregate covers the transfer but NO single account does — this
    /// is exactly the case the UI's aggregate-balance gate let through
    /// before. The helper must report `.insufficient`, not pick one.
    func testAggregateCoversButNoSingleAccountDoes() {
        let candidates = [
            Candidate(accountIndex: 0, balance: 400),
            Candidate(accountIndex: 1, balance: 400),
        ] // aggregate 800 >= 600, but neither account alone does
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 500, fee: 100),
            .insufficient(largestAccountIndex: 0)
        )
    }

    /// Insufficient still surfaces the largest-balance account (tie-broken
    /// on smaller index) as a best-effort fallback for callers that opt to
    /// proceed; the send screen chooses to abort instead.
    func testInsufficientReportsLargestAccountFallback() {
        let candidates = [
            Candidate(accountIndex: 5, balance: 100),
            Candidate(accountIndex: 2, balance: 300),
            Candidate(accountIndex: 9, balance: 300),
        ]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 10_000, fee: 1),
            .insufficient(largestAccountIndex: 2)
        )
    }

    /// No candidate accounts at all → insufficient with no fallback index.
    func testNoCandidatesYieldsInsufficientNil() {
        XCTAssertEqual(
            Selection.choose(from: [], amount: 500, fee: 100),
            .insufficient(largestAccountIndex: nil)
        )
    }

    // MARK: - Overflow safety

    /// amount + fee overflowing UInt64 must be treated as "no account can
    /// cover it" (insufficient), never trap or wrap to a tiny requirement.
    func testAmountPlusFeeOverflowIsInsufficient() {
        let candidates = [Candidate(accountIndex: 0, balance: UInt64.max)]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: UInt64.max, fee: 1),
            .insufficient(largestAccountIndex: 0)
        )
    }

    /// A zero requirement (off-path fallback when amount/fee are absent)
    /// makes the largest account trivially covering.
    func testZeroRequirementPicksLargestAccount() {
        let candidates = [
            Candidate(accountIndex: 0, balance: 10),
            Candidate(accountIndex: 1, balance: 20),
        ]
        XCTAssertEqual(
            Selection.choose(from: candidates, amount: 0, fee: 0),
            .covering(accountIndex: 1)
        )
    }
}
