import Foundation

/// Value-only analyzers shared by the diagnostic logger and its unit tests.
/// Keeping comparison and truncation here makes the tests exercise the exact
/// decisions that produce `swift/run.log`, without requiring a live Rust
/// wallet handle.
enum CoreWalletDiagnosticAnalyzer {
    /// One deterministic mismatch record. `outpoint` is hashed by the logger
    /// and is never rendered directly.
    struct TxoDiffDetail: Sendable {
        let outpoint: Data
        let reason: String
        let row: CoreWalletDatabaseDiagnosticSnapshot.Txo
    }

    /// Aggregate DB↔Rust account/UTXO comparison plus bounded log details.
    struct TxoDiff: Sendable {
        let commonCount: Int
        let databaseAccountOnlyCount: Int
        let memoryAccountOnlyCount: Int
        let databaseOnlyCount: Int
        let memoryOnlyCount: Int
        let fieldMismatchCount: Int
        let details: [TxoDiffDetail]
        let emittedDetails: [TxoDiffDetail]
        let truncatedCount: Int
    }

    /// Compares UTXOs by outpoint and reports each differing field separately.
    /// Duplicate outpoints are resolved deterministically before comparison.
    static func compareTxos(
        database: [CoreWalletDatabaseDiagnosticSnapshot.Txo],
        memory: [CoreWalletDatabaseDiagnosticSnapshot.Txo],
        databaseAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>,
        memoryAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>
    ) -> TxoDiff {
        var databaseByOutpoint: [Data: CoreWalletDatabaseDiagnosticSnapshot.Txo] = [:]
        for row in database.sorted(by: txoOrder) where databaseByOutpoint[row.outpoint] == nil {
            databaseByOutpoint[row.outpoint] = row
        }
        var memoryByOutpoint: [Data: CoreWalletDatabaseDiagnosticSnapshot.Txo] = [:]
        for row in memory.sorted(by: txoOrder) where memoryByOutpoint[row.outpoint] == nil {
            memoryByOutpoint[row.outpoint] = row
        }

        let databaseOnly = databaseByOutpoint.keys
            .filter { memoryByOutpoint[$0] == nil }
            .sorted { $0.lexicographicallyPrecedes($1) }
        let memoryOnly = memoryByOutpoint.keys
            .filter { databaseByOutpoint[$0] == nil }
            .sorted { $0.lexicographicallyPrecedes($1) }

        var mismatchDetails: [TxoDiffDetail] = []
        for outpoint in databaseByOutpoint.keys.sorted(by: { $0.lexicographicallyPrecedes($1) }) {
            guard let databaseRow = databaseByOutpoint[outpoint],
                  let memoryRow = memoryByOutpoint[outpoint]
            else { continue }
            if databaseRow.amount != memoryRow.amount {
                mismatchDetails.append(.init(
                    outpoint: outpoint,
                    reason: "amount_mismatch",
                    row: databaseRow
                ))
            }
            if databaseRow.height != memoryRow.height {
                mismatchDetails.append(.init(
                    outpoint: outpoint,
                    reason: "height_mismatch",
                    row: databaseRow
                ))
            }
            if databaseRow.scriptPubKey != memoryRow.scriptPubKey {
                mismatchDetails.append(.init(
                    outpoint: outpoint,
                    reason: "script_mismatch",
                    row: databaseRow
                ))
            }
            if databaseRow.isLocked != memoryRow.isLocked {
                mismatchDetails.append(.init(
                    outpoint: outpoint,
                    reason: "lock_mismatch",
                    row: databaseRow
                ))
            }
            if databaseRow.account != memoryRow.account {
                mismatchDetails.append(.init(
                    outpoint: outpoint,
                    reason: "account_mismatch",
                    row: databaseRow
                ))
            }
        }

        var details = databaseOnly.compactMap { outpoint in
            databaseByOutpoint[outpoint].map {
                TxoDiffDetail(outpoint: outpoint, reason: "database_only", row: $0)
            }
        }
        details.append(contentsOf: memoryOnly.compactMap { outpoint in
            memoryByOutpoint[outpoint].map {
                TxoDiffDetail(outpoint: outpoint, reason: "memory_only", row: $0)
            }
        })
        details.append(contentsOf: mismatchDetails)
        details.sort(by: txoDetailOrder)
        let limited = limitedTxoDetails(details)

        return TxoDiff(
            commonCount: Set(databaseByOutpoint.keys).intersection(memoryByOutpoint.keys).count,
            databaseAccountOnlyCount: databaseAccounts.subtracting(memoryAccounts).count,
            memoryAccountOnlyCount: memoryAccounts.subtracting(databaseAccounts).count,
            databaseOnlyCount: databaseOnly.count,
            memoryOnlyCount: memoryOnly.count,
            fieldMismatchCount: mismatchDetails.count,
            details: details,
            emittedDetails: limited.emitted,
            truncatedCount: limited.truncated
        )
    }

