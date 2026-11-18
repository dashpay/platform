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
