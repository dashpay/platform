import Foundation
import SwiftData

/// SwiftData model for persisting wallet manager-level metadata.
///
/// Stores the combined sync height across all wallets and other
/// manager-wide state. Singleton per network.
@Model
public final class PersistentWalletManagerMetadata {
    /// Network name (unique per network).
    @Attribute(.unique) public var network: String
    /// Combined sync height across all wallets.
    public var combinedSyncHeight: UInt32
    /// Combined sync block hash (32 bytes).
    public var combinedSyncBlockHash: Data?
    /// Number of wallets managed.
    public var walletCount: Int
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    public init(network: String) {
        self.network = network
        self.combinedSyncHeight = 0
        self.walletCount = 0
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}
