// ShieldedBalanceReading.swift
// SwiftExampleApp
//
// Thin readers over `PlatformWalletManager.shieldedBalances`, the shielded
// balance state Rust pushes for each wallet. Screens read these instead of
// note rows. Whether a number is a known balance is decided by the SDK's
// `ordinaryBalance` / `accountBalance(_:)`; nil means not known.

import Foundation
import SwiftDashSDK

extension PlatformWalletManager {
    /// What the wallet holds in the shielded pool (its ordinary accounts),
    /// or `nil` while that is not a known balance.
    func ordinaryShieldedBalance(walletId: Data) -> UInt64? {
        shieldedBalances[walletId]?.ordinaryBalance
    }

    /// What one shielded account of the wallet can spend, or `nil` while
    /// that is not a known balance.
    func spendableShieldedCredits(walletId: Data, account: UInt32) -> UInt64? {
        shieldedBalances[walletId]?.accountBalance(account)
    }

    /// The total of the wallets' shielded balances.
    func networkShieldedTotal(walletIds: some Sequence<Data>) -> NetworkShieldedTotal {
        NetworkShieldedTotal(walletIds.map { ordinaryShieldedBalance(walletId: $0) })
    }
}

/// The sum of several wallets' Rust-computed shielded balances.
enum NetworkShieldedTotal: Equatable, Sendable {
    /// Every wallet's balance is known; their sum.
    case complete(UInt64)
    /// At least one wallet's balance is not known (an unbound wallet, a
    /// ledger not established yet, or no update yet); the sum of the known
    /// ones. The real total is at least this.
    case partial(UInt64)
    /// The known balances do not fit in `UInt64`.
    case overflow

    /// `nil` entries are wallets whose balance is not known.
    init(_ balances: some Sequence<UInt64?>) {
        var total: UInt64 = 0
        var complete = true
        for balance in balances {
            guard let balance else {
                complete = false
                continue
            }
            let (sum, overflow) = total.addingReportingOverflow(balance)
            guard !overflow else {
                self = .overflow
                return
            }
            total = sum
        }
        self = complete ? .complete(total) : .partial(total)
    }
}

/// Text for a lower bound ("At least X DASH") when part of a total is not
/// known. The figure is rounded DOWN to 8 fraction digits with exact
/// decimal arithmetic, so it never shows more than the known subtotal.
enum LowerBoundDashText {
    /// Platform and shielded credits per DASH.
    static let creditsPerDash = Decimal(100_000_000_000)
    /// Core duffs per DASH.
    static let duffsPerDash = Decimal(100_000_000)

    /// "At least X DASH" for an amount in credits.
    static func atLeast(credits: Decimal) -> String {
        "At least \(dash(credits / creditsPerDash)) DASH"
    }

    /// "At least X DASH" for Core duffs plus credits.
    static func atLeast(duffs: UInt64, credits: UInt64) -> String {
        atLeast(credits: Decimal(duffs) * (creditsPerDash / duffsPerDash) + Decimal(credits))
    }

    /// `amount` in DASH, rounded down to 8 fraction digits, with "," as
    /// the grouping separator and "." as the decimal separator.
    static func dash(_ amount: Decimal) -> String {
        var input = amount
        var rounded = Decimal()
        NSDecimalRound(&rounded, &input, 8, .down)
        let plain = NSDecimalNumber(decimal: rounded).stringValue
        let parts = plain.split(separator: ".", maxSplits: 1)
        var integerDigits = Array(parts.first.map(String.init) ?? "0")
        var grouped = ""
        while integerDigits.count > 3 {
            grouped = "," + String(integerDigits.suffix(3)) + grouped
            integerDigits.removeLast(3)
        }
        grouped = String(integerDigits) + grouped
        guard parts.count == 2 else { return grouped }
        return grouped + "." + parts[1]
    }
}
