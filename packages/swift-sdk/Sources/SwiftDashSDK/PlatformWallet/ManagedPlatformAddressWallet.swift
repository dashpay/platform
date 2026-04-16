import Foundation

/// Platform address wallet for BLAST sync, transfers, and withdrawals.
///
/// Obtained via `ManagedPlatformWallet.platformAddressWallet()`.
/// Replaces the manual `AddressSyncService` + `SDK.syncAddressBalances` flow
/// with a single integrated sync that handles incremental state automatically.
public class ManagedPlatformAddressWallet {
    let handle: Handle

    init(handle: Handle) {
        self.handle = handle
    }

    deinit {
        var error = PlatformWalletFFIError()
        _ = platform_address_wallet_destroy(handle, &error)
    }

    // MARK: - Balance queries

    /// Platform address with its credit balance.
    public struct AddressBalance {
        /// Address type (0 = P2PKH).
        public let addressType: UInt8
        /// 20-byte address hash.
        public let hash: Data
        /// Credit balance.
        public let balance: UInt64
    }

    /// Get total platform credits across all addresses.
    public func totalCredits() throws -> UInt64 {
        var credits: UInt64 = 0
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_total_credits(handle, &credits, &error)

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        return credits
    }

    /// Get all platform addresses with their cached balances.
    public func addressesWithBalances() throws -> [AddressBalance] {
        var entriesPtr: UnsafeMutablePointer<AddressBalanceEntryFFI>?
        var count: Int = 0
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_addresses_with_balances(
            handle, &entriesPtr, &count, &error
        )

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        defer {
            platform_address_wallet_free_address_balances(entriesPtr, count)
        }

        guard let entries = entriesPtr, count > 0 else {
            return []
        }

        return (0..<count).map { i in
            let entry = entries[i]
            let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
            return AddressBalance(
                addressType: entry.address.address_type,
                hash: hashData,
                balance: entry.balance
            )
        }
    }

    // MARK: - Sync State Restore

    /// Restore sync state from persisted values.
    ///
    /// Call after wallet creation and before the first sync to resume
    /// incremental mode instead of doing a full trunk/branch/compact rescan.
    public func restoreSyncState(
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) throws {
        var error = PlatformWalletFFIError()
        let result = platform_address_wallet_restore_sync_state(
            handle, syncHeight, syncTimestamp, lastKnownRecentBlock, &error
        )
        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }
    }

    // MARK: - Sync

    /// Metrics from a single sync round.
    public struct SyncMetrics {
        public let trunkQueries: UInt32
        public let branchQueries: UInt32
        public let totalElementsSeen: UInt32
        public let totalProofBytes: UInt32
        public let iterations: UInt32
        public let compactedQueries: UInt32
        public let recentQueries: UInt32
        public let recentEntriesReturned: UInt32
        public let compactedEntriesReturned: UInt32
    }

    /// Sync result for a single account.
    public struct SyncResult {
        /// Number of addresses found with balances.
        public let foundCount: Int
        /// Number of addresses proven absent.
        public let absentCount: Int
        /// Block height at tree snapshot.
        public let checkpointHeight: UInt64
        /// New sync height for next incremental sync.
        public let newSyncHeight: UInt64
        /// New sync timestamp for next call.
        public let newSyncTimestamp: UInt64
        /// What the sync engine did internally.
        public let metrics: SyncMetrics
    }

    /// Sync platform address balances across every platform payment
    /// account on the wallet in a single trunk/branch scan.
    ///
    /// The unified Rust-side provider presents pending addresses from
    /// all accounts at once, so one GroveDB proof covers everything.
    /// The wallet retains incremental sync state between calls.
    ///
    /// - Returns: A single combined sync result.
    public func syncBalances() throws -> SyncResult {
        var syncResult = AddressSyncResultFFI(
            found: nil, found_count: 0,
            absent: nil, absent_count: 0,
            checkpoint_height: 0, new_sync_height: 0, new_sync_timestamp: 0,
            last_known_recent_block: 0,
            metrics: AddressSyncMetricsFFI(
                trunk_queries: 0, branch_queries: 0, total_elements_seen: 0,
                total_proof_bytes: 0, iterations: 0, compacted_queries: 0,
                recent_queries: 0, recent_entries_returned: 0, compacted_entries_returned: 0
            )
        )
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_sync_balances(
            handle, false, nil, &syncResult, &error
        )

        defer {
            platform_address_wallet_free_sync_result(&syncResult)
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        let m = syncResult.metrics
        return SyncResult(
            foundCount: syncResult.found_count,
            absentCount: syncResult.absent_count,
            checkpointHeight: syncResult.checkpoint_height,
            newSyncHeight: syncResult.new_sync_height,
            newSyncTimestamp: syncResult.new_sync_timestamp,
            metrics: SyncMetrics(
                trunkQueries: m.trunk_queries,
                branchQueries: m.branch_queries,
                totalElementsSeen: m.total_elements_seen,
                totalProofBytes: m.total_proof_bytes,
                iterations: m.iterations,
                compactedQueries: m.compacted_queries,
                recentQueries: m.recent_queries,
                recentEntriesReturned: m.recent_entries_returned,
                compactedEntriesReturned: m.compacted_entries_returned
            )
        )
    }
}
