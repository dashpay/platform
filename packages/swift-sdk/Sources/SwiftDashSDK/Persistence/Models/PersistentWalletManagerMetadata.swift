import Foundation
import SwiftData

/// SwiftData model for persisting wallet manager-level metadata.
///
/// Stores the combined sync height across all wallets and other
/// manager-wide state. Singleton per network.
@Model
public final class PersistentWalletManagerMetadata {
    /// Network this metadata row belongs to (unique per network).
    ///
    /// Stored as the `Network.rawValue` `UInt32` rather than the
    /// enum itself because SwiftData refuses `@Attribute(.unique)`
    /// on non-primitive types (Codable-wrapped raw-value enums are
    /// not "valid for unique constraints" per Core Data's rule) —
    /// the app crashes at container init otherwise. The `network`
    /// computed accessor below keeps the public API type-safe; only
    /// predicates that need to filter by network have to reach for
    /// `networkRaw`.
    @Attribute(.unique) public var networkRaw: UInt32
    /// Combined sync height across all wallets.
    public var combinedSyncHeight: UInt32
    /// Combined sync block hash (32 bytes).
    public var combinedSyncBlockHash: Data?
    /// Number of wallets managed.
    public var walletCount: Int
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Type-safe accessor over `networkRaw`. Reads fall back to
    /// `.testnet` if the stored raw value ever drifts out of the
    /// `Network` range (shouldn't happen — writers only go
    /// through this setter which uses `Network.rawValue`).
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    public init(network: Network) {
        self.networkRaw = network.rawValue
        self.combinedSyncHeight = 0
        self.walletCount = 0
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}
