import Foundation
import SwiftData

/// Persistent storage for platform address sync state
///
/// This model stores the sync state for a specific account's platform addresses,
/// allowing sync to resume from where it left off between app sessions.
@Model
public final class PersistentPlatformSyncState {
    /// Unique identifier combining wallet ID and account info
    @Attribute(.unique) public var id: String

    /// The wallet identifier this sync state belongs to
    public var walletId: String

    /// Account category (e.g., "bip44", "identityRegistration")
    public var accountCategory: String

    /// Account index (if applicable)
    public var accountIndex: Int?

    /// Network identifier (e.g., "testnet", "mainnet")
    public var network: String

    /// Timestamp of the last full sync (seconds since Unix epoch)
    public var lastFullSyncTimestamp: Int64

    /// Platform block height at last checkpoint
    public var checkpointHeight: Int64

    /// Last terminal block processed
    public var lastTerminalBlock: Int64

    /// Highest address index found with a balance (-1 if none)
    public var highestFoundIndex: Int32

    /// Total balance found across all addresses (in credits)
    public var totalBalance: Int64

    /// Number of addresses with non-zero balance
    public var fundedAddressCount: Int32

    /// Timestamp when this record was last updated
    public var lastUpdated: Date

    public init(
        walletId: String,
        accountCategory: String,
        accountIndex: Int? = nil,
        network: String = "testnet"
    ) {
        // Create unique ID from wallet + account + network
        if let idx = accountIndex {
            self.id = "\(walletId)-\(accountCategory)-\(idx)-\(network)"
        } else {
            self.id = "\(walletId)-\(accountCategory)-\(network)"
        }
        self.walletId = walletId
        self.accountCategory = accountCategory
        self.accountIndex = accountIndex
        self.network = network
        self.lastFullSyncTimestamp = 0
        self.checkpointHeight = 0
        self.lastTerminalBlock = 0
        self.highestFoundIndex = -1
        self.totalBalance = 0
        self.fundedAddressCount = 0
        self.lastUpdated = Date()
    }

    /// Update from a PlatformSyncState
    public func update(from state: PlatformSyncState) {
        self.lastFullSyncTimestamp = Int64(state.lastFullSyncTimestamp)
        self.checkpointHeight = Int64(state.checkpointHeight)
        self.lastTerminalBlock = Int64(state.lastTerminalBlock)
        self.highestFoundIndex = state.highestFoundIndex.map { Int32($0) } ?? -1
        self.lastUpdated = Date()
    }

    /// Update from a PlatformSyncResult
    public func update(from result: PlatformSyncResult) {
        self.checkpointHeight = Int64(result.checkpointHeight)
        self.lastTerminalBlock = Int64(result.highestTerminalBlock)
        self.totalBalance = Int64(result.totalBalance)
        self.fundedAddressCount = Int32(result.fundedAddressCount)
        self.highestFoundIndex = result.highestFoundIndex.map { Int32($0) } ?? -1
        if result.fullSyncPerformed {
            self.lastFullSyncTimestamp = Int64(Date().timeIntervalSince1970)
        }
        self.lastUpdated = Date()
    }

    /// Convert to PlatformSyncState for use with FFI
    public func toPlatformSyncState() -> PlatformSyncState {
        return PlatformSyncState(
            lastFullSyncTimestamp: UInt64(max(0, lastFullSyncTimestamp)),
            checkpointHeight: UInt64(max(0, checkpointHeight)),
            lastTerminalBlock: UInt64(max(0, lastTerminalBlock)),
            highestFoundIndex: highestFoundIndex >= 0 ? UInt32(highestFoundIndex) : nil
        )
    }

    /// Check if a full sync is needed (more than 6 days 20 hours since last)
    public var needsFullSync: Bool {
        if lastFullSyncTimestamp == 0 {
            return true
        }
        let sixDays20Hours: Int64 = 6 * 24 * 60 * 60 + 20 * 60 * 60
        let now = Int64(Date().timeIntervalSince1970)
        return (now - lastFullSyncTimestamp) > sixDays20Hours
    }

    /// Format the total balance as DASH
    public var formattedBalance: String {
        // Credits to DASH: 1 DASH = 100,000,000,000 credits (10^11)
        let dash = Double(totalBalance) / 100_000_000_000.0
        return String(format: "%.8f DASH", dash)
    }

    /// Format the last sync time
    public var formattedLastSync: String {
        if lastFullSyncTimestamp == 0 {
            return "Never"
        }
        let date = Date(timeIntervalSince1970: TimeInterval(lastFullSyncTimestamp))
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}
