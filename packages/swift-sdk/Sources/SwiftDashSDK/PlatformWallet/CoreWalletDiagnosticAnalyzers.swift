import Foundation

/// Value-only analyzers shared by the diagnostic logger and its unit tests.
/// Keeping comparison and truncation here makes the tests exercise the exact
/// decisions that produce `swift/run.log`, without requiring a live Rust
/// wallet handle.
enum CoreWalletDiagnosticAnalyzer {
    struct TxoDiffDetail: Sendable {
        let outpoint: Data
        let reason: String
        let row: CoreWalletDatabaseDiagnosticSnapshot.Txo
    }

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

    struct AssetLockDiffDetail: Sendable {
        let outpointDisplay: String
        let reason: String
    }

    struct AssetLockDiff: Sendable {
        let details: [AssetLockDiffDetail]
        let emittedDetails: [AssetLockDiffDetail]
        let truncatedCount: Int
    }

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

    struct RestoreCandidate: Sendable {
        enum RejectionReason: String, Sendable {
            case missingAccount = "missing_account"
            case invalidTxid = "invalid_txid"
            case invalidAccountType = "invalid_account_type"
        }

        let txo: CoreWalletDatabaseDiagnosticSnapshot.Txo
        let accountType: UInt32?
        let standardTag: UInt8?
        let rejectionReason: RejectionReason?
        let isCoinbase: Bool
        let isConfirmed: Bool
        let isInstantLocked: Bool
    }

    struct RestoreBufferSummary: Sendable {
        let candidateCount: Int
        let candidateValueDuffs: UInt64
        let candidateBip44Count: Int
        let candidateBip44ValueDuffs: UInt64
        let candidateCoinJoinCount: Int
        let candidateCoinJoinValueDuffs: UInt64
        let builtCount: Int
        let emittedCandidates: [RestoreCandidate]
        let emittedValueDuffs: UInt64
        let emittedBip44Count: Int
        let emittedBip44ValueDuffs: UInt64
        let emittedCoinJoinCount: Int
        let emittedCoinJoinValueDuffs: UInt64
        let missingAccountCount: Int
        let invalidTxidCount: Int
        let invalidAccountTypeCount: Int
    }

    static func summarizeRestoreBuffer(
        candidates: [RestoreCandidate],
        emittedCount: Int,
        errored: Bool
    ) -> RestoreBufferSummary {
        let valid = candidates.filter { $0.rejectionReason == nil }
        let emittedCandidates = errored ? [] : Array(valid.prefix(max(0, emittedCount)))
        let candidateBip44 = candidates.filter {
            $0.accountType == 0 && $0.standardTag == 0
        }
        let candidateCoinJoin = candidates.filter { $0.accountType == 1 }
        let emittedBip44 = emittedCandidates.filter {
            $0.accountType == 0 && $0.standardTag == 0
        }
        let emittedCoinJoin = emittedCandidates.filter { $0.accountType == 1 }
        return RestoreBufferSummary(
            candidateCount: candidates.count,
            candidateValueDuffs: diagnosticSaturatingSum(candidates.map(\.txo.amount)),
            candidateBip44Count: candidateBip44.count,
            candidateBip44ValueDuffs: diagnosticSaturatingSum(
                candidateBip44.map(\.txo.amount)
            ),
            candidateCoinJoinCount: candidateCoinJoin.count,
            candidateCoinJoinValueDuffs: diagnosticSaturatingSum(
                candidateCoinJoin.map(\.txo.amount)
            ),
            builtCount: emittedCount,
            emittedCandidates: emittedCandidates,
            emittedValueDuffs: diagnosticSaturatingSum(emittedCandidates.map(\.txo.amount)),
            emittedBip44Count: emittedBip44.count,
            emittedBip44ValueDuffs: diagnosticSaturatingSum(emittedBip44.map(\.txo.amount)),
            emittedCoinJoinCount: emittedCoinJoin.count,
            emittedCoinJoinValueDuffs: diagnosticSaturatingSum(
                emittedCoinJoin.map(\.txo.amount)
            ),
            missingAccountCount: candidates.filter {
                $0.rejectionReason == .missingAccount
            }.count,
            invalidTxidCount: candidates.filter {
                $0.rejectionReason == .invalidTxid
            }.count,
            invalidAccountTypeCount: candidates.filter {
                $0.rejectionReason == .invalidAccountType
            }.count
        )
    }

    struct DatabaseTxoAuditRow: Sendable {
        let txo: CoreWalletDatabaseDiagnosticSnapshot.Txo
        let hasParentTransaction: Bool
        let walletIdMismatch: Bool
        let isSpent: Bool
        let hasSpendingTransaction: Bool
    }

    struct DatabaseTxoAnomaly: Sendable {
        let txo: CoreWalletDatabaseDiagnosticSnapshot.Txo
        let reason: String
    }

    struct DatabaseTxoAnomalyResult: Sendable {
        let details: [DatabaseTxoAnomaly]
        let emittedDetails: [DatabaseTxoAnomaly]
        let truncatedCount: Int

        func count(reason: String) -> Int {
            details.filter { $0.reason == reason }.count
        }
    }

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
            if !row.isSpent && row.hasSpendingTransaction {
                details.append(.init(
                    txo: row.txo,
                    reason: "unspent_with_spending_transaction"
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
        return .init(details: details, emittedDetails: emitted, truncatedCount: truncated)
    }

    struct ShieldedNote: Sendable {
        let value: UInt64
        let isSpent: Bool
    }

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