    /// One AssetLock mismatch, retaining the display outpoint only so the
    /// logger can derive a stable reference from it.
    struct AssetLockDiffDetail: Sendable {
        let outpointDisplay: String
        let reason: String
    }

    /// Complete AssetLock mismatch set and its per-reason bounded projection.
    struct AssetLockDiff: Sendable {
        let details: [AssetLockDiffDetail]
        let emittedDetails: [AssetLockDiffDetail]
        let truncatedCount: Int
    }

    /// Compares persisted and Rust-tracked locks without exposing transaction
    /// or proof bytes.
    static func compareAssetLocks(
        database: [CoreWalletDatabaseDiagnosticSnapshot.AssetLock],
        memory: [CoreWalletDatabaseDiagnosticSnapshot.AssetLock]
    ) -> AssetLockDiff {
        var databaseByOutpoint: [String: CoreWalletDatabaseDiagnosticSnapshot.AssetLock] = [:]
        for row in database.sorted(by: assetLockOrder)
        where databaseByOutpoint[row.outpointDisplay] == nil {
            databaseByOutpoint[row.outpointDisplay] = row
        }
        var memoryByOutpoint: [String: CoreWalletDatabaseDiagnosticSnapshot.AssetLock] = [:]
        for row in memory.sorted(by: assetLockOrder)
        where memoryByOutpoint[row.outpointDisplay] == nil {
            memoryByOutpoint[row.outpointDisplay] = row
        }

        var details: [AssetLockDiffDetail] = []
        for outpoint in databaseByOutpoint.keys where memoryByOutpoint[outpoint] == nil {
            details.append(.init(outpointDisplay: outpoint, reason: "database_only"))
        }
        for outpoint in memoryByOutpoint.keys where databaseByOutpoint[outpoint] == nil {
            details.append(.init(outpointDisplay: outpoint, reason: "memory_only"))
        }
        for outpoint in databaseByOutpoint.keys.sorted() {
            guard let databaseRow = databaseByOutpoint[outpoint],
                  let memoryRow = memoryByOutpoint[outpoint]
            else { continue }
            if databaseRow.fundingType != memoryRow.fundingType {
                details.append(.init(outpointDisplay: outpoint, reason: "funding_type_mismatch"))
            }
            if databaseRow.status != memoryRow.status {
                details.append(.init(outpointDisplay: outpoint, reason: "status_mismatch"))
            }
            if databaseRow.accountIndex != memoryRow.accountIndex {
                details.append(.init(outpointDisplay: outpoint, reason: "account_index_mismatch"))
            }
            if databaseRow.registrationIndex != memoryRow.registrationIndex {
                details.append(.init(
                    outpointDisplay: outpoint,
                    reason: "registration_index_mismatch"
                ))
            }
            if databaseRow.amountDuffs != memoryRow.amountDuffs {
                details.append(.init(outpointDisplay: outpoint, reason: "amount_mismatch"))
            }
            if databaseRow.hasProof != memoryRow.hasProof {
                details.append(.init(
                    outpointDisplay: outpoint,
                    reason: "proof_presence_mismatch"
                ))
            }
        }
        details.sort(by: assetLockDetailOrder)
        let limited = limitedAssetLockDetails(details)
        return AssetLockDiff(
            details: details,
            emittedDetails: limited.emitted,
            truncatedCount: limited.truncated
        )
    }

