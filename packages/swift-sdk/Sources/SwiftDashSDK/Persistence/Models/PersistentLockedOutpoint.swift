import Foundation
import SwiftData

/// An outpoint a wallet keeps out of coin selection: the collateral of a
/// masternode registration the wallet processed (spending it would end the
/// registration), or an outpoint locked by hand. The row is the lock, and
/// deleting it unlocks the outpoint.
///
/// A model of its own rather than a flag on `PersistentTxo`: a lock can
/// exist before its coin does and outlives the coin's spend.
@Model
public final class PersistentLockedOutpoint {
    #Unique<PersistentLockedOutpoint>([\.networkRaw, \.walletId, \.outpoint])
    public var networkRaw: UInt32
    public var walletId: Data
    /// 36 bytes, as `PersistentTxo.outpoint`: the txid's raw bytes, then the
    /// vout little-endian.
    public var outpoint: Data

    public init(networkRaw: UInt32, walletId: Data, outpoint: Data) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.outpoint = outpoint
    }
}
