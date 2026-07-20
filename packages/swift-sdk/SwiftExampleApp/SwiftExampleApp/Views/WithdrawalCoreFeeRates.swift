// WithdrawalCoreFeeRates.swift
// SwiftExampleApp
//
// Pure, testable source of truth for the Core L1 fee rates the protocol
// accepts on an address credit withdrawal. Kept out of the SwiftUI view
// (mirroring `KeyDisableGate`) so the offered set can be unit-tested.
//
// DPP's `AddressCreditWithdrawalTransitionV0::validate_structure` rejects
// any `core_fee_per_byte` that is not a NON-ZERO Fibonacci number
// (`is_non_zero_fibonacci_number`). Non-Fibonacci rates (4, 6, 7, 9, 10,
// 100, …) deterministically fail structure validation on submit, so the
// withdraw sheet must only offer Fibonacci rates. We generate the same
// sequence the validator recognizes — 1, 2, 3, 5, 8, … — by a Fibonacci
// walk (rather than hardcoding) so the offered values stay in lockstep
// with the protocol's definition, capped at an app-side ceiling.

import Foundation

enum WithdrawalCoreFeeRates {
    /// Non-zero Fibonacci fee rates up to and including `ceiling`
    /// (deduplicated; the protocol accepts 1, and 0 is rejected).
    ///
    /// `addingReportingOverflow` guards the walk so an unusually large
    /// ceiling can never wrap `UInt32`.
    static func rates(upTo ceiling: UInt32) -> [UInt32] {
        guard ceiling >= 1 else { return [] }
        var rates: [UInt32] = []
        var previous: UInt32 = 1
        var current: UInt32 = 1
        while previous <= ceiling {
            if rates.last != previous {
                rates.append(previous)
            }
            let (next, overflow) = previous.addingReportingOverflow(current)
            previous = current
            current = next
            if overflow { break }
        }
        return rates
    }
}