    /// Lightweight description of a row considered by startup restore.
    struct RestoreCandidate: Sendable {
        enum RejectionReason: String, Sendable {
            case missingAccount = "missing_account"
            case invalidTxid = "invalid_txid"
            case invalidAccountType = "invalid_account_type"
        }

        /// Only the fields required by the restore summary. Keeping this
        /// intentionally small prevents the launch-time callback from doing
        /// any diagnostic outpoint/script materialization or fingerprinting.
        let amount: UInt64
        let accountType: UInt32?
        let standardTag: UInt8?
        let rejectionReason: RejectionReason?
    }

    /// Counts and values for candidates, rows actually emitted, and each
    /// validation rejection reason.
    struct RestoreBufferSummary: Sendable {
        let candidateCount: Int
        let candidateValueDuffs: UInt64
        let candidateBip44Count: Int
        let candidateBip44ValueDuffs: UInt64
        let candidateCoinJoinCount: Int
        let candidateCoinJoinValueDuffs: UInt64
        let builtCount: Int
        let emittedCount: Int
        let emittedValueDuffs: UInt64
        let emittedBip44Count: Int
        let emittedBip44ValueDuffs: UInt64
        let emittedCoinJoinCount: Int
        let emittedCoinJoinValueDuffs: UInt64
        let missingAccountCount: Int
        let invalidTxidCount: Int
        let invalidAccountTypeCount: Int
    }

    /// Reconciles the candidate list with the compact FFI buffer length. An
    /// errored build reports zero emitted rows even if validation failed late.
    ///
    /// Deliberately a single pass that accumulates into locals. This runs on
    /// the launch restore path while the persistence queue is held, so a large
    /// CoinJoin wallet must not pay for a dozen full-length `filter`/`map`
    /// allocations, and nothing here retains a per-row array.
    /// - Parameter candidateCountOverride: reported instead of the number of
    ///   candidates walked. For the errored path, where classifying each row
    ///   would fault a relationship per row to describe a load that is being
    ///   discarded: the caller passes the row count it already has and an
    ///   empty sequence, so `candidate_count` stays truthful and every other
    ///   counter is honestly zero.
    static func summarizeRestoreBuffer<S: Sequence>(
        candidates: S,
        emittedCount: Int,
        errored: Bool,
        candidateCountOverride: Int? = nil
    ) -> RestoreBufferSummary where S.Element == RestoreCandidate {
        // Rust is handed the first `emittedCount` rows that passed validation,
        // in order; an errored build deallocated the whole buffer, so none of
        // them reached it.
        let emissionLimit = errored ? 0 : max(0, emittedCount)

        var candidateCount = 0
        var candidateValue: UInt64 = 0
        var candidateBip44Count = 0
        var candidateBip44Value: UInt64 = 0
        var candidateCoinJoinCount = 0
        var candidateCoinJoinValue: UInt64 = 0
        var emitted = 0
        var emittedValue: UInt64 = 0
        var emittedBip44Count = 0
        var emittedBip44Value: UInt64 = 0
        var emittedCoinJoinCount = 0
        var emittedCoinJoinValue: UInt64 = 0
        var missingAccountCount = 0
        var invalidTxidCount = 0
        var invalidAccountTypeCount = 0
        var validSeen = 0

        for candidate in candidates {
            candidateCount += 1
            let isBip44 = candidate.accountType == 0 && candidate.standardTag == 0
            let isCoinJoin = candidate.accountType == 1
            candidateValue = diagnosticSaturatingAdd(candidateValue, candidate.amount)
            if isBip44 {
                candidateBip44Count += 1
                candidateBip44Value = diagnosticSaturatingAdd(
                    candidateBip44Value,
                    candidate.amount
                )
            }
            if isCoinJoin {
                candidateCoinJoinCount += 1
                candidateCoinJoinValue = diagnosticSaturatingAdd(
                    candidateCoinJoinValue,
                    candidate.amount
                )
            }

            switch candidate.rejectionReason {
            case .missingAccount:
                missingAccountCount += 1
            case .invalidTxid:
                invalidTxidCount += 1
            case .invalidAccountType:
                invalidAccountTypeCount += 1
            case nil:
                let emissionIndex = validSeen
                validSeen += 1
                guard emissionIndex < emissionLimit else { continue }
                emitted += 1
                emittedValue = diagnosticSaturatingAdd(emittedValue, candidate.amount)
                if isBip44 {
                    emittedBip44Count += 1
                    emittedBip44Value = diagnosticSaturatingAdd(
                        emittedBip44Value,
                        candidate.amount
                    )
                }
                if isCoinJoin {
                    emittedCoinJoinCount += 1
                    emittedCoinJoinValue = diagnosticSaturatingAdd(
                        emittedCoinJoinValue,
                        candidate.amount
                    )
                }
            }
        }

        return RestoreBufferSummary(
            candidateCount: candidateCountOverride ?? candidateCount,
            candidateValueDuffs: candidateValue,
            candidateBip44Count: candidateBip44Count,
            candidateBip44ValueDuffs: candidateBip44Value,
            candidateCoinJoinCount: candidateCoinJoinCount,
            candidateCoinJoinValueDuffs: candidateCoinJoinValue,
            builtCount: emittedCount,
            emittedCount: emitted,
            emittedValueDuffs: emittedValue,
            emittedBip44Count: emittedBip44Count,
            emittedBip44ValueDuffs: emittedBip44Value,
            emittedCoinJoinCount: emittedCoinJoinCount,
            emittedCoinJoinValueDuffs: emittedCoinJoinValue,
            missingAccountCount: missingAccountCount,
            invalidTxidCount: invalidTxidCount,
            invalidAccountTypeCount: invalidAccountTypeCount
        )
    }

