import Foundation
import SwiftData

/// Raw consensus bytes for Rust's load-time SPV verification; storage grants no finality.
@Model
public final class PersistentCoreInstantLock {
    #Unique<PersistentCoreInstantLock>([\.networkRaw, \.walletId, \.txid])
    #Index<PersistentCoreInstantLock>([\.walletId])

    public var networkRaw: UInt32
    public var walletId: Data
    public var txid: Data
    public var consensusBytes: Data

    public init(networkRaw: UInt32, walletId: Data, txid: Data, consensusBytes: Data) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.txid = txid
        self.consensusBytes = consensusBytes
    }
}
