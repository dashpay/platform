import CryptoKit
import DashSDKFFI
import Foundation
import SwiftData

/// Labels the two Core diagnostic paths without accepting free-form strings.
/// Only ``preExport`` performs database and Rust-memory inspection;
/// ``restoreBuffer`` labels the lightweight summary built from rows that the
/// restore callback had to fetch and marshal anyway.
enum CoreWalletDiagnosticCheckpoint: String, Sendable {
    case restoreBuffer = "restore_buffer"
    case preExport = "pre_export"
}

/// Value-only copy of the SwiftData state used after the handler has released
/// its serial queue. No SwiftData model object crosses the queue boundary.
struct CoreWalletDatabaseDiagnosticSnapshot: Sendable {
    /// Canonical account tuple shared by SwiftData and Rust FFI snapshots.
    struct AccountKey: Hashable, Sendable {
        let typeTag: UInt32
        let standardTag: UInt8
        let index: UInt32
        let registrationIndex: UInt32
        let keyClass: UInt32
        let userIdentityId: Data
        let friendIdentityId: Data

        init(
            typeTag: UInt32,
            standardTag: UInt8,
            index: UInt32,
            registrationIndex: UInt32,
            keyClass: UInt32,
            userIdentityId: Data,
            friendIdentityId: Data
        ) {
            self.typeTag = typeTag
            self.standardTag = standardTag
            self.index = index
            self.registrationIndex = registrationIndex
            self.keyClass = keyClass
            self.userIdentityId = Self.ffiIdentityBytes(userIdentityId)
            self.friendIdentityId = Self.ffiIdentityBytes(friendIdentityId)
        }

        var referenceMaterial: Data {
            var data = Data()
            data.appendLittleEndian(typeTag)
            data.append(standardTag)
            data.appendLittleEndian(index)
            data.appendLittleEndian(registrationIndex)
            data.appendLittleEndian(keyClass)
            data.append(userIdentityId)
            data.append(friendIdentityId)
            return data
        }

        private static func ffiIdentityBytes(_ value: Data) -> Data {
            if value.count == 32 { return value }
            if value.count > 32 { return Data(value.prefix(32)) }
            var padded = Data(value)
            padded.append(Data(repeating: 0, count: 32 - value.count))
            return padded
        }
    }

    /// Minimal owned-output representation required for deterministic diffing.
    struct Txo: Sendable {
        let outpoint: Data
        let amount: UInt64
        let height: UInt32
        let scriptPubKey: Data
        let isLocked: Bool
        let account: AccountKey?
    }

    /// Comparable subset of one tracked AssetLock; no transaction or proof
    /// bytes cross the SwiftData queue boundary.
    struct AssetLock: Sendable {
        let outpointDisplay: String
        let fundingType: Int
        let status: Int
        let accountIndex: UInt32
        let registrationIndex: UInt32
        /// `nil` represents a corrupt negative value in the signed legacy
        /// SwiftData column; a valid in-memory `UInt64` can never equal it.
        let amountDuffs: UInt64?
        let hasProof: Bool
    }

    let walletId: Data
    let accounts: [AccountKey]
    let unspentTxos: [Txo]
    let assetLocks: [AssetLock]
    let assetLocksAvailable: Bool
}

enum CoreDiagnosticConstants {
    static let detailLimit = 25
}

private extension Data {
    mutating func appendLittleEndian<T: FixedWidthInteger>(_ value: T) {
        var littleEndian = value.littleEndian
        Swift.withUnsafeBytes(of: &littleEndian) { append(contentsOf: $0) }
    }
}

/// Adds diagnostic values without allowing corrupt data to trap the exporter.
func diagnosticSaturatingSum<S: Sequence>(_ values: S) -> UInt64
where S.Element == UInt64 {
    values.reduce(0) { partial, value in
        let (sum, overflow) = partial.addingReportingOverflow(value)
        return overflow ? UInt64.max : sum
    }
}

private func diagnosticSignedSaturatingSum<S: Sequence>(_ values: S) -> Int64
where S.Element == Int64 {
    values.reduce(0) { partial, value in
        let (sum, overflow) = partial.addingReportingOverflow(value)
        if !overflow { return sum }
        return value >= 0 ? Int64.max : Int64.min
    }
}

/// Hashes length-delimited canonical records after sorting, making the result
/// stable across SwiftData/Rust iteration order without logging raw records.
func diagnosticFingerprint(_ records: [Data]) -> Data {
    var hasher = SHA256()
    for record in records.sorted(by: { $0.lexicographicallyPrecedes($1) }) {
        var length = UInt64(record.count).littleEndian
        Swift.withUnsafeBytes(of: &length) { hasher.update(bufferPointer: $0) }
        hasher.update(data: record)
    }
    return Data(hasher.finalize())
}

/// Canonical binary representation of every TXO field compared by the
/// database↔memory analyzer. The caller hashes this value before logging it.
func diagnosticTxoFingerprint(
    outpoint: Data,
    amount: UInt64,
    height: UInt32,
    scriptPubKey: Data,
    isLocked: Bool,
    account: CoreWalletDatabaseDiagnosticSnapshot.AccountKey?
) -> Data {
    var data = Data()
    data.appendLittleEndian(UInt64(outpoint.count))
    data.append(outpoint)
    data.appendLittleEndian(amount)
    data.appendLittleEndian(height)
    data.append(isLocked ? 1 : 0)
    data.appendLittleEndian(UInt64(scriptPubKey.count))
    data.append(scriptPubKey)
    if let account {
        data.append(1)
        data.appendLittleEndian(UInt64(account.referenceMaterial.count))
        data.append(account.referenceMaterial)
    } else {
        data.append(0)
    }
    return data
}

extension PlatformWalletPersistenceHandler {
    /// Main-actor-friendly entry point used by manual log export. The handler's
    /// serial queue owns the ModelContext; only a Sendable value snapshot is
    /// resumed across the continuation.
    func emitCoreWalletDatabaseDiagnostics(
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) async -> CoreWalletDatabaseDiagnosticSnapshot? {
        await withCheckedContinuation { continuation in
            serialQueue.async { [self] in
                let snapshot = autoreleasepool { () -> CoreWalletDatabaseDiagnosticSnapshot? in
                    return emitCoreWalletDatabaseDiagnosticsOnQueue(
                        walletId: walletId,
                        checkpoint: checkpoint
                    )
                }
                continuation.resume(returning: snapshot)
            }
        }
    }

