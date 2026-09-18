import Foundation
import SwiftData

/// Freshness of the identity balance, committed atomically with its identity row.
/// A separate entity preserves every released identity model's schema hash.
@Model
public final class PersistentIdentityBalanceMetadata {
    #Unique<PersistentIdentityBalanceMetadata>([\.networkRaw, \.walletId, \.identityId])
    public var networkRaw: UInt32
    public var walletId: Data
    public var identityId: Data
    // Bit patterns preserve the full unsigned FFI domain in SQLite integers.
    public var platformHeight: Int64
    public var coreHeight: UInt32
    public var timestampMillis: Int64

    public init(networkRaw: UInt32, walletId: Data, identityId: Data,
                platformHeight: UInt64, coreHeight: UInt32, timestampMillis: UInt64) {
        self.networkRaw = networkRaw
        self.walletId = walletId
        self.identityId = identityId
        self.platformHeight = Int64(bitPattern: platformHeight)
        self.coreHeight = coreHeight
        self.timestampMillis = Int64(bitPattern: timestampMillis)
    }
}
