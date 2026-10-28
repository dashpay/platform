import Foundation
import SwiftData

/// One transaction's Core accounting for ONE wallet, exactly as
/// `rs-platform-wallet` emitted it (`TransactionAccountingFFI`, through
/// `on_persist_transaction_accounting_fn`).
///
/// A transaction shared by several local wallets has one row per wallet,
/// so each wallet shows its own side of it. A wallet with no row has no
/// accounting from Rust yet (a payload-only provider transaction, or a
/// stored row the load-time backfill has not reached). Pure storage:
/// nothing on the Swift side derives, corrects or vetoes these values.
@Model
public final class PersistentTransactionAccounting {
    #Unique<PersistentTransactionAccounting>([\.walletId, \.txid])
    #Index<PersistentTransactionAccounting>([\.walletId], [\.txid])

    /// The wallet this accounting belongs to (32 bytes).
    public var walletId: Data
    /// Transaction id, raw wire bytes like `PersistentTransaction.txid`.
    public var txid: Data
    /// Net Core amount in duffs for this wallet (positive = received).
    public var netAmount: Int64
    /// `CoreDirectionCode` wire value for this wallet.
    public var direction: UInt32
    /// Fee in duffs; `nil` unless this wallet funded every input.
    public var fee: UInt64?
    /// Inverse of `PersistentTransaction.walletAccounting`, which cascades.
    public var transaction: PersistentTransaction?
    public var lastUpdated: Date

    public init(
        walletId: Data,
        txid: Data,
        netAmount: Int64,
        direction: UInt32,
        fee: UInt64?,
        transaction: PersistentTransaction?
    ) {
        self.walletId = walletId
        self.txid = txid
        self.netAmount = netAmount
        self.direction = direction
        self.fee = fee
        self.transaction = transaction
        self.lastUpdated = Date()
    }
}