    /// Queue-confined implementation behind the async export API. Callers must
    /// already own `serialQueue`; it intentionally performs the full exact
    /// audit and returns only Sendable value copies.
    @discardableResult
    func emitCoreWalletDatabaseDiagnosticsOnQueue(
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) -> CoreWalletDatabaseDiagnosticSnapshot? {
        do {
            let walletDescriptor = FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletId: walletId)
            )
            guard let wallet = try backgroundContext.fetch(walletDescriptor).first else {
                SDKLogger.event(
                    "core_diagnostics_unavailable",
                    category: .persistence,
                    severity: .warning,
                    fields: [
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "reason": .publicText("wallet_not_found"),
                        "wallet_reference": .reference(walletId),
                    ]
                )
                return nil
            }

            // Exact #4438 classification needs a complete cross-wallet pass:
            // an output absent from this wallet may be `wrong_wallet`, not
            // `missing_txo`. This first export-only implementation materializes
            // that pass. A future bounded version must stream every row rather
            // than apply a fetch limit, so it preserves the distinction.
            let allTxos = try backgroundContext.fetch(FetchDescriptor<PersistentTxo>())
            let walletTxos = allTxos.filter {
                $0.walletId == walletId || Self.relationshipWalletId(of: $0) == walletId
            }
            // Walking every transaction relationship is deliberately export-only.
            // A heavily mixed wallet can have enough history for this traversal to
            // stall restore, which is precisely the failure this instrumentation is
            // intended to diagnose rather than reproduce.
            let allTransactions: [PersistentTransaction]?
            let walletTransactions: [PersistentTransaction]?
            if checkpoint == .preExport {
                do {
                    let fetched = try backgroundContext.fetch(
                        FetchDescriptor<PersistentTransaction>()
                    )
                    allTransactions = fetched
                    walletTransactions = fetched.filter {
                        Self.walletOwnsTransaction(walletId: walletId, transaction: $0)
                    }
                } catch {
                    allTransactions = nil
                    walletTransactions = nil
                    SDKLogger.event(
                        "core_owned_output_audit_summary",
                        category: .persistence,
                        severity: .warning,
                        fields: [
                            "audit_incomplete": .boolean(true),
                            "checkpoint": .publicText(checkpoint.rawValue),
                            "reason": .publicText("transaction_fetch_failed"),
                            "wallet_reference": .reference(walletId),
                        ]
                    )
                }
            } else {
                allTransactions = nil
                walletTransactions = nil
            }
            let pending: [PersistentPendingInput]?
            do {
                pending = try backgroundContext.fetch(
                    FetchDescriptor<PersistentPendingInput>(
                        predicate: #Predicate { $0.walletId == walletId }
                    )
                )
            } catch {
                pending = nil
            }

            let confirmed = walletTxos.filter(\.isConfirmed)
            let unconfirmed = walletTxos.filter { !$0.isConfirmed }
            let spent = walletTxos.filter(\.isSpent)
            let unspent = walletTxos.filter { !$0.isSpent }
            let locked = walletTxos.filter(\.isLocked)
            let txoFingerprint = diagnosticFingerprint(walletTxos.map {
                diagnosticTxoFingerprint(
                    outpoint: $0.outpoint,
                    amount: $0.amount,
                    height: $0.height,
                    scriptPubKey: $0.scriptPubKey,
                    isLocked: $0.isLocked,
                    account: Self.diagnosticAccountKey($0.account)
                )
            })
            let now = Date()
            let oldestPendingAge: Int64
            if let pending {
                oldestPendingAge = pending.compactMap { row -> Int64? in
                    let interval = now.timeIntervalSince(row.createdAt)
                    guard interval.isFinite else { return nil }
                    if interval <= 0 { return 0 }
                    if interval >= Double(Int64.max) { return Int64.max }
                    return Int64(interval)
                }.max() ?? 0
            } else {
                oldestPendingAge = -1
            }

