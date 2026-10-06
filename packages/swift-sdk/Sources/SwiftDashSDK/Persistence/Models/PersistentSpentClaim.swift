import Foundation
import SwiftData

/// The engine's authoritative claim mirror; independent of history and TXO tombstones.
@Model
public final class PersistentSpentClaim {
    #Unique<PersistentSpentClaim>([\.walletId, \.outpoint])
    #Index<PersistentSpentClaim>([\.walletId])

    public var walletId: Data
    public var outpoint: Data
    public var claimant: Data?

    public init(walletId: Data, outpoint: Data, claimant: Data?) {
        self.walletId = walletId
        self.outpoint = outpoint
        self.claimant = claimant
    }
}
