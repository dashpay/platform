//
//  WithdrawalCoreFeeRatesTests.swift
//  SwiftExampleAppTests
//
//  Unit coverage for `WithdrawalCoreFeeRates.rates(upTo:)` — the set of
//  Core L1 fee rates the ADDR-04 withdraw sheet offers. The protocol
//  (DPP `AddressCreditWithdrawalTransitionV0::validate_structure`) only
//  accepts NON-ZERO Fibonacci rates, so this set must contain exactly the
//  Fibonacci numbers within the app-side ceiling and nothing else.
//

import XCTest
@testable import SwiftExampleApp

final class WithdrawalCoreFeeRatesTests: XCTestCase {

    /// The Fibonacci numbers <= 10_000 — the ceiling the withdraw sheet
    /// uses. Matches the validator's accepted set (sibling DPP test
    /// `should_accept_valid_fibonacci_core_fees` accepts [1,2,3,5,8,13,21]).
    func testRatesUpTo10000AreExactlyTheFibonacciNumbers() {
        let expected: [UInt32] = [
            1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987,
            1597, 2584, 4181, 6765,
        ]
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 10_000), expected)
    }

    /// 1 is the default and must be present and first.
    func testDefaultRateOneIsOfferedFirst() {
        let rates = WithdrawalCoreFeeRates.rates(upTo: 10_000)
        XCTAssertEqual(rates.first, 1)
    }

    /// The leading repeated 1 in the Fibonacci sequence is de-duplicated.
    func testNoDuplicateLeadingOne() {
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 10_000).filter { $0 == 1 }.count, 1)
    }

    /// Known non-Fibonacci values the reviewer called out are never offered.
    func testNonFibonacciRatesAreNotOffered() {
        let rates = Set(WithdrawalCoreFeeRates.rates(upTo: 10_000))
        for invalid: UInt32 in [4, 6, 7, 9, 10, 11, 12, 14, 100, 1000] {
            XCTAssertFalse(rates.contains(invalid), "\(invalid) is not Fibonacci and must not be offered")
        }
    }

    /// The ceiling itself is inclusive when it is a Fibonacci number, and
    /// nothing above the ceiling leaks in.
    func testCeilingIsInclusiveAndBounded() {
        // 13 is Fibonacci -> included; 14 is the boundary for a 13 ceiling.
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 13).last, 13)
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 14).last, 13)
        XCTAssertTrue(WithdrawalCoreFeeRates.rates(upTo: 10_000).allSatisfy { $0 <= 10_000 })
    }

    /// Degenerate ceilings: 0 yields an empty set; 1 yields exactly [1].
    func testLowCeilings() {
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 0), [])
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 1), [1])
        XCTAssertEqual(WithdrawalCoreFeeRates.rates(upTo: 2), [1, 2])
    }
}
