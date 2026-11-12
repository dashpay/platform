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

    /// Human-readable outpoint (`<txid hex>:<vout>`), the txid in display
    /// order as `PersistentTxo.outpointHex`. A key that is not 36 bytes
    /// shows as its plain hex.
    public var outpointHex: String {
        guard outpoint.count == 36 else {
            return outpoint.map { String(format: "%02x", $0) }.joined()
        }
        let txidHex = outpoint.prefix(32).reversed().map { String(format: "%02x", $0) }.joined()
        let vout = outpoint.suffix(4).reversed().reduce(UInt32(0)) { $0 << 8 | UInt32($1) }
        return "\(txidHex):\(vout)"
    }
}