            SDKLogger.event(
                "core_db_wallet_snapshot",
                category: .persistence,
                fields: [
                    "account_count": .integer(Int64(wallet.accounts.count)),
                    "birth_height": .unsignedInteger(UInt64(wallet.birthHeight)),
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "confirmed_count": .integer(Int64(confirmed.count)),
                    "confirmed_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(confirmed.map(\.amount))
                    ),
                    "locked_count": .integer(Int64(locked.count)),
                    "locked_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(locked.map(\.amount))
                    ),
                    "oldest_pending_input_age_seconds": .integer(oldestPendingAge),
                    "pending_input_count": .integer(pending.map { Int64($0.count) } ?? -1),
                    "pending_query_available": .boolean(pending != nil),
                    "spent_count": .integer(Int64(spent.count)),
                    "spent_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(spent.map(\.amount))
                    ),
                    "synced_height": .unsignedInteger(UInt64(wallet.syncedHeight)),
                    "transaction_count": .integer(
                        walletTransactions.map { Int64($0.count) } ?? -1
                    ),
                    "transaction_scan_available": .boolean(walletTransactions != nil),
                    "txo_count": .integer(Int64(walletTxos.count)),
                    "txo_fingerprint": .reference(txoFingerprint),
                    "unconfirmed_count": .integer(Int64(unconfirmed.count)),
                    "unconfirmed_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(unconfirmed.map(\.amount))
                    ),
                    "unspent_count": .integer(Int64(unspent.count)),
                    "unspent_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(unspent.map(\.amount))
                    ),
                    "wallet_reference": .reference(walletId),
                ]
            )

            let sortedAccounts = wallet.accounts.sorted {
                ($0.accountType, $0.standardTag, $0.accountIndex,
                 $0.registrationIndex, $0.keyClass)
                    < ($1.accountType, $1.standardTag, $1.accountIndex,
                       $1.registrationIndex, $1.keyClass)
            }
            for account in sortedAccounts {
                let key = Self.diagnosticAccountKey(account)!
                let accountTxos = walletTxos.filter { $0.account === account }
                let accountSpent = accountTxos.filter(\.isSpent)
                let accountUnspent = accountTxos.filter { !$0.isSpent }
                let accountConfirmed = accountTxos.filter(\.isConfirmed)
                let accountUnconfirmed = accountTxos.filter { !$0.isConfirmed }
                let accountLocked = accountTxos.filter(\.isLocked)
                let externalAddresses = account.coreAddresses.filter { $0.poolTypeTag == 0 }
                let internalAddresses = account.coreAddresses.filter { $0.poolTypeTag == 1 }
                let accountFingerprint = diagnosticFingerprint(accountTxos.map {
                    diagnosticTxoFingerprint(
                        outpoint: $0.outpoint,
                        amount: $0.amount,
                        height: $0.height,
                        scriptPubKey: $0.scriptPubKey,
                        isLocked: $0.isLocked,
                        account: key
                    )
                })
                SDKLogger.event(
                    "core_db_account_snapshot",
                    category: .persistence,
                    fields: [
                        "account_index": .unsignedInteger(UInt64(account.accountIndex)),
                        "account_reference": .reference(key.referenceMaterial),
                        "account_type": .unsignedInteger(UInt64(account.accountType)),
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "confirmed_count": .integer(Int64(accountConfirmed.count)),
                        "confirmed_value_duffs": .unsignedInteger(
                            diagnosticSaturatingSum(accountConfirmed.map(\.amount))
                        ),
                        "external_address_count": .integer(Int64(externalAddresses.count)),
                        "external_highest_used": .integer(Int64(account.externalHighestUsed)),
                        "internal_address_count": .integer(Int64(internalAddresses.count)),
                        "internal_highest_used": .integer(Int64(account.internalHighestUsed)),
                        "locked_count": .integer(Int64(accountLocked.count)),
                        "locked_value_duffs": .unsignedInteger(
                            diagnosticSaturatingSum(accountLocked.map(\.amount))
                        ),
                        "registration_index": .unsignedInteger(UInt64(account.registrationIndex)),
                        "spent_count": .integer(Int64(accountSpent.count)),
                        "spent_value_duffs": .unsignedInteger(
                            diagnosticSaturatingSum(accountSpent.map(\.amount))
                        ),
                        "standard_tag": .unsignedInteger(UInt64(account.standardTag)),
                        "txo_fingerprint": .reference(accountFingerprint),
                        "unconfirmed_count": .integer(Int64(accountUnconfirmed.count)),
                        "unconfirmed_value_duffs": .unsignedInteger(
                            diagnosticSaturatingSum(accountUnconfirmed.map(\.amount))
                        ),
                        "unspent_count": .integer(Int64(accountUnspent.count)),
                        "unspent_value_duffs": .unsignedInteger(
                            diagnosticSaturatingSum(accountUnspent.map(\.amount))
                        ),
                        "used_address_count": .integer(
                            Int64(account.coreAddresses.filter(\.isUsed).count)
                        ),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }

            Self.logTxoAnomalies(
                walletId: walletId,
                checkpoint: checkpoint,
                txos: walletTxos
            )
            // Decoding a heavily mixed wallet's full transaction history can
            // be expensive. The exact #4438 audit is needed for the manually
            // exported artifact, not for restoring Rust, so keep startup's
            // persistence queue limited to lightweight summaries.
            if checkpoint == .preExport,
               let allTransactions {
                Self.auditCoinJoinOwnedBip44Outputs(
                    wallet: wallet,
                    walletId: walletId,
                    checkpoint: checkpoint,
                    allTxos: allTxos,
                    allTransactions: allTransactions
                )
            }

            let assetLocks: [CoreWalletDatabaseDiagnosticSnapshot.AssetLock]
            let assetLocksAvailable: Bool
            do {
                assetLocks = try Self.logAssetLockDatabaseSnapshot(
                    context: backgroundContext,
                    walletId: walletId,
                    checkpoint: checkpoint,
                    walletTransactions: walletTransactions
                )
                assetLocksAvailable = true
            } catch {
                assetLocks = []
                assetLocksAvailable = false
                SDKLogger.event(
                    "asset_lock_db_snapshot",
                    category: .persistence,
                    severity: .warning,
                    fields: [
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "query_available": .boolean(false),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }
            do {
                try Self.logShieldedStoreSnapshot(
                    context: backgroundContext,
                    walletId: walletId,
                    checkpoint: checkpoint
                )
            } catch {
                SDKLogger.event(
                    "shielded_store_snapshot",
                    category: .shielded,
                    severity: .warning,
                    fields: [
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "query_available": .boolean(false),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }

            let snapshot = CoreWalletDatabaseDiagnosticSnapshot(
                walletId: walletId,
                accounts: sortedAccounts.compactMap(Self.diagnosticAccountKey),
                unspentTxos: unspent.map {
                    CoreWalletDatabaseDiagnosticSnapshot.Txo(
                        outpoint: $0.outpoint,
                        amount: $0.amount,
                        height: $0.height,
                        scriptPubKey: $0.scriptPubKey,
                        isLocked: $0.isLocked,
                        account: Self.diagnosticAccountKey($0.account)
                    )
                },
                assetLocks: assetLocks,
                assetLocksAvailable: assetLocksAvailable
            )
            return snapshot
        } catch {
            SDKLogger.event(
                "core_diagnostics_unavailable",
                category: .persistence,
                severity: .error,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("swiftdata_fetch_failed"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return nil
        }
    }

    /// Logs the exact UTXO slice handed to Rust, independently of the broader
    /// database snapshot. This sits after compact-write, so `emitted_count`
    /// cannot be confused with the number of fetched candidates.
    func logCoreRestoreBufferSnapshotOnQueue(
        walletId: Data,
        rows: [PersistentTxo],
        emittedCount: Int,
        errored: Bool
    ) {
        let candidates = rows.map { row in
            let rejection: CoreWalletDiagnosticAnalyzer.RestoreCandidate.RejectionReason?
            if row.account == nil {
                rejection = .missingAccount
            } else if row.txid.count != 32 {
                rejection = .invalidTxid
            } else if let account = row.account,
                      UInt8(exactly: account.accountType) == nil {
                rejection = .invalidAccountType
            } else {
                rejection = nil
            }
            return CoreWalletDiagnosticAnalyzer.RestoreCandidate(
                amount: row.amount,
                accountType: row.account?.accountType,
                standardTag: row.account?.standardTag,
                rejectionReason: rejection
            )
        }
        // A validation error deallocates the compact buffer and aborts the
        // whole callback, so zero rows were actually handed to Rust even if
        // some valid rows preceded the corrupt one.
        let summary = CoreWalletDiagnosticAnalyzer.summarizeRestoreBuffer(
            candidates: candidates,
            emittedCount: emittedCount,
            errored: errored
        )
        let hasRejectedRows = summary.missingAccountCount > 0
            || summary.invalidTxidCount > 0
            || summary.invalidAccountTypeCount > 0

        SDKLogger.event(
            "core_restore_buffer_snapshot",
            category: .persistence,
            severity: errored ? .error : (hasRejectedRows ? .warning : .info),
            fields: [
                "candidate_count": .integer(Int64(summary.candidateCount)),
                "candidate_bip44_count": .integer(Int64(summary.candidateBip44Count)),
                "candidate_bip44_value_duffs": .unsignedInteger(
                    summary.candidateBip44ValueDuffs
                ),
                "candidate_coinjoin_count": .integer(Int64(summary.candidateCoinJoinCount)),
                "candidate_coinjoin_value_duffs": .unsignedInteger(
                    summary.candidateCoinJoinValueDuffs
                ),
                "candidate_value_duffs": .unsignedInteger(summary.candidateValueDuffs),
                "built_count": .integer(Int64(summary.builtCount)),
                "checkpoint": .publicText(CoreWalletDiagnosticCheckpoint.restoreBuffer.rawValue),
                "emitted_count": .integer(Int64(summary.emittedCandidates.count)),
                "emitted_bip44_count": .integer(Int64(summary.emittedBip44Count)),
                "emitted_bip44_value_duffs": .unsignedInteger(
                    summary.emittedBip44ValueDuffs
                ),
                "emitted_coinjoin_count": .integer(Int64(summary.emittedCoinJoinCount)),
                "emitted_coinjoin_value_duffs": .unsignedInteger(
                    summary.emittedCoinJoinValueDuffs
                ),
                "emitted_value_duffs": .unsignedInteger(summary.emittedValueDuffs),
                "errored": .boolean(errored),
                "skipped_invalid_account_type_count": .integer(
                    Int64(summary.invalidAccountTypeCount)
                ),
                "skipped_invalid_txid_count": .integer(Int64(summary.invalidTxidCount)),
                "skipped_missing_account_count": .integer(Int64(summary.missingAccountCount)),
                "wallet_reference": .reference(walletId),
            ]
        )
    }

    private static func diagnosticAccountKey(
        _ account: PersistentAccount?
    ) -> CoreWalletDatabaseDiagnosticSnapshot.AccountKey? {
        guard let account else { return nil }
        return CoreWalletDatabaseDiagnosticSnapshot.AccountKey(
            typeTag: account.accountType,
            standardTag: account.standardTag,
            index: account.accountIndex,
            registrationIndex: account.registrationIndex,
            keyClass: account.keyClass,
            userIdentityId: account.userIdentityId,
            friendIdentityId: account.friendIdentityId
        )
    }

    /// Read the relationship-owned wallet independently of the denormalized
    /// `PersistentTxo.walletId`. Diagnostics must compare the two sources;
    /// `resolvedWalletId(of:)` deliberately prefers the denormalized value and
    /// would therefore hide exactly the corruption we are trying to expose.
    private static func relationshipWalletId(of txo: PersistentTxo) -> Data? {
        let account: PersistentAccount? = txo.account
        guard let account else { return nil }
        let wallet: PersistentWallet? = account.wallet
        return wallet?.walletId
    }

    private static func logTxoAnomalies(
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint,
        txos: [PersistentTxo]
    ) {
        let result = CoreWalletDiagnosticAnalyzer.databaseTxoAnomalies(txos.map { txo in
            let relationshipWalletId = relationshipWalletId(of: txo)
            return CoreWalletDiagnosticAnalyzer.DatabaseTxoAuditRow(
                txo: CoreWalletDatabaseDiagnosticSnapshot.Txo(
                    outpoint: txo.outpoint,
                    amount: txo.amount,
                    height: txo.height,
                    scriptPubKey: txo.scriptPubKey,
                    isLocked: txo.isLocked,
                    account: diagnosticAccountKey(txo.account)
                ),
                hasParentTransaction: txo.transaction != nil,
                walletIdMismatch: !txo.walletId.isEmpty
                    && relationshipWalletId != nil
                    && txo.walletId != relationshipWalletId,
                isSpent: txo.isSpent,
                hasSpendingTransaction: txo.spendingTransaction != nil
            )
        })
        SDKLogger.event(
            "core_db_anomaly_summary",
            category: .persistence,
            severity: result.details.isEmpty ? .info : .warning,
            fields: [
                "anomaly_count": .integer(Int64(result.details.count)),
                "checkpoint": .publicText(checkpoint.rawValue),
                "detail_count": .integer(Int64(result.emittedDetails.count)),
                "empty_script_count": .integer(Int64(result.count(reason: "empty_script_pubkey"))),
                "invalid_outpoint_count": .integer(Int64(
                    result.count(reason: "invalid_outpoint_length")
                )),
                "missing_account_count": .integer(Int64(result.count(reason: "missing_account"))),
                "missing_parent_transaction_count": .integer(Int64(
                    result.count(reason: "missing_parent_transaction")
                )),
                "spent_relation_mismatch_count": .integer(Int64(
                    result.count(reason: "spent_without_spending_transaction")
                        + result.count(reason: "unspent_with_spending_transaction")
                )),
                "truncated_count": .integer(Int64(result.truncatedCount)),
                "wallet_mismatch_count": .integer(Int64(
                    result.count(reason: "wallet_id_mismatch")
                )),
                "wallet_reference": .reference(walletId),
            ]
        )
        for detail in result.emittedDetails {
            SDKLogger.event(
                "core_db_txo_anomaly",
                category: .persistence,
                severity: .warning,
                fields: [
                    "amount_duffs": .unsignedInteger(detail.txo.amount),
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "height": .unsignedInteger(UInt64(detail.txo.height)),
                    "outpoint_reference": .reference(detail.txo.outpoint),
                    "reason": .publicText(detail.reason),
                    "wallet_reference": .reference(walletId),
                ]
            )
        }
    }

    /// Exact detector for dashpay/platform#4438. It does not trust the
    /// transaction's persisted role: it decodes inputs, proves at least one
    /// spends a known CoinJoin TXO, then checks every decoded output against
    /// the persisted BIP44 address pool and the TXO table.
    private static func auditCoinJoinOwnedBip44Outputs(
        wallet: PersistentWallet,
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint,
        allTxos: [PersistentTxo],
        allTransactions: [PersistentTransaction]
    ) {
        let coinJoinOutpoints = Set(allTxos.compactMap { txo -> Data? in
            guard relationshipWalletId(of: txo) == walletId,
                  txo.account?.accountType == 1
            else { return nil }
            return txo.outpoint
        })
        var bip44Addresses: [String: PersistentAccount] = [:]
        for account in wallet.accounts where account.accountType == 0 && account.standardTag == 0 {
            for coreAddress in account.coreAddresses where bip44Addresses[coreAddress.address] == nil {
                bip44Addresses[coreAddress.address] = account
            }
        }
        let txoByOutpoint = Dictionary(grouping: allTxos, by: \.outpoint)

        var candidateCount = 0
        var decodeFailureCount = 0
        var ownedOutputCount = 0
        var ownedOutputValue: UInt64 = 0
        var validCount = 0
        var anomalies: [(tx: PersistentTransaction, vout: UInt32, amount: UInt64,
                         outpoint: Data, reason: String)] = []

        guard let network = wallet.network else {
            SDKLogger.event(
                "core_owned_output_audit_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("wallet_network_unknown"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return
        }

        for transaction in allTransactions where !transaction.transactionData.isEmpty {
            let decoded: DecodedTransaction
            do {
                decoded = try TransactionDecoder.decode(transaction.transactionData, network: network)
            } catch {
                // Only count decode failures for rows already associated with
                // this wallet; unrelated-wallet corruption must not pollute
                // this wallet's audit result.
                if walletOwnsTransaction(walletId: walletId, transaction: transaction) {
                    decodeFailureCount += 1
                }
                continue
            }
            let spendsCoinJoin = decoded.inputs.contains { input in
                coinJoinOutpoints.contains(
                    PersistentTxo.makeOutpoint(txid: input.prevTxid, vout: input.prevVout)
                )
            }
            guard spendsCoinJoin else { continue }
            candidateCount += 1

            for (index, output) in decoded.outputs.enumerated() {
                guard let address = output.address,
                      let expectedAccount = bip44Addresses[address]
                else { continue }
                ownedOutputCount += 1
                let (newValue, overflow) = ownedOutputValue.addingReportingOverflow(output.valueDuffs)
                ownedOutputValue = overflow ? UInt64.max : newValue
                let vout = UInt32(index)
                let outpoint = PersistentTxo.makeOutpoint(txid: decoded.txid, vout: vout)
                guard let rows = txoByOutpoint[outpoint], let row = rows.first else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "missing_txo"))
                    continue
                }
                guard relationshipWalletId(of: row) == walletId,
                      row.walletId.isEmpty || row.walletId == walletId
                else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "wrong_wallet"))
                    continue
                }
                guard row.account === expectedAccount,
                      row.account?.accountType == 0,
                      row.account?.standardTag == 0
                else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "wrong_account"))
                    continue
                }
                guard row.amount == output.valueDuffs else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "amount_mismatch"))
                    continue
                }
                guard row.scriptPubKey == output.scriptPubkey else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "script_mismatch"))
                    continue
                }
                validCount += 1
            }
        }

        anomalies.sort {
            if $0.outpoint != $1.outpoint {
                return $0.outpoint.lexicographicallyPrecedes($1.outpoint)
            }
            return $0.reason < $1.reason
        }
        let anomalyGroups = Dictionary(grouping: anomalies, by: { $0.reason })
        let truncatedAnomalyCount = anomalyGroups.values.reduce(0) {
            $0 + max(0, $1.count - CoreDiagnosticConstants.detailLimit)
        }
        let missingCount = anomalies.filter { $0.reason == "missing_txo" }.count
        let missingValue = diagnosticSaturatingSum(anomalies.compactMap {
            $0.reason == "missing_txo" ? $0.amount : nil
        })
        SDKLogger.event(
            "core_owned_output_audit_summary",
            category: .persistence,
            severity: anomalies.isEmpty && decodeFailureCount == 0 ? .info : .warning,
            fields: [
                "audit_incomplete": .boolean(decodeFailureCount > 0),
                "candidate_transaction_count": .integer(Int64(candidateCount)),
                "checkpoint": .publicText(checkpoint.rawValue),
                "coinjoin_to_bip44_missing_count": .integer(Int64(missingCount)),
                "coinjoin_to_bip44_missing_value_duffs": .unsignedInteger(missingValue),
                "decode_failure_count": .integer(Int64(decodeFailureCount)),
                "owned_bip44_output_count": .integer(Int64(ownedOutputCount)),
                "owned_bip44_output_value_duffs": .unsignedInteger(ownedOutputValue),
                "persisted_valid_count": .integer(Int64(validCount)),
                "total_anomaly_count": .integer(Int64(anomalies.count)),
                "truncated_count": .integer(Int64(truncatedAnomalyCount)),
                "wallet_reference": .reference(walletId),
            ]
        )
        for reason in anomalyGroups.keys.sorted() {
            for anomaly in (anomalyGroups[reason] ?? []).prefix(CoreDiagnosticConstants.detailLimit) {
                SDKLogger.event(
                    "core_owned_output_anomaly",
                    category: .persistence,
                    severity: .warning,
                    fields: [
                        "amount_duffs": .unsignedInteger(anomaly.amount),
                        "block_height": .unsignedInteger(UInt64(anomaly.tx.blockHeight)),
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "input_account_kind": .publicText("coinjoin"),
                        "outpoint_reference": .reference(anomaly.outpoint),
                        "output_account_kind": .publicText("bip44"),
                        "reason": .publicText(reason),
                        "transaction_context": .unsignedInteger(UInt64(anomaly.tx.context)),
                        "transaction_reference": .reference(anomaly.tx.txid),
                        "vout": .unsignedInteger(UInt64(anomaly.vout)),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }
        }
    }

    private static func logAssetLockDatabaseSnapshot(
        context: ModelContext,
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint,
        walletTransactions: [PersistentTransaction]?
    ) throws -> [CoreWalletDatabaseDiagnosticSnapshot.AssetLock] {
        let rows = try context.fetch(
            FetchDescriptor<PersistentAssetLock>(
                predicate: PersistentAssetLock.predicate(walletId: walletId)
            )
        )
        SDKLogger.event(
            "asset_lock_db_snapshot",
            category: .persistence,
            fields: [
                "checkpoint": .publicText(checkpoint.rawValue),
                "core_type_8_transaction_count": .integer(
                    walletTransactions.map { Int64($0.filter(\.isAssetLock).count) } ?? -1
                ),
                "core_transaction_scan_available": .boolean(walletTransactions != nil),
                "lock_count": .integer(Int64(rows.count)),
                "proof_present_count": .integer(Int64(rows.filter {
                    $0.proofBytes?.isEmpty == false
                }.count)),
                "query_available": .boolean(true),
                "shielded_funding_count": .integer(Int64(rows.filter {
                    $0.fundingTypeRaw == 5
                }.count)),
                "transaction_bytes_present_count": .integer(Int64(rows.filter {
                    !$0.transactionBytes.isEmpty
                }.count)),
                "wallet_reference": .reference(walletId),
            ]
        )

        let groups = Dictionary(grouping: rows) {
            "\($0.fundingTypeRaw):\($0.statusRaw)"
        }
        for key in groups.keys.sorted() {
            guard let group = groups[key], let first = group.first else { continue }
            SDKLogger.event(
                "asset_lock_db_group",
                category: .persistence,
                fields: [
                    "amount_duffs": .integer(
                        diagnosticSignedSaturatingSum(group.map(\.amountDuffs))
                    ),
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "count": .integer(Int64(group.count)),
                    "funding_type": .integer(Int64(first.fundingTypeRaw)),
                    "status": .integer(Int64(first.statusRaw)),
                    "wallet_reference": .reference(walletId),
                ]
            )
        }
        return rows.map {
            CoreWalletDatabaseDiagnosticSnapshot.AssetLock(
                outpointDisplay: $0.outPointHex,
                fundingType: $0.fundingTypeRaw,
                status: $0.statusRaw,
                accountIndex: UInt32(bitPattern: $0.accountIndexRaw),
                registrationIndex: UInt32(bitPattern: $0.identityIndexRaw),
                amountDuffs: UInt64(exactly: $0.amountDuffs),
                hasProof: $0.proofBytes?.isEmpty == false
            )
        }
    }

    private static func logShieldedStoreSnapshot(
        context: ModelContext,
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) throws {
        let notes = try context.fetch(FetchDescriptor<PersistentShieldedNote>(
            predicate: #Predicate { $0.walletId == walletId }
        ))
        let outgoing = try context.fetch(FetchDescriptor<PersistentShieldedOutgoingNote>(
            predicate: #Predicate { $0.walletId == walletId }
        ))
        let states = try context.fetch(FetchDescriptor<PersistentShieldedSyncState>(
            predicate: #Predicate { $0.walletId == walletId }
        ))
        let activity = try context.fetch(FetchDescriptor<PersistentShieldedActivity>(
            predicate: #Predicate { $0.walletId == walletId }
        ))
        let viewingKeys = try context.fetch(FetchDescriptor<PersistentShieldedViewingKey>(
            predicate: #Predicate { $0.walletId == walletId }
        ))
        let summary = CoreWalletDiagnosticAnalyzer.summarizeShieldedStore(
            notes: notes.map { .init(value: $0.value, isSpent: $0.isSpent) },
            outgoingNoteCount: outgoing.count,
            activityStatuses: activity.map(\.status),
            viewingKeyCount: viewingKeys.count,
            syncWatermarks: states.map(\.lastSyncedIndex)
        )
        SDKLogger.event(
            "shielded_store_snapshot",
            category: .shielded,
            fields: [
                "activity_count": .integer(Int64(summary.activityCount)),
                "activity_failed_count": .integer(Int64(summary.activityFailedCount)),
                "activity_pending_count": .integer(Int64(summary.activityPendingCount)),
                "checkpoint": .publicText(checkpoint.rawValue),
                "maximum_sync_watermark": .unsignedInteger(summary.maximumSyncWatermark),
                "note_count": .integer(Int64(summary.noteCount)),
                "outgoing_note_count": .integer(Int64(summary.outgoingNoteCount)),
                "query_available": .boolean(true),
                "spent_note_count": .integer(Int64(summary.spentNoteCount)),
                "spent_value_credits": .unsignedInteger(summary.spentValueCredits),
                "subwallet_sync_state_count": .integer(Int64(summary.subwalletSyncStateCount)),
                "unspent_note_count": .integer(Int64(summary.unspentNoteCount)),
                "unspent_value_credits": .unsignedInteger(summary.unspentValueCredits),
                "viewing_key_count": .integer(Int64(summary.viewingKeyCount)),
                "wallet_reference": .reference(walletId),
            ]
        )
    }
}