    /// Persistent facts used to detect malformed or contradictory TXO rows.
    struct DatabaseTxoAuditRow: Sendable {
        let txo: CoreWalletDatabaseDiagnosticSnapshot.Txo
        let hasParentTransaction: Bool
        let walletIdMismatch: Bool
        let isSpent: Bool
        let hasSpendingTransaction: Bool
        /// Whether the linked spending transaction has reached a confirmed
        /// context. `nil` when there is no spending transaction to ask.
        /// Load-bearing: a linked-but-unconfirmed spender with `isSpent ==
        /// false` is the normal in-flight send, not an anomaly.
        let spendingTransactionIsInBlock: Bool?

        init(
            txo: CoreWalletDatabaseDiagnosticSnapshot.Txo,
            hasParentTransaction: Bool,
            walletIdMismatch: Bool,
            isSpent: Bool,
            hasSpendingTransaction: Bool,
            spendingTransactionIsInBlock: Bool? = nil
        ) {
            self.txo = txo
            self.hasParentTransaction = hasParentTransaction
            self.walletIdMismatch = walletIdMismatch
            self.isSpent = isSpent
            self.hasSpendingTransaction = hasSpendingTransaction
            self.spendingTransactionIsInBlock = spendingTransactionIsInBlock
        }
    }

    /// A database anomaly whose raw TXO identity is later hashed by the logger.
    struct DatabaseTxoAnomaly: Sendable {
        let txo: CoreWalletDatabaseDiagnosticSnapshot.Txo
        let reason: String
    }

    /// Complete database anomaly set and its per-reason bounded projection.
    struct DatabaseTxoAnomalyResult: Sendable {
        let details: [DatabaseTxoAnomaly]
        let emittedDetails: [DatabaseTxoAnomaly]
        let truncatedCount: Int
        /// Per-reason totals, computed once from the grouping the truncation
        /// already needed: the summary asks eight times, on the held queue.
        let countsByReason: [String: Int]

