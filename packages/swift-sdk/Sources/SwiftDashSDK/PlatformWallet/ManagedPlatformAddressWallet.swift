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
        /// Highest derivation index found.
        public let highestFoundIndex: UInt32?
        /// Block height at tree snapshot.
        public let checkpointHeight: UInt64
        /// New sync height for next incremental sync.
        public let newSyncHeight: UInt64
        /// New sync timestamp for next call.
        public let newSyncTimestamp: UInt64
        /// What the sync engine did internally.
        public let metrics: SyncMetrics
    }

    /// Sync platform address balances across all accounts.
    ///
    /// Uses privacy-preserving trunk/branch queries with automatic incremental
    /// catch-up. The provider retains sync state between calls.
    ///
    /// - Returns: Array of per-account sync results.
    public func syncBalances() throws -> [SyncResult] {
        var resultsArray = AddressSyncResultArrayFFI(results: nil, count: 0)
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_sync_balances(
            handle, false, nil, &resultsArray, &error
        )

        defer {
            platform_address_wallet_free_sync_result_array(&resultsArray)
        }

        guard result == Success else {
            throw PlatformWalletError(result: result, error: error)
        }

        guard let results = resultsArray.results, resultsArray.count > 0 else {
            return []
        }

        return (0..<resultsArray.count).map { i in
            let r = results[i]
            let m = r.metrics
            return SyncResult(
                foundCount: r.found_count,
                absentCount: r.absent_count,
                highestFoundIndex: r.has_highest_found_index ? r.highest_found_index : nil,
                checkpointHeight: r.checkpoint_height,
                newSyncHeight: r.new_sync_height,
                newSyncTimestamp: r.new_sync_timestamp,
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

    /// Sync platform address balances for a single account.
    public func syncBalances(accountIndex: UInt32) throws -> SyncResult {
        var syncResult = AddressSyncResultFFI(
            found: nil, found_count: 0,
            absent: nil, absent_count: 0,
            highest_found_index: 0, has_highest_found_index: false,
            checkpoint_height: 0, new_sync_height: 0, new_sync_timestamp: 0,
            last_known_recent_block: 0,
            metrics: AddressSyncMetricsFFI(
                trunk_queries: 0, branch_queries: 0, total_elements_seen: 0,
                total_proof_bytes: 0, iterations: 0, compacted_queries: 0,
                recent_queries: 0, recent_entries_returned: 0, compacted_entries_returned: 0
            )
        )
        var error = PlatformWalletFFIError()

        let result = platform_address_wallet_sync_balances_on_account(
            handle, accountIndex, false, nil, &syncResult, &error
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
            highestFoundIndex: syncResult.has_highest_found_index ? syncResult.highest_found_index : nil,
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