// MARK: - Rust memory comparison

@MainActor
extension PlatformWalletManager {
    /// Emit a best-effort, read-only snapshot immediately before a diagnostic
    /// export. The method intentionally never throws: a failed sub-query is a
    /// diagnostic fact and is logged as `unavailable`, not reported as zero.
    public func emitCoreWalletDiagnostics(for walletId: Data) async {
        await emitCoreWalletDiagnostics(for: walletId, checkpoint: .preExport)
    }

    /// Coordinates the queue-owned SwiftData snapshot with read-only Rust FFI
    /// queries. Admission happens after the database await, then keeps the
    /// native handle alive until the off-main worker finishes.
    private func emitCoreWalletDiagnostics(
        for walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) async {
        guard walletId.count == 32, let handler = persistence else {
            SDKLogger.event(
                "core_diagnostics_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("invalid_wallet_or_persistence_disabled"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return
        }
        let database = await handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            checkpoint: checkpoint
        )
        guard let database else { return }
        // The DB await above lets shutdown interleave. Admission is atomic on
        // MainActor and keeps the copied handle alive across the off-main FFI
        // work; shutdown drains this operation before consuming the handle.
        guard isConfigured, handle != NULL_HANDLE else {
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("manager_not_configured_after_database_snapshot"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return
        }
        do {
            try admitCoreDiagnosticsNativeOp()
        } catch {
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("manager_shutdown_in_progress"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return
        }
        defer { finishCoreDiagnosticsNativeOp() }

        let managerHandle = handle
        let managedWallet = wallets[walletId]
        await withCheckedContinuation { continuation in
            Self.destroyQueue.async {
                Self.emitCoreMemoryDiagnostics(
                    managerHandle: managerHandle,
                    managedWallet: managedWallet,
                    database: database,
                    checkpoint: checkpoint
                )
                continuation.resume()
            }
        }
    }

    /// Runs all Rust-memory reads on `destroyQueue`. Each subsystem reports its
    /// own unavailable state so one failed query does not hide the others.
    private nonisolated static func emitCoreMemoryDiagnostics(
        managerHandle: Handle,
        managedWallet: ManagedPlatformWallet?,
        database: CoreWalletDatabaseDiagnosticSnapshot,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) {
        // Keep the two Rust-memory sources independent: corrupt account state
        // must not suppress the AssetLock evidence that can explain a missing
        // balance (and vice versa).
        compareAssetLocks(
            database,
            managedWallet: managedWallet,
            checkpoint: checkpoint
        )
        let balanceQuery = diagnosticAccountBalances(
            managerHandle: managerHandle,
            walletId: database.walletId
        )
        guard case .success(let balances) = balanceQuery else {
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("account_balance_query_failed"),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
            return
        }

        var memoryTxos: [CoreWalletDatabaseDiagnosticSnapshot.Txo] = []
        var unavailableAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey> = []
        let sortedBalances = balances.sorted {
            Self.diagnosticAccountKey($0).referenceMaterial.lexicographicallyPrecedes(
                Self.diagnosticAccountKey($1).referenceMaterial
            )
        }
        for balance in sortedBalances {
            let key = Self.diagnosticAccountKey(balance)
            let query = diagnosticAccountUtxos(
                managerHandle: managerHandle,
                walletId: database.walletId,
                balance: balance
            )
            guard case .success(let utxos) = query else {
                unavailableAccounts.insert(key)
                SDKLogger.event(
                    "core_memory_account_snapshot",
                    category: .persistence,
                    severity: .warning,
                    fields: [
                        "account_reference": .reference(key.referenceMaterial),
                        "account_type": .unsignedInteger(UInt64(key.typeTag)),
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "query_available": .boolean(false),
                        "wallet_reference": .reference(database.walletId),
                    ]
                )
                continue
            }
            let materials = utxos.map {
                diagnosticTxoFingerprint(
                    outpoint: $0.outpoint,
                    amount: $0.amount,
                    height: $0.height,
                    scriptPubKey: $0.scriptPubKey,
                    isLocked: $0.isLocked,
                    account: key
                )
            }
            SDKLogger.event(
                "core_memory_account_snapshot",
                category: .persistence,
                fields: [
                    "account_index": .unsignedInteger(UInt64(balance.index)),
                    "account_reference": .reference(key.referenceMaterial),
                    "account_type": .unsignedInteger(UInt64(balance.typeTag)),
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "confirmed_duffs": .unsignedInteger(balance.confirmed),
                    "immature_duffs": .unsignedInteger(balance.immature),
                    "locked_duffs": .unsignedInteger(balance.locked),
                    "query_available": .boolean(true),
                    "standard_tag": .unsignedInteger(UInt64(balance.standardTag)),
                    "unconfirmed_duffs": .unsignedInteger(balance.unconfirmed),
                    "utxo_count": .integer(Int64(utxos.count)),
                    "utxo_fingerprint": .reference(diagnosticFingerprint(materials)),
                    "utxo_value_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(utxos.map(\.amount))
                    ),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
            memoryTxos.append(contentsOf: utxos)
        }
        compareDatabase(
            database,
            memoryTxos: memoryTxos,
            memoryAccounts: Set(balances.map(Self.diagnosticAccountKey)),
            unavailableAccounts: unavailableAccounts,
            checkpoint: checkpoint
        )
    }

    /// Logs the deterministic DB↔Rust UTXO diff, excluding accounts whose Rust
    /// UTXO query failed instead of falsely reporting all their rows DB-only.
    private nonisolated static func compareDatabase(
        _ database: CoreWalletDatabaseDiagnosticSnapshot,
        memoryTxos: [CoreWalletDatabaseDiagnosticSnapshot.Txo],
        memoryAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>,
        unavailableAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) {
        let excludedDatabaseTxos = database.unspentTxos.filter { row in
            row.account.map(unavailableAccounts.contains) ?? false
        }
        let comparableDatabaseTxos = database.unspentTxos.filter { row in
            !(row.account.map(unavailableAccounts.contains) ?? false)
        }
        let result = CoreWalletDiagnosticAnalyzer.compareTxos(
            database: comparableDatabaseTxos,
            memory: memoryTxos,
            databaseAccounts: Set(database.accounts),
            memoryAccounts: memoryAccounts
        )
        SDKLogger.event(
            "core_db_memory_diff_summary",
            category: .persistence,
            severity: result.details.isEmpty
                && result.databaseAccountOnlyCount == 0
                && result.memoryAccountOnlyCount == 0
                ? .info : .warning,
            fields: [
                "checkpoint": .publicText(checkpoint.rawValue),
                "common_count": .integer(Int64(result.commonCount)),
                "database_account_only_count": .integer(
                    Int64(result.databaseAccountOnlyCount)
                ),
                "database_only_count": .integer(Int64(result.databaseOnlyCount)),
                "diff_incomplete": .boolean(!unavailableAccounts.isEmpty),
                "excluded_database_txo_count": .integer(Int64(excludedDatabaseTxos.count)),
                "field_mismatch_count": .integer(Int64(result.fieldMismatchCount)),
                "memory_only_count": .integer(Int64(result.memoryOnlyCount)),
                "memory_account_only_count": .integer(Int64(result.memoryAccountOnlyCount)),
                "truncated_count": .integer(Int64(result.truncatedCount)),
                "unavailable_account_count": .integer(Int64(unavailableAccounts.count)),
                "wallet_reference": .reference(database.walletId),
            ]
        )
        for detail in result.emittedDetails {
            logDiffItem(
                database.walletId,
                checkpoint,
                detail.row,
                detail.outpoint,
                detail.reason
            )
        }
    }

    private nonisolated static func logDiffItem(
        _ walletId: Data,
        _ checkpoint: CoreWalletDiagnosticCheckpoint,
        _ row: CoreWalletDatabaseDiagnosticSnapshot.Txo,
        _ outpoint: Data,
        _ reason: String
    ) {
        SDKLogger.event(
            "core_db_memory_diff_item",
            category: .persistence,
            severity: .warning,
            fields: [
                "amount_duffs": .unsignedInteger(row.amount),
                "checkpoint": .publicText(checkpoint.rawValue),
                "height": .unsignedInteger(UInt64(row.height)),
                "outpoint_reference": .reference(outpoint),
                "reason": .publicText(reason),
                "wallet_reference": .reference(walletId),
            ]
        )
    }

    /// Captures the managed wallet's tracked locks and compares them with the
    /// queue-safe SwiftData snapshot. Raw outpoints are only reference-hashed.
    private nonisolated static func compareAssetLocks(
        _ database: CoreWalletDatabaseDiagnosticSnapshot,
        managedWallet: ManagedPlatformWallet?,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) {
        let memory: [ManagedAssetLockManager.TrackedAssetLock]
        do {
            guard let managedWallet else {
                throw PlatformWalletError.notFound("diagnostic wallet is not loaded")
            }
            memory = try managedWallet.assetLockManager().listTrackedLocks()
        } catch {
            SDKLogger.event(
                "asset_lock_memory_snapshot",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "query_available": .boolean(false),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
            return
        }
        SDKLogger.event(
            "asset_lock_memory_snapshot",
            category: .persistence,
            fields: [
                "checkpoint": .publicText(checkpoint.rawValue),
                "lock_count": .integer(Int64(memory.count)),
                "locked_value_duffs": .unsignedInteger(
                    diagnosticSaturatingSum(memory.map(\.amount))
                ),
                "proof_present_count": .integer(Int64(memory.filter(\.hasProof).count)),
                "query_available": .boolean(true),
                "shielded_funding_count": .integer(Int64(memory.filter {
                    $0.fundingType == .assetLockShieldedAddressTopUp
                }.count)),
                "wallet_reference": .reference(database.walletId),
            ]
        )

        let groups = Dictionary(grouping: memory) {
            "\($0.fundingType.rawValue):\($0.status.rawValue)"
        }
        for key in groups.keys.sorted() {
            guard let group = groups[key], let first = group.first else { continue }
            SDKLogger.event(
                "asset_lock_memory_group",
                category: .persistence,
                fields: [
                    "amount_duffs": .unsignedInteger(
                        diagnosticSaturatingSum(group.map(\.amount))
                    ),
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "count": .integer(Int64(group.count)),
                    "funding_type": .unsignedInteger(UInt64(first.fundingType.rawValue)),
                    "proof_present_count": .integer(Int64(group.filter(\.hasProof).count)),
                    "status": .unsignedInteger(UInt64(first.status.rawValue)),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
        }

        let normalizedMemory = memory.map { row in
            CoreWalletDatabaseDiagnosticSnapshot.AssetLock(
                outpointDisplay: Self.assetLockOutpointDisplay(txid: row.txid, vout: row.vout),
                fundingType: Int(row.fundingType.rawValue),
                status: Int(row.status.rawValue),
                accountIndex: row.accountIndex,
                registrationIndex: row.identityIndex,
                amountDuffs: row.amount,
                hasProof: row.hasProof
            )
        }
        guard database.assetLocksAvailable else {
            SDKLogger.event(
                "asset_lock_db_memory_diff_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "database_query_available": .boolean(false),
                    "diff_incomplete": .boolean(true),
                    "mismatch_count": .integer(0),
                    "truncated_count": .integer(0),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
            return
        }
        let result = CoreWalletDiagnosticAnalyzer.compareAssetLocks(
            database: database.assetLocks,
            memory: normalizedMemory
        )
        SDKLogger.event(
            "asset_lock_db_memory_diff_summary",
            category: .persistence,
            severity: result.details.isEmpty ? .info : .warning,
            fields: [
                "checkpoint": .publicText(checkpoint.rawValue),
                "database_query_available": .boolean(true),
                "diff_incomplete": .boolean(false),
                "mismatch_count": .integer(Int64(result.details.count)),
                "truncated_count": .integer(Int64(result.truncatedCount)),
                "wallet_reference": .reference(database.walletId),
            ]
        )
        for detail in result.emittedDetails {
            SDKLogger.event(
                "asset_lock_db_memory_diff_item",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "outpoint_reference": .referenceString(detail.outpointDisplay),
                    "reason": .publicText(detail.reason),
                    "wallet_reference": .reference(database.walletId),
                ]
            )
        }
    }

    /// Copies the Rust-owned account-balance array into Swift values and frees
    /// the FFI allocation on every successful non-empty path.
    private nonisolated static func diagnosticAccountBalances(
        managerHandle: Handle,
        walletId: Data
    ) -> Result<[AccountBalance], PlatformWalletError> {
        var outEntries: UnsafePointer<AccountBalanceEntryFFI>?
        var outCount: UInt = 0
        let ffi = walletId.withUnsafeBytes { raw in
            platform_wallet_manager_get_account_balances(
                managerHandle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outEntries,
                &outCount
            )
        }
        let result = PlatformWalletResult(ffi)
        guard result.isSuccess else { return .failure(PlatformWalletError(result: result)) }
        guard let entries = outEntries, outCount > 0 else { return .success([]) }
        defer {
            platform_wallet_manager_free_account_balances(
                UnsafeMutablePointer(mutating: entries), outCount
            )
        }
        return .success((0..<Int(outCount)).map { index in
            var entry = entries[index]
            let userId = Swift.withUnsafeBytes(of: &entry.user_identity_id) { Data($0) }
            let friendId = Swift.withUnsafeBytes(of: &entry.friend_identity_id) { Data($0) }
            return AccountBalance(
                typeTag: entry.type_tag,
                standardTag: entry.standard_tag,
                index: entry.index,
                registrationIndex: entry.registration_index,
                keyClass: entry.key_class,
                userIdentityId: userId,
                friendIdentityId: friendId,
                confirmed: entry.confirmed,
                unconfirmed: entry.unconfirmed,
                immature: entry.immature,
                locked: entry.locked,
                keysUsed: entry.keys_used,
                keysTotal: entry.keys_total
            )
        })
    }

    /// Marshals one account selector, copies its Rust-owned UTXO slice, and
    /// releases the native allocation after all pointed-to scripts are copied.
    private nonisolated static func diagnosticAccountUtxos(
        managerHandle: Handle,
        walletId: Data,
        balance: AccountBalance
    ) -> Result<[CoreWalletDatabaseDiagnosticSnapshot.Txo], PlatformWalletError> {
        var spec = AccountSpecFFI()
        spec.type_tag = balance.typeTag
        spec.standard_tag = balance.standardTag
        spec.index = balance.index
        spec.registration_index = balance.registrationIndex
        spec.key_class = balance.keyClass
        _ = Swift.withUnsafeMutableBytes(of: &spec.user_identity_id) { raw in
            balance.userIdentityId.copyBytes(
                to: raw.bindMemory(to: UInt8.self),
                count: min(32, balance.userIdentityId.count)
            )
        }
        _ = Swift.withUnsafeMutableBytes(of: &spec.friend_identity_id) { raw in
            balance.friendIdentityId.copyBytes(
                to: raw.bindMemory(to: UInt8.self),
                count: min(32, balance.friendIdentityId.count)
            )
        }
        var outEntries: UnsafePointer<AccountUtxoEntryFFI>?
        var outCount: UInt = 0
        let ffi = walletId.withUnsafeBytes { raw in
            platform_wallet_account_utxos(
                managerHandle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &spec,
                &outEntries,
                &outCount
            )
        }
        let result = PlatformWalletResult(ffi)
        guard result.isSuccess else { return .failure(PlatformWalletError(result: result)) }
        guard let entries = outEntries, outCount > 0 else { return .success([]) }
        defer {
            platform_wallet_account_utxos_free(
                UnsafeMutablePointer(mutating: entries), outCount
            )
        }
        let key = Self.diagnosticAccountKey(balance)
        return .success((0..<Int(outCount)).map { index in
            var entry = entries[index]
            let txid = Swift.withUnsafeBytes(of: &entry.outpoint_txid) { Data($0) }
            let script = entry.script_pubkey.map {
                Data(bytes: $0, count: Int(entry.script_pubkey_len))
            } ?? Data()
            return CoreWalletDatabaseDiagnosticSnapshot.Txo(
                outpoint: PersistentTxo.makeOutpoint(txid: txid, vout: entry.outpoint_vout),
                amount: entry.value_duffs,
                height: entry.height,
                scriptPubKey: script,
                isLocked: entry.is_locked,
                account: key
            )
        })
    }

    private nonisolated static func diagnosticAccountKey(
        _ balance: AccountBalance
    ) -> CoreWalletDatabaseDiagnosticSnapshot.AccountKey {
        CoreWalletDatabaseDiagnosticSnapshot.AccountKey(
            typeTag: UInt32(balance.typeTag),
            standardTag: balance.standardTag,
            index: balance.index,
            registrationIndex: balance.registrationIndex,
            keyClass: balance.keyClass,
            userIdentityId: balance.userIdentityId,
            friendIdentityId: balance.friendIdentityId
        )
    }

    private nonisolated static func assetLockOutpointDisplay(
        txid: Data,
        vout: UInt32
    ) -> String {
        let display = txid.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(display):\(vout)"
    }
}
