// PlatformPaymentAccountSelection.swift
// SwiftExampleApp
//
// Pure, testable source of truth for choosing WHICH DIP-17 Platform
// Payment account funds a platform → platform transfer. Kept out of the
// SwiftUI view (mirroring `WithdrawalCoreFeeRates`) so the selection
// logic can be unit-tested without a SwiftData model container.
//
// Why this matters: the Rust `platform-wallet` Auto selector picks the
// transfer's inputs WITHIN a single chosen `account_index` (resolved via
// `platform_payment_managed_account_at_index`, key class 0); it does NOT
// span accounts. The send screen, however, gates its Send button on the
// AGGREGATE platform balance. With multiple key-class-0 Platform Payment
// accounts, naively handing Rust "the first account with any balance"
// can pick an account that can't cover the amount + fee while a sibling
// account could — Rust then rejects a send the UI enabled. This helper
// picks an account whose own balance covers amount + fee, so the chosen
// index matches what Rust will actually be able to spend.

import Foundation

enum PlatformPaymentAccountSelection {
    /// One candidate funding account: its DIP-17 `accountIndex` and the
    /// total credit balance held across that account's key-class-0
    /// addresses (the only addresses Rust will spend for this index).
    struct Candidate {
        let accountIndex: UInt32
        let balance: UInt64
    }

    /// Outcome of choosing a funding account.
    enum Outcome: Equatable {
        /// An account whose own balance covers amount + fee was chosen.
        case covering(accountIndex: UInt32)
        /// No single account covers amount + fee; the largest-balance
        /// account is offered as a best-effort fallback (Rust will return
        /// a typed insufficient-balance error if it truly can't cover it).
        /// `nil` when there are no candidate accounts at all.
        case insufficient(largestAccountIndex: UInt32?)
    }

    /// Choose the funding account for a transfer of `amount` credits with
    /// an estimated `fee` (both in platform credits).
    ///
    /// Selection rule:
    /// - Among accounts whose OWN balance is `>= amount + fee`, prefer the
    ///   one with the largest balance (deterministic tie-break on the
    ///   smaller `accountIndex`), and return `.covering`.
    /// - If none covers it, return `.insufficient` carrying the
    ///   largest-balance account index (or `nil` if there are no
    ///   candidates), so the caller can decide whether to surface an
    ///   error or proceed best-effort.
    ///
    /// `amount + fee` is summed with `addingReportingOverflow`; an
    /// overflowing requirement is treated as "no account can cover it"
    /// (`.insufficient`) rather than trapping/wrapping.
    static func choose(
        from candidates: [Candidate],
        amount: UInt64,
        fee: UInt64
    ) -> Outcome {
        // Largest-balance account overall (tie-break on smaller index) —
        // used both as the covering pick's preference order and as the
        // insufficient-case fallback.
        let largest = candidates.max {
            ($0.balance, $1.accountIndex) < ($1.balance, $0.accountIndex)
        }

        let (required, overflow) = amount.addingReportingOverflow(fee)
        if overflow {
            return .insufficient(largestAccountIndex: largest?.accountIndex)
        }

        // Largest covering account (same tie-break: larger balance, then
        // smaller index).
        let covering = candidates
            .filter { $0.balance >= required }
            .max { ($0.balance, $1.accountIndex) < ($1.balance, $0.accountIndex) }

        if let covering {
            return .covering(accountIndex: covering.accountIndex)
        }
        return .insufficient(largestAccountIndex: largest?.accountIndex)
    }
}