        func count(reason: String) -> Int {
            countsByReason[reason] ?? 0
        }
    }

    /// Derives all applicable anomaly reasons for every supplied row.
    static func databaseTxoAnomalies(
        _ rows: [DatabaseTxoAuditRow]
    ) -> DatabaseTxoAnomalyResult {
        var details: [DatabaseTxoAnomaly] = []
        for row in rows {
            if row.txo.account == nil {
                details.append(.init(txo: row.txo, reason: "missing_account"))
            }
            if !row.hasParentTransaction {
                details.append(.init(txo: row.txo, reason: "missing_parent_transaction"))
            }
            if row.walletIdMismatch {
                details.append(.init(txo: row.txo, reason: "wallet_id_mismatch"))
            }
            if row.isSpent && !row.hasSpendingTransaction {
                details.append(.init(
                    txo: row.txo,
                    reason: "spent_without_spending_transaction"
                ))
            }
            // `reconcileSpendObservation` deliberately links a mempool
            // spender while leaving `isSpent == false`, because a sighting
            // alone is reversible by RBF or eviction. Only a spender that has
            // landed in a block contradicts an unspent row; flagging the
            // mempool case would put one warning per output on every wallet
            // with an unconfirmed outgoing transaction and bury the real
            // anomalies this export exists to surface.
            if !row.isSpent && row.spendingTransactionIsInBlock == true {
                details.append(.init(
                    txo: row.txo,
                    reason: "unspent_with_confirmed_spending_transaction"
                ))
            }
            if row.txo.outpoint.count != 36 {
                details.append(.init(txo: row.txo, reason: "invalid_outpoint_length"))
            }
            if row.txo.scriptPubKey.isEmpty {
                details.append(.init(txo: row.txo, reason: "empty_script_pubkey"))
            }
        }
        details.sort {
            if $0.reason != $1.reason { return $0.reason < $1.reason }
            return $0.txo.outpoint.lexicographicallyPrecedes($1.txo.outpoint)
        }
        let grouped = Dictionary(grouping: details, by: \.reason)
        var emitted: [DatabaseTxoAnomaly] = []
        var truncated = 0
        for reason in grouped.keys.sorted() {
            let rows = grouped[reason] ?? []
            emitted.append(contentsOf: rows.prefix(CoreDiagnosticConstants.detailLimit))
            truncated += max(0, rows.count - CoreDiagnosticConstants.detailLimit)
        }
        return .init(
            details: details,
            emittedDetails: emitted,
            truncatedCount: truncated,
            countsByReason: grouped.mapValues(\.count)
        )
    }

    /// Value and spent state of a shielded note; identifiers are unnecessary
    /// for the aggregate store diagnostic.
    struct ShieldedNote: Sendable {
        let value: UInt64
        let isSpent: Bool
    }

    /// Aggregate shielded persistence state used by the exported snapshot.
    struct ShieldedStoreSummary: Sendable {
        let noteCount: Int
        let spentNoteCount: Int
        let spentValueCredits: UInt64
        let unspentNoteCount: Int
        let unspentValueCredits: UInt64
        let outgoingNoteCount: Int
        let activityCount: Int
        let activityPendingCount: Int
        let activityFailedCount: Int
        let viewingKeyCount: Int
        let subwalletSyncStateCount: Int
        let maximumSyncWatermark: UInt64
    }

    /// Aggregates shielded note values, activity state, keys, and watermarks
    /// without retaining any note or viewing-key identifiers.
    static func summarizeShieldedStore(
        notes: [ShieldedNote],
        outgoingNoteCount: Int,
        activityStatuses: [Int],
        viewingKeyCount: Int,
        syncWatermarks: [UInt64]
    ) -> ShieldedStoreSummary {
        let spent = notes.filter(\.isSpent)
        let unspent = notes.filter { !$0.isSpent }
        return ShieldedStoreSummary(
            noteCount: notes.count,
            spentNoteCount: spent.count,
            spentValueCredits: diagnosticSaturatingSum(spent.map(\.value)),
            unspentNoteCount: unspent.count,
            unspentValueCredits: diagnosticSaturatingSum(unspent.map(\.value)),
            outgoingNoteCount: outgoingNoteCount,
            activityCount: activityStatuses.count,
            activityPendingCount: activityStatuses.filter { $0 == 0 }.count,
            activityFailedCount: activityStatuses.filter { $0 == 2 }.count,
            viewingKeyCount: viewingKeyCount,
            subwalletSyncStateCount: syncWatermarks.count,
            maximumSyncWatermark: syncWatermarks.max() ?? 0
        )
    }

    private static func limitedTxoDetails(
        _ details: [TxoDiffDetail]
    ) -> (emitted: [TxoDiffDetail], truncated: Int) {
        let grouped = Dictionary(grouping: details, by: \.reason)
        var emitted: [TxoDiffDetail] = []
        var truncated = 0
        for reason in grouped.keys.sorted() {
            let rows = (grouped[reason] ?? []).sorted(by: txoDetailOrder)
            emitted.append(contentsOf: rows.prefix(CoreDiagnosticConstants.detailLimit))
            truncated += max(0, rows.count - CoreDiagnosticConstants.detailLimit)
        }
        return (emitted, truncated)
    }

    private static func limitedAssetLockDetails(
        _ details: [AssetLockDiffDetail]
    ) -> (emitted: [AssetLockDiffDetail], truncated: Int) {
        let grouped = Dictionary(grouping: details, by: \.reason)
        var emitted: [AssetLockDiffDetail] = []
        var truncated = 0
        for reason in grouped.keys.sorted() {
            let rows = (grouped[reason] ?? []).sorted(by: assetLockDetailOrder)
            emitted.append(contentsOf: rows.prefix(CoreDiagnosticConstants.detailLimit))
            truncated += max(0, rows.count - CoreDiagnosticConstants.detailLimit)
        }
        return (emitted, truncated)
    }

    private static func txoOrder(
        _ lhs: CoreWalletDatabaseDiagnosticSnapshot.Txo,
        _ rhs: CoreWalletDatabaseDiagnosticSnapshot.Txo
    ) -> Bool {
        if lhs.outpoint != rhs.outpoint {
            return lhs.outpoint.lexicographicallyPrecedes(rhs.outpoint)
        }
        if lhs.amount != rhs.amount { return lhs.amount < rhs.amount }
        if lhs.height != rhs.height { return lhs.height < rhs.height }
        if lhs.scriptPubKey != rhs.scriptPubKey {
            return lhs.scriptPubKey.lexicographicallyPrecedes(rhs.scriptPubKey)
        }
        if lhs.isLocked != rhs.isLocked { return !lhs.isLocked && rhs.isLocked }
        let lhsAccount = lhs.account?.referenceMaterial ?? Data()
        let rhsAccount = rhs.account?.referenceMaterial ?? Data()
        return lhsAccount.lexicographicallyPrecedes(rhsAccount)
    }

    private static func txoDetailOrder(_ lhs: TxoDiffDetail, _ rhs: TxoDiffDetail) -> Bool {
        if lhs.reason != rhs.reason { return lhs.reason < rhs.reason }
        return lhs.outpoint.lexicographicallyPrecedes(rhs.outpoint)
    }

    private static func assetLockOrder(
        _ lhs: CoreWalletDatabaseDiagnosticSnapshot.AssetLock,
        _ rhs: CoreWalletDatabaseDiagnosticSnapshot.AssetLock
    ) -> Bool {
        lhs.outpointDisplay < rhs.outpointDisplay
    }

    private static func assetLockDetailOrder(
        _ lhs: AssetLockDiffDetail,
        _ rhs: AssetLockDiffDetail
    ) -> Bool {
        if lhs.reason != rhs.reason { return lhs.reason < rhs.reason }
        return lhs.outpointDisplay < rhs.outpointDisplay
    }
}
