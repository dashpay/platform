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

/// One-way flag from `PlatformWalletManager.shutdown()` to a diagnostic pass
/// running off the main actor. Checked before every FFI read; once set, the
/// pass reports the reads it skipped and returns, releasing its admission.
final class CoreDiagnosticsCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    var isCancelled: Bool {
        lock.withLock { cancelled }
    }

    func cancel() {
        lock.withLock { cancelled = true }
    }
}

/// Ceilings on what one support export may materialize at once.
///
/// The exact #4438 audit needs every TXO and every transaction cross-wallet —
/// an output absent from this wallet may be `wrong_wallet`, not `missing_txo`,
/// and only the full table can say which. That pass runs inside the
/// persistence serial queue, so on a heavily mixed wallet it stalls every
/// Rust persister callback and the main thread behind them until it finishes:
/// a watchdog kill, and no export artifact, on exactly the wallet support asked
/// about. A fetch limit is the wrong tool because a truncated table silently
/// misclassifies. Counting first and declining above a ceiling keeps the
/// distinction exact wherever it is computable and refuses honestly where it
/// is not. The figures are a cap on materialized objects, not a tuned number.
struct CoreDiagnosticRowLimits: Sendable {
    /// Above this many `PersistentTxo` rows table-wide, only this wallet's rows
    /// are fetched and the cross-wallet audit is declined.
    let crossWalletTxoRows: Int
    /// Above this many `PersistentTransaction` rows table-wide, transaction
    /// bodies are not materialized and the exact audit is declined.
    let exactAuditTransactionRows: Int

    /// The transaction ceiling is the one that matters: `walletOwnsTransaction`
    /// faults four relationships per transaction cross-wallet, each a query
    /// under the coordinator lock, so it — not decoding — dominates the time
    /// the persistence queue is held.
    static let production = CoreDiagnosticRowLimits(
        crossWalletTxoRows: 100_000,
        exactAuditTransactionRows: 10_000
    )
}

private extension Data {
    mutating func appendLittleEndian<T: FixedWidthInteger>(_ value: T) {
        var littleEndian = value.littleEndian
        Swift.withUnsafeBytes(of: &littleEndian) { append(contentsOf: $0) }
    }
}

/// Adds one diagnostic value without allowing corrupt data to trap the
/// exporter. The single saturating rule every diagnostic total shares.
func diagnosticSaturatingAdd(_ partial: UInt64, _ value: UInt64) -> UInt64 {
    let (sum, overflow) = partial.addingReportingOverflow(value)
    return overflow ? UInt64.max : sum
}

/// Adds diagnostic values without allowing corrupt data to trap the exporter.
func diagnosticSaturatingSum<S: Sequence>(_ values: S) -> UInt64
where S.Element == UInt64 {
    values.reduce(0, diagnosticSaturatingAdd)
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
        limits: CoreDiagnosticRowLimits = .production,
        cancellation: CoreDiagnosticsCancellation? = nil
    ) async -> CoreWalletDatabaseDiagnosticSnapshot? {
        await withCheckedContinuation { continuation in
            serialQueue.async { [self] in
                let snapshot = autoreleasepool { () -> CoreWalletDatabaseDiagnosticSnapshot? in
                    // A scratch context, still on the serial queue. Two things
                    // the handler's own context could not give: it sees only
                    // COMMITTED state — a Rust `store()` round is one changeset
                    // spread across several separate `sync` blocks, and this
                    // block can land between two of them, where the handler's
                    // context holds pending rows that `endChangeset` may still
                    // roll back — and it is dropped with this block, so the up
                    // to ~110k objects the pass registers do not stay resident
                    // for the life of the process. The queue still guarantees
                    // no save lands mid-pass.
                    let context = ModelContext(modelContainer)
                    context.autosaveEnabled = false
                    return emitCoreWalletDatabaseDiagnosticsOnQueue(
                        walletId: walletId,
                        context: context,
                        limits: limits,
                        cancellation: cancellation
                    )
                }
                continuation.resume(returning: snapshot)
            }
        }
    }

    /// Queue-confined implementation behind the async export API. Callers must
    /// already own `serialQueue` and hand in a context confined to it; it
    /// performs the full exact audit whenever the tables fit under `limits`
    /// and returns only Sendable value copies. Between its stages it asks
    /// `cancellation` whether shutdown has begun and, if so, says what it
    /// skipped and stops — the drain in `shutdown()` covers this pass, and
    /// must never wait for a whole cross-wallet scan.
    @discardableResult
    func emitCoreWalletDatabaseDiagnosticsOnQueue(
        walletId: Data,
        context: ModelContext,
        limits: CoreDiagnosticRowLimits = .production,
        cancellation: CoreDiagnosticsCancellation? = nil
    ) -> CoreWalletDatabaseDiagnosticSnapshot? {
        // This whole pass is export-only: the launch restore path takes
        // `logCoreRestoreBufferSnapshotOnQueue` and never comes here, so the
        // checkpoint every event below carries is a constant, not a parameter.
        let checkpoint = CoreWalletDiagnosticCheckpoint.preExport
        func shutdownBegan(before stage: String) -> Bool {
            guard let cancellation, cancellation.isCancelled else { return false }
            SDKLogger.event(
                "core_diagnostics_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("shutdown_requested"),
                    "skipped_from_stage": .publicText(stage),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return true
        }
        do {
            let walletDescriptor = FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletId: walletId)
            )
            guard let wallet = try context.fetch(walletDescriptor).first else {
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
            // `missing_txo`. Count before materializing (see
            // `CoreDiagnosticRowLimits`): under the ceiling the whole table is
            // held and the audit is exact; over it only this wallet's rows are
            // fetched, and the snapshot says so, because a relationship-only
            // row or a cross-wallet duplicate is then invisible to it. A
            // streaming pass would lift the ceiling without losing the
            // distinction and remains the follow-up.
            if shutdownBegan(before: "txo_fetch") { return nil }
            let txoRowCount = try context.fetchCount(FetchDescriptor<PersistentTxo>())
            let crossWalletTxoScan = txoRowCount <= limits.crossWalletTxoRows
            let allTxos: [PersistentTxo]
            if crossWalletTxoScan {
                allTxos = try context.fetch(FetchDescriptor<PersistentTxo>())
            } else {
                allTxos = try context.fetch(FetchDescriptor<PersistentTxo>(
                    predicate: #Predicate { $0.walletId == walletId }
                ))
            }
            let walletTxos = allTxos.filter {
                $0.walletId == walletId || Self.relationshipWalletId(of: $0) == walletId
            }
            if shutdownBegan(before: "transaction_fetch") { return nil }
            // Walking every transaction relationship is deliberately export-only.
            // A heavily mixed wallet can have enough history for this traversal to
            // stall restore, which is precisely the failure this instrumentation is
            // intended to diagnose rather than reproduce.
            let allTransactions: [PersistentTransaction]?
            let walletTransactions: [PersistentTransaction]?
            do {
                let transactionRowCount = try context.fetchCount(
                    FetchDescriptor<PersistentTransaction>()
                )
                // Both tables must fit: the audit resolves each decoded
                // output against `txoByOutpoint`, so a wallet-only TXO
                // scan would turn every foreign row into `missing_txo`.
                if crossWalletTxoScan,
                   transactionRowCount <= limits.exactAuditTransactionRows {
                    allTransactions = try context.fetch(
                        FetchDescriptor<PersistentTransaction>()
                    )
                    // Through the accounts' inverse relationship, not
                    // `walletOwnsTransaction` over every row: that faults four
                    // relationships per transaction and dominated the time the
                    // queue is held, to feed two counts. This is the
                    // `involvedAccounts` route only — a row tied to the wallet
                    // solely through a TXO is not counted, which the field
                    // names (`involved_…`) say.
                    var seen = Set<ObjectIdentifier>()
                    walletTransactions = wallet.accounts
                        .flatMap(\.involvedTransactions)
                        .filter { seen.insert(ObjectIdentifier($0)).inserted }
                } else {
                    allTransactions = nil
                    walletTransactions = nil
                    SDKLogger.event(
                        "core_owned_output_audit_summary",
                        category: .persistence,
                        severity: .warning,
                        fields: [
                            "audit_incomplete": .boolean(true),
                            "checkpoint": .publicText(checkpoint.rawValue),
                            "reason": .publicText("tables_too_large_for_exact_audit"),
                            "transaction_row_count": .integer(Int64(transactionRowCount)),
                            "transaction_row_limit": .integer(
                                Int64(limits.exactAuditTransactionRows)
                            ),
                            "txo_row_count": .integer(Int64(txoRowCount)),
                            "txo_row_limit": .integer(Int64(limits.crossWalletTxoRows)),
                            "wallet_reference": .reference(walletId),
                        ]
                    )
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
            let pending: [PersistentPendingInput]?
            do {
                pending = try context.fetch(
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
            // Every stage from here on is O(rows) or O(accounts × rows) on its
            // own, so the drain's documented "at most one stage in flight" only
            // holds if each is gated. The cost is one atomic read per stage.
            if shutdownBegan(before: "wallet_fingerprint") { return nil }
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
                    "involved_transaction_count": .integer(
                        walletTransactions.map { Int64($0.count) } ?? -1
                    ),
                    "transaction_scan_available": .boolean(walletTransactions != nil),
                    "txo_count": .integer(Int64(walletTxos.count)),
                    "txo_fingerprint": .reference(txoFingerprint),
                    "txo_row_count": .integer(Int64(txoRowCount)),
                    "txo_scan_scope": .publicText(
                        crossWalletTxoScan ? "cross_wallet" : "wallet_id_only"
                    ),
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

            let sortedAccounts = wallet.accounts.sorted(by: Self.accountOrder)
            // One pass over the wallet's rows, grouped by account, instead of an
            // identity filter per account (O(accounts × rows)) followed by five
            // more filters per account — all on the held persistence queue.
            struct AccountTally {
                var spentCount = 0, spentValue: UInt64 = 0
                var unspentCount = 0, unspentValue: UInt64 = 0
                var confirmedCount = 0, confirmedValue: UInt64 = 0
                var unconfirmedCount = 0, unconfirmedValue: UInt64 = 0
                var lockedCount = 0, lockedValue: UInt64 = 0
                var fingerprintMaterial: [Data] = []
            }
            var tallies: [ObjectIdentifier: AccountTally] = [:]
            var accountKeys: [ObjectIdentifier: CoreWalletDatabaseDiagnosticSnapshot.AccountKey] = [:]
            for txo in walletTxos {
                // Rows without an account are `missing_account` anomalies,
                // reported by `logTxoAnomalies`; no account snapshot owns them.
                guard let account = txo.account else { continue }
                let id = ObjectIdentifier(account)
                let key: CoreWalletDatabaseDiagnosticSnapshot.AccountKey
                if let known = accountKeys[id] {
                    key = known
                } else {
                    key = Self.diagnosticAccountKey(account)!
                    accountKeys[id] = key
                }
                var tally = tallies[id, default: AccountTally()]
                if txo.isSpent {
                    tally.spentCount += 1
                    tally.spentValue = diagnosticSaturatingAdd(tally.spentValue, txo.amount)
                } else {
                    tally.unspentCount += 1
                    tally.unspentValue = diagnosticSaturatingAdd(tally.unspentValue, txo.amount)
                }
                if txo.isConfirmed {
                    tally.confirmedCount += 1
                    tally.confirmedValue = diagnosticSaturatingAdd(tally.confirmedValue, txo.amount)
                } else {
                    tally.unconfirmedCount += 1
                    tally.unconfirmedValue = diagnosticSaturatingAdd(tally.unconfirmedValue, txo.amount)
                }
                if txo.isLocked {
                    tally.lockedCount += 1
                    tally.lockedValue = diagnosticSaturatingAdd(tally.lockedValue, txo.amount)
                }
                tally.fingerprintMaterial.append(diagnosticTxoFingerprint(
                    outpoint: txo.outpoint,
                    amount: txo.amount,
                    height: txo.height,
                    scriptPubKey: txo.scriptPubKey,
                    isLocked: txo.isLocked,
                    account: key
                ))
                tallies[id] = tally
            }

            if shutdownBegan(before: "account_snapshots") { return nil }
            for account in sortedAccounts {
                let key = Self.diagnosticAccountKey(account)!
                let tally = tallies[ObjectIdentifier(account)] ?? AccountTally()
                var externalAddressCount = 0
                var internalAddressCount = 0
                var usedAddressCount = 0
                for address in account.coreAddresses {
                    if address.poolTypeTag == 0 { externalAddressCount += 1 }
                    if address.poolTypeTag == 1 { internalAddressCount += 1 }
                    if address.isUsed { usedAddressCount += 1 }
                }
                SDKLogger.event(
                    "core_db_account_snapshot",
                    category: .persistence,
                    fields: [
                        "account_index": .unsignedInteger(UInt64(account.accountIndex)),
                        "account_reference": .reference(key.referenceMaterial),
                        "account_type": .unsignedInteger(UInt64(account.accountType)),
                        "checkpoint": .publicText(checkpoint.rawValue),
                        "confirmed_count": .integer(Int64(tally.confirmedCount)),
                        "confirmed_value_duffs": .unsignedInteger(tally.confirmedValue),
                        "external_address_count": .integer(Int64(externalAddressCount)),
                        "external_highest_used": .integer(Int64(account.externalHighestUsed)),
                        "internal_address_count": .integer(Int64(internalAddressCount)),
                        "internal_highest_used": .integer(Int64(account.internalHighestUsed)),
                        "locked_count": .integer(Int64(tally.lockedCount)),
                        "locked_value_duffs": .unsignedInteger(tally.lockedValue),
                        "registration_index": .unsignedInteger(UInt64(account.registrationIndex)),
                        "spent_count": .integer(Int64(tally.spentCount)),
                        "spent_value_duffs": .unsignedInteger(tally.spentValue),
                        "standard_tag": .unsignedInteger(UInt64(account.standardTag)),
                        "txo_fingerprint": .reference(diagnosticFingerprint(tally.fingerprintMaterial)),
                        "unconfirmed_count": .integer(Int64(tally.unconfirmedCount)),
                        "unconfirmed_value_duffs": .unsignedInteger(tally.unconfirmedValue),
                        "unspent_count": .integer(Int64(tally.unspentCount)),
                        "unspent_value_duffs": .unsignedInteger(tally.unspentValue),
                        "used_address_count": .integer(Int64(usedAddressCount)),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }

            // Faults `transaction`, `account.wallet` and `spendingTransaction`
            // for every row it inspects — the single most expensive stage after
            // the audit itself.
            if shutdownBegan(before: "txo_anomalies") { return nil }
            Self.logTxoAnomalies(
                walletId: walletId,
                checkpoint: checkpoint,
                txos: walletTxos
            )
            // Decoding a heavily mixed wallet's full transaction history can
            // be expensive. The exact #4438 audit is needed for the manually
            // exported artifact, not for restoring Rust, so keep startup's
            // persistence queue limited to lightweight summaries.
            // Not nested inside `if let allTransactions`: the stages that
            // follow run whether or not the audit was declined for table size,
            // so gating the check on the audit's input skipped it exactly when
            // the pass was already refusing to do the cheap thing.
            if shutdownBegan(before: "owned_output_audit") {
                if let allTransactions {
                    SDKLogger.event(
                        "core_owned_output_audit_summary",
                        category: .persistence,
                        severity: .warning,
                        fields: [
                            "audit_incomplete": .boolean(true),
                            "checkpoint": .publicText(checkpoint.rawValue),
                            "reason": .publicText("shutdown_requested"),
                            "transaction_row_count": .integer(Int64(allTransactions.count)),
                            "wallet_reference": .reference(walletId),
                        ]
                    )
                }
                return nil
            }
            if let allTransactions {
                let gaveUp = Self.auditCoinJoinOwnedBip44Outputs(
                    wallet: wallet,
                    walletId: walletId,
                    checkpoint: checkpoint,
                    allTxos: allTxos,
                    allTransactions: allTransactions,
                    isCancelled: { cancellation?.isCancelled == true }
                )
                // It stopped mid-loop rather than between stages, so the rest
                // of the pass is abandoned the same way every other stage
                // abandons it — the summary it did not emit is the signal.
                if gaveUp, shutdownBegan(before: "owned_output_audit_interrupted") {
                    return nil
                }
            }

            if shutdownBegan(before: "asset_lock_snapshot") { return nil }
            let assetLocks: [CoreWalletDatabaseDiagnosticSnapshot.AssetLock]
            let assetLocksAvailable: Bool
            do {
                assetLocks = try Self.logAssetLockDatabaseSnapshot(
                    context: context,
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
            if shutdownBegan(before: "shielded_snapshot") { return nil }
            do {
                try Self.logShieldedStoreSnapshot(
                    context: context,
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
    ///
    /// - Parameters:
    ///   - rows: the rows the restore buffer was built from, in build order.
    ///   - accountLessRows: rows the restore fetch matched to this wallet by its
    ///     denormalized id but which carry no account, so they never reached FFI
    ///     marshalling. Passed separately rather than merged upstream: they are
    ///     rare, and keeping them out of `rows` avoids a second full-length copy
    ///     of every unspent row on the launch path.
    func logCoreRestoreBufferSnapshotOnQueue(
        walletId: Data,
        rows: [PersistentTxo],
        accountLessRows: [PersistentTxo] = [],
        emittedCount: Int,
        errored: Bool
    ) {
        func candidate(_ row: PersistentTxo)
        -> CoreWalletDiagnosticAnalyzer.RestoreCandidate {
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
        //
        // It also means classifying the rows would cost more than it is worth:
        // `buildUtxoRestoreBuffer` can bail at row 0 having faulted nothing,
        // and `candidate` touches `account` and `txid` — to-one relationships
        // — on every row, so the walk would issue a fault per row, at launch,
        // with the persistence queue held, to describe a load that is about to
        // be discarded. The row that failed is already named with its reason
        // by `persistence_wallet_load_validation_failed`; here the count is
        // what is left to say, and the positional emission window the walk
        // exists for is moot at zero emitted.
        let summary: CoreWalletDiagnosticAnalyzer.RestoreBufferSummary
        if errored {
            summary = CoreWalletDiagnosticAnalyzer.summarizeRestoreBuffer(
                candidates: EmptyCollection<
                    CoreWalletDiagnosticAnalyzer.RestoreCandidate
                >(),
                emittedCount: 0,
                errored: true,
                candidateCountOverride: rows.count + accountLessRows.count
            )
        } else {
            // Lazily, and with the built rows first: the summary's emission
            // window is positional, so the rejected rows must not shift it.
            // Nothing here materializes a per-row array — this runs while the
            // launch restore holds the persistence queue, and every row it
            // touches was already faulted by the build it is reconciling.
            summary = CoreWalletDiagnosticAnalyzer.summarizeRestoreBuffer(
                candidates: [rows, accountLessRows].lazy.flatMap { $0 }.map(candidate),
                emittedCount: emittedCount,
                errored: false
            )
        }
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
                "emitted_count": .integer(Int64(summary.emittedCount)),
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

    /// The one ordering every per-account pass uses, so anything that picks
    /// "the first account" picks the same one on every export.
    private static func accountOrder(_ lhs: PersistentAccount, _ rhs: PersistentAccount) -> Bool {
        (lhs.accountType, lhs.standardTag, lhs.accountIndex, lhs.registrationIndex, lhs.keyClass)
            < (rhs.accountType, rhs.standardTag, rhs.accountIndex, rhs.registrationIndex, rhs.keyClass)
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
                hasSpendingTransaction: txo.spendingTransaction != nil,
                spendingTransactionIsInBlock: txo.spendingTransaction.map(spendIsInBlock)
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
                        + result.count(reason: "unspent_with_confirmed_spending_transaction")
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
    ///
    /// Ownership is decided by the persisted `PersistentCoreAddress` pool, so
    /// `coinjoin_to_bip44_missing_count == 0` proves only that every output the
    /// audit could attribute is persisted. An output beyond the derived pool,
    /// or one whose address row was never written, is attributable to nobody
    /// and lands in `unattributed_output_count` instead — which is why the
    /// summary reports that counter and the pool size next to the verdict.
    /// - Parameter isCancelled: polled inside the long loops, not only around
    ///   them. This is the most expensive stage in the pass — up to
    ///   `exactAuditTransactionRows` decodes plus per-output relationship work
    ///   — so a `shutdown()` that arrives mid-loop must not wait for the whole
    ///   of it. Returns true if the audit gave up, and the caller says so.
    private static func auditCoinJoinOwnedBip44Outputs(
        wallet: PersistentWallet,
        walletId: Data,
        checkpoint: CoreWalletDiagnosticCheckpoint,
        allTxos: [PersistentTxo],
        allTransactions: [PersistentTransaction],
        isCancelled: () -> Bool
    ) -> Bool {
        // Match the wallet exactly as `walletTxos` does. Accepting only the
        // relationship would drop a CoinJoin row whose `account.wallet` link is
        // broken — the very corruption this audit exists to expose — and its
        // spending transaction would never even become a candidate.
        let coinJoinOutpoints = Set(allTxos.compactMap { txo -> Data? in
            guard txo.account?.accountType == 1,
                  txo.walletId == walletId || relationshipWalletId(of: txo) == walletId
            else { return nil }
            return txo.outpoint
        })
        // `wallet.accounts` is unordered. Two BIP44 accounts holding a row for
        // the same address would otherwise make `expectedAccount` — and so
        // `wrong_account` — depend on which faulted first; the same ordering
        // the account snapshots use makes the winner the same on every run.
        //
        // Both kinds of account we can own an output on. A mixed send consumes
        // a CoinJoin output and typically pays CoinJoin change back to the
        // wallet: that output has a persisted address row, so booking it as
        // unattributed both overstated "paid to someone else" and hid the case
        // where it is the CoinJoin-side output that went missing from
        // `PersistentTxo` — a third route to the false all-clear.
        //
        // BIP44 first, so a shared address keeps the BIP44 attribution this
        // audit has always given it.
        var ownedAddresses: [String: PersistentAccount] = [:]
        let bip44Accounts = wallet.accounts
            .filter { $0.accountType == 0 && $0.standardTag == 0 }
            .sorted(by: Self.accountOrder)
        let coinJoinAccounts = wallet.accounts
            .filter { $0.accountType == 1 }
            .sorted(by: Self.accountOrder)
        for account in bip44Accounts + coinJoinAccounts {
            for coreAddress in account.coreAddresses where ownedAddresses[coreAddress.address] == nil {
                ownedAddresses[coreAddress.address] = account
            }
        }
        let bip44AddressCount = bip44Accounts.reduce(0) { $0 + $1.coreAddresses.count }
        let coinJoinAddressCount = coinJoinAccounts.reduce(0) { $0 + $1.coreAddresses.count }
        let txoByOutpoint = Dictionary(grouping: allTxos, by: \.outpoint)

        var candidateCount = 0
        var decodeFailureCount = 0
        var transactionBytesMissingCount = 0
        var ownedOutputCount = 0
        var ownedOutputValue: UInt64 = 0
        var ownedCoinJoinOutputCount = 0
        var ownedCoinJoinOutputValue: UInt64 = 0
        var unattributedOutputCount = 0
        var undecodableAddressOutputCount = 0
        var validCount = 0
        var anomalies: [(tx: PersistentTransaction, vout: UInt32, amount: UInt64,
                         outpoint: Data, reason: String, outputIsCoinJoin: Bool)] = []

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
            return false
        }

        for transaction in allTransactions {
            if isCancelled() { return true }
            // A stub row with no consensus bytes is a real production state
            // (an orphaned upsert reads back as empty) — and the half-written
            // persistence #4438 is about. It cannot be decoded, so it cannot
            // become a candidate; it must be counted rather than skipped, or
            // the export reports a clean wallet with `audit_incomplete=false`.
            // Stubs are rare, so the ownership check is cheap here even though
            // it is the expensive one.
            if transaction.transactionData.isEmpty {
                if walletOwnsTransaction(walletId: walletId, transaction: transaction) {
                    transactionBytesMissingCount += 1
                }
                continue
            }
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
                if isCancelled() { return true }
                guard let address = output.address else {
                    // Non-P2PKH/P2SH scriptPubKey: nothing to match against the
                    // address pool, so it is unclassified rather than foreign.
                    undecodableAddressOutputCount += 1
                    continue
                }
                guard let expectedAccount = ownedAddresses[address] else {
                    // Either a genuine payment to someone else or one of our
                    // own change addresses with no persisted row. The audit
                    // cannot tell them apart, so it counts rather than clears.
                    unattributedOutputCount += 1
                    continue
                }
                let outputIsCoinJoin = expectedAccount.accountType == 1
                if outputIsCoinJoin {
                    ownedCoinJoinOutputCount += 1
                    ownedCoinJoinOutputValue = diagnosticSaturatingAdd(
                        ownedCoinJoinOutputValue, output.valueDuffs)
                } else {
                    ownedOutputCount += 1
                    ownedOutputValue = diagnosticSaturatingAdd(ownedOutputValue, output.valueDuffs)
                }
                let vout = UInt32(index)
                let outpoint = PersistentTxo.makeOutpoint(txid: decoded.txid, vout: vout)
                guard let row = representativeTxo(
                    rows: txoByOutpoint[outpoint],
                    walletId: walletId
                ) else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "missing_txo", outputIsCoinJoin))
                    continue
                }
                // Same admission rule as `walletTxos` and the candidate set:
                // the denormalized id OR the relationship may name this
                // wallet. Judging by the relationship alone here would report
                // this wallet's own row with a broken link as `wrong_wallet`.
                let rowRelationshipWallet = relationshipWalletId(of: row)
                let denormalizedNamesWallet = row.walletId == walletId
                let relationshipNamesWallet = rowRelationshipWallet == walletId
                guard denormalizedNamesWallet || relationshipNamesWallet else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "wrong_wallet", outputIsCoinJoin))
                    continue
                }
                guard relationshipNamesWallet else {
                    // Ours by id, but the relationship says otherwise. Reuse
                    // the vocabulary `logTxoAnomalies` already emits for the
                    // same two facts, so the analyst sees one story.
                    let reason = rowRelationshipWallet == nil
                        ? "relationship_missing" : "wallet_id_mismatch"
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, reason, outputIsCoinJoin))
                    continue
                }
                // Against the account the address pool named, not a hardcoded
                // BIP44 shape: the same check now serves CoinJoin-owned
                // outputs, whose account carries type 1.
                guard row.account === expectedAccount,
                      row.account?.accountType == expectedAccount.accountType,
                      row.account?.standardTag == expectedAccount.standardTag
                else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "wrong_account", outputIsCoinJoin))
                    continue
                }
                guard row.amount == output.valueDuffs else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "amount_mismatch", outputIsCoinJoin))
                    continue
                }
                guard row.scriptPubKey == output.scriptPubkey else {
                    anomalies.append((transaction, vout, output.valueDuffs, outpoint, "script_mismatch", outputIsCoinJoin))
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
        let missing = anomalies.filter { $0.reason == "missing_txo" }
        let missingCount = missing.filter { !$0.outputIsCoinJoin }.count
        let missingValue = diagnosticSaturatingSum(missing.compactMap {
            $0.outputIsCoinJoin ? nil : $0.amount
        })
        let missingCoinJoinCount = missing.filter(\.outputIsCoinJoin).count
        let missingCoinJoinValue = diagnosticSaturatingSum(missing.compactMap {
            $0.outputIsCoinJoin ? $0.amount : nil
        })
        // With no persisted BIP44 addresses nothing can be attributed to this
        // wallet: every decoded output falls into `unattributedOutputCount`,
        // no output reaches the `missing_txo` check, and the summary would
        // otherwise read as a clean, complete audit — on a wallet whose
        // address rows are exactly what went missing. The pool being empty is
        // an incompleteness of the same kind as an undecodable transaction.
        //
        // `unattributedOutputCount > 0` is deliberately NOT part of this: a
        // CoinJoin-spending transaction pays its peers, and their outputs are
        // unattributable by construction, so every healthy audit has some.
        // Since the pool now covers CoinJoin accounts too, that count is
        // finally only peers and rows we genuinely lack — our own CoinJoin
        // change no longer inflates it.
        // A partially lost pool is not distinguishable from a small one here;
        // `bip44_address_pool_size` sits beside this flag for that reading.
        // Both pools, now that both are audited. A wallet always has BIP44
        // accounts, so an empty BIP44 pool is unconditionally wrong; CoinJoin
        // accounts only exist on a mixed wallet, so an empty CoinJoin pool is
        // only evidence when there are accounts that should have filled it.
        // Without the second clause, widening `ownedAddresses` to CoinJoin
        // would have added a fourth route to the false all-clear: a lost
        // CoinJoin pool, on the mixed wallet this audit is written for.
        let addressPoolEmpty = bip44AddressCount == 0
            || (!coinJoinAccounts.isEmpty && coinJoinAddressCount == 0)
        let auditIncomplete = decodeFailureCount > 0
            || transactionBytesMissingCount > 0
            || addressPoolEmpty
        SDKLogger.event(
            "core_owned_output_audit_summary",
            category: .persistence,
            severity: anomalies.isEmpty && !auditIncomplete ? .info : .warning,
            fields: [
                "audit_incomplete": .boolean(auditIncomplete),
                "bip44_address_pool_empty": .boolean(addressPoolEmpty),
                "bip44_address_pool_size": .integer(Int64(bip44AddressCount)),
                "coinjoin_address_pool_size": .integer(Int64(coinJoinAddressCount)),
                "coinjoin_to_coinjoin_missing_count": .integer(Int64(missingCoinJoinCount)),
                "coinjoin_to_coinjoin_missing_value_duffs": .unsignedInteger(missingCoinJoinValue),
                "candidate_transaction_count": .integer(Int64(candidateCount)),
                "checkpoint": .publicText(checkpoint.rawValue),
                "coinjoin_to_bip44_missing_count": .integer(Int64(missingCount)),
                "coinjoin_to_bip44_missing_value_duffs": .unsignedInteger(missingValue),
                "decode_failure_count": .integer(Int64(decodeFailureCount)),
                "output_address_undecodable_count": .integer(
                    Int64(undecodableAddressOutputCount)
                ),
                "owned_bip44_output_count": .integer(Int64(ownedOutputCount)),
                "owned_bip44_output_value_duffs": .unsignedInteger(ownedOutputValue),
                "owned_coinjoin_output_count": .integer(Int64(ownedCoinJoinOutputCount)),
                "owned_coinjoin_output_value_duffs": .unsignedInteger(ownedCoinJoinOutputValue),
                "persisted_valid_count": .integer(Int64(validCount)),
                "total_anomaly_count": .integer(Int64(anomalies.count)),
                "transaction_bytes_missing_count": .integer(Int64(transactionBytesMissingCount)),
                "truncated_count": .integer(Int64(truncatedAnomalyCount)),
                "unattributed_output_count": .integer(Int64(unattributedOutputCount)),
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
                        "output_account_kind": .publicText(
                            anomaly.outputIsCoinJoin ? "coinjoin" : "bip44"
                        ),
                        "reason": .publicText(reason),
                        "transaction_context": .unsignedInteger(UInt64(anomaly.tx.context)),
                        "transaction_reference": .reference(anomaly.tx.txid),
                        "vout": .unsignedInteger(UInt64(anomaly.vout)),
                        "wallet_reference": .reference(walletId),
                    ]
                )
            }
        }
        return false
    }

    /// Picks the row that represents one outpoint when the table holds more
    /// than one.
    ///
    /// A duplicated outpoint split across wallets is precisely the `wrong_wallet`
    /// corruption this audit names, so the choice must not depend on SwiftData's
    /// fetch order — the same database would otherwise report `wrong_wallet` on
    /// one run and a clean count on the next. A row this wallet owns wins (the
    /// output IS persisted here, whatever else shares the outpoint); otherwise a
    /// deterministic representative is chosen the way `compareTxos` resolves
    /// duplicates before comparing.
    static func representativeTxo(
        rows: [PersistentTxo]?,
        walletId: Data
    ) -> PersistentTxo? {
        guard let rows, !rows.isEmpty else { return nil }
        if rows.count == 1 { return rows[0] }
        let ordered = rows.sorted {
            duplicateResolutionKey($0).lexicographicallyPrecedes(
                duplicateResolutionKey($1)
            )
        }
        return ordered.first {
            $0.walletId == walletId || relationshipWalletId(of: $0) == walletId
        } ?? ordered[0]
    }

    /// Total order over rows sharing an outpoint. Uses only persisted bytes, so
    /// two runs over the same database agree.
    ///
    /// A single `0` byte separates the two wallet ids. That is unambiguous only
    /// because each is exactly 32 bytes or empty — a variable-width id would
    /// need length prefixes, as `diagnosticTxoFingerprint` uses.
    private static func duplicateResolutionKey(_ txo: PersistentTxo) -> Data {
        var key = Data()
        key.append(txo.walletId)
        key.append(0)
        key.append(relationshipWalletId(of: txo) ?? Data())
        key.append(0)
        withUnsafeBytes(of: txo.amount.littleEndian) { key.append(contentsOf: $0) }
        // Two rows ours by id with the same amount and script but different
        // accounts (BIP44 vs CoinJoin) must not tie: `sorted` is not stable,
        // so a tie would make `wrong_account` depend on fetch order.
        if let account = diagnosticAccountKey(txo.account) {
            key.append(1)
            key.append(account.referenceMaterial)
        } else {
            key.append(0)
        }
        key.append(0)
        key.append(txo.scriptPubKey)
        return key
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
                "involved_type_8_transaction_count": .integer(
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
    ///
    /// Cost, so hosts do not trigger this mid-sync: the SwiftData half runs on
    /// the persistence serial queue and holds it for its whole duration, which
    /// blocks every Rust persister and SPV callback (they enter through
    /// `serialQueue.sync`) and any main-thread persistence access until it
    /// returns. Under `CoreDiagnosticRowLimits` that includes materializing
    /// the full TXO and transaction tables for the exact #4438 audit; above
    /// them the export narrows to this wallet's rows, declines the audit, and
    /// says so in `core_owned_output_audit_summary`. A paged variant that
    /// lifts the ceilings without losing the classification is tracked as a
    /// follow-up.
    ///
    /// Coordinates the queue-owned SwiftData snapshot with read-only Rust FFI
    /// queries. Admission happens after the database await, then keeps the
    /// native handle alive until the off-main worker finishes.
    ///
    /// No caller inside this repository, by design: the artifact this produces
    /// is a support export, and the only screen that asks for one lives in the
    /// host app (dashpay/dashwallet-ios#1105 wires Contact Support to it).
    /// `SwiftExampleApp` deliberately does not — it has no support flow, and a
    /// demo button would make a pass documented as holding the persistence
    /// queue look like something to press casually. Everything under the
    /// `preExport` checkpoint is therefore reachable only through a host; the
    /// `restoreBuffer` summary and `core_store_open_result` are the parts that
    /// run unprompted.
    public func emitCoreWalletDiagnostics(for walletId: Data) async {
        let checkpoint = CoreWalletDiagnosticCheckpoint.preExport
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
        let cancellation = coreDiagnosticsCancellation
        // Counted for the whole pass, both halves, so a `shutdown()` that
        // finds no handle can still tell a running database half to stop —
        // and so one that finds neither leaves the latch down.
        beginCoreDiagnosticsPass()
        defer { endCoreDiagnosticsPass() }

        // The database half may come back empty — wallet row missing, fetch
        // failed — and those are exactly the "coins gone from the database"
        // reports this exists for. The Rust half needs only the wallet id, so
        // it runs regardless and marks its diffs as one-sided.
        //
        // Deliberately OUTSIDE the native-op admission. That admission exists
        // for one thing: keeping `handle` alive while an FFI read is in
        // flight. This half makes no FFI call at all — it reads SwiftData on
        // the persistence serial queue — so covering it buys the handle
        // nothing and costs `shutdown()`'s drain everything: the drain would
        // then wait on a block whose progress depends on that queue, and a
        // wedged persister round (the failure this export exists to
        // investigate) is exactly when the queue does not advance. There is
        // no deadline on the drain, so that wait would be unbounded.
        //
        // Ordering is still safe without the drain, because it does not come
        // from the drain: native teardown runs on `destroyQueue`, and the Rust
        // destroy's persister callbacks enter through `serialQueue.sync`, so
        // they queue BEHIND this block rather than racing it — off the main
        // thread, and bounded by the cancellation flag this block polls
        // between stages. ARC covers the rest: the block holds the handler and
        // its container, so neither can be deallocated under the read.
        let database = await handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            cancellation: cancellation
        )

        // Admission covers the FFI half only, which runs on
        // `coreDiagnosticsQueue` and polls cancellation between reads — so the
        // drain's "at most one stage in flight" is bounded by a stage this
        // manager owns, not by whatever is holding the persistence queue.
        guard isConfigured, handle != NULL_HANDLE else {
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("manager_not_configured"),
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
            Self.coreDiagnosticsQueue.async {
                Self.emitCoreMemoryDiagnostics(
                    managerHandle: managerHandle,
                    managedWallet: managedWallet,
                    walletId: walletId,
                    database: database,
                    checkpoint: checkpoint,
                    cancellation: cancellation
                )
                continuation.resume()
            }
        }
    }

    /// Runs all Rust-memory reads on `coreDiagnosticsQueue`. Each subsystem
    /// reports its own unavailable state so one failed query does not hide the
    /// others.
    private nonisolated static func emitCoreMemoryDiagnostics(
        managerHandle: Handle,
        managedWallet: ManagedPlatformWallet?,
        walletId: Data,
        database: CoreWalletDatabaseDiagnosticSnapshot?,
        checkpoint: CoreWalletDiagnosticCheckpoint,
        cancellation: CoreDiagnosticsCancellation
    ) {
        // Each FFI read below can park this thread on a Rust lock for as long
        // as a wedged sync pass holds it, and `shutdown()` waits on this
        // pass's admission. So before every read, ask whether shutdown has
        // begun; if so, say which reads were skipped and let the drain go.
        func shutdownBegan(before stage: String) -> Bool {
            guard cancellation.isCancelled else { return false }
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("shutdown_requested"),
                    "skipped_from_stage": .publicText(stage),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return true
        }

        // `compareDatabase` and `compareAssetLocks` both emit a
        // `diff_incomplete=true` summary rather than nothing when their input
        // is missing, because an absent summary is indistinguishable from a
        // log that was cut off mid-export. Every early return out of this
        // function owes the reader the same line — otherwise grepping
        // `core_db_memory_diff_summary` on a failed balance read finds
        // silence, which reads as truncation.
        func emitAbandonedDiffSummary(reason: String) {
            SDKLogger.event(
                "core_db_memory_diff_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "database_snapshot_available": .boolean(database != nil),
                    "diff_incomplete": .boolean(true),
                    "reason": .publicText(reason),
                    "wallet_reference": .reference(walletId),
                ]
            )
        }

        // Keep the two Rust-memory sources independent: corrupt account state
        // must not suppress the AssetLock evidence that can explain a missing
        // balance (and vice versa).
        if shutdownBegan(before: "asset_locks") {
            emitAbandonedDiffSummary(reason: "shutdown_requested")
            return
        }
        compareAssetLocks(
            database,
            walletId: walletId,
            managedWallet: managedWallet,
            checkpoint: checkpoint
        )
        if shutdownBegan(before: "account_balances") {
            emitAbandonedDiffSummary(reason: "shutdown_requested")
            return
        }
        let balanceQuery = readAccountBalances(
            handle: managerHandle,
            walletId: walletId
        )
        guard case .success(let balances) = balanceQuery else {
            SDKLogger.event(
                "core_memory_snapshot_unavailable",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "reason": .publicText("account_balance_query_failed"),
                    "wallet_reference": .reference(walletId),
                ]
            )
            emitAbandonedDiffSummary(reason: "account_balance_query_failed")
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
            if shutdownBegan(before: "account_utxos") {
                emitAbandonedDiffSummary(reason: "shutdown_requested")
                return
            }
            // One pool per account, matching `emitCoreWalletDatabaseDiagnostics`.
            // libdispatch drains its own pool once per work item, and this
            // whole loop is one work item: without this, every account's
            // per-UTXO txid and scriptPubKey copies, its fingerprint material
            // and the formatter each log event allocates all stay resident
            // until the export ends, so the peak is the sum of every account
            // rather than the largest one. `memoryTxos` is returned out of the
            // pool on purpose — `compareDatabase` needs the whole set.
            let accountTxos: [CoreWalletDatabaseDiagnosticSnapshot.Txo]? = autoreleasepool {
                let key = Self.diagnosticAccountKey(balance)
                let query = diagnosticAccountUtxos(
                    managerHandle: managerHandle,
                    walletId: walletId,
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
                            "wallet_reference": .reference(walletId),
                        ]
                    )
                    return nil
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
                        "wallet_reference": .reference(walletId),
                    ]
                )
                return utxos
            }
            if let accountTxos { memoryTxos.append(contentsOf: accountTxos) }
        }
        compareDatabase(
            database,
            walletId: walletId,
            memoryTxos: memoryTxos,
            memoryAccounts: Set(balances.map(Self.diagnosticAccountKey)),
            unavailableAccounts: unavailableAccounts,
            checkpoint: checkpoint
        )
    }

    /// Logs the deterministic DB↔Rust UTXO diff, excluding accounts whose Rust
    /// UTXO query failed instead of falsely reporting all their rows DB-only.
    private nonisolated static func compareDatabase(
        _ database: CoreWalletDatabaseDiagnosticSnapshot?,
        walletId: Data,
        memoryTxos: [CoreWalletDatabaseDiagnosticSnapshot.Txo],
        memoryAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>,
        unavailableAccounts: Set<CoreWalletDatabaseDiagnosticSnapshot.AccountKey>,
        checkpoint: CoreWalletDiagnosticCheckpoint
    ) {
        guard let database else {
            // No database side to diff against: say so, with the memory side's
            // size, rather than emit nothing — an absent summary reads like a
            // truncated log, and this is the case where Rust may still hold
            // the funds the database lost.
            SDKLogger.event(
                "core_db_memory_diff_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "database_snapshot_available": .boolean(false),
                    "diff_incomplete": .boolean(true),
                    "memory_account_count": .integer(Int64(memoryAccounts.count)),
                    "memory_txo_count": .integer(Int64(memoryTxos.count)),
                    "unavailable_account_count": .integer(Int64(unavailableAccounts.count)),
                    "wallet_reference": .reference(walletId),
                ]
            )
            return
        }
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
                "database_snapshot_available": .boolean(true),
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
                "wallet_reference": .reference(walletId),
            ]
        )
        for detail in result.emittedDetails {
            logDiffItem(
                walletId,
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
        _ database: CoreWalletDatabaseDiagnosticSnapshot?,
        walletId: Data,
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
                    "wallet_reference": .reference(walletId),
                ]
            )
            // Mirror the database-unavailable path below: an analyst greps for
            // `asset_lock_db_memory_diff_summary`, and a missing line is
            // indistinguishable from a truncated log. Say the diff is
            // incomplete instead of saying nothing.
            SDKLogger.event(
                "asset_lock_db_memory_diff_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "database_query_available": .boolean(database?.assetLocksAvailable ?? false),
                    "database_snapshot_available": .boolean(database != nil),
                    "diff_incomplete": .boolean(true),
                    "memory_query_available": .boolean(false),
                    "mismatch_count": .integer(0),
                    "truncated_count": .integer(0),
                    "wallet_reference": .reference(walletId),
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
                "wallet_reference": .reference(walletId),
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
                    "wallet_reference": .reference(walletId),
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
        guard let database, database.assetLocksAvailable else {
            SDKLogger.event(
                "asset_lock_db_memory_diff_summary",
                category: .persistence,
                severity: .warning,
                fields: [
                    "checkpoint": .publicText(checkpoint.rawValue),
                    "database_query_available": .boolean(false),
                    "database_snapshot_available": .boolean(database != nil),
                    "diff_incomplete": .boolean(true),
                    "memory_query_available": .boolean(true),
                    "mismatch_count": .integer(0),
                    "truncated_count": .integer(0),
                    "wallet_reference": .reference(walletId),
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
                "database_snapshot_available": .boolean(true),
                "diff_incomplete": .boolean(false),
                "memory_query_available": .boolean(true),
                "mismatch_count": .integer(Int64(result.details.count)),
                "truncated_count": .integer(Int64(result.truncatedCount)),
                "wallet_reference": .reference(walletId),
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
                    "wallet_reference": .reference(walletId),
                ]
            )
        }
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

    /// Renders the memory side of the AssetLock diff in the exact format the
    /// database side is keyed by.
    ///
    /// `compareAssetLocks` matches the two sides on this string, so it must go
    /// through `PersistentAssetLock.encodeOutPoint` rather than a second hex
    /// loop: a hand-rolled copy agrees only by coincidence, and any later change
    /// to the canonical encoder would silently make every lock report as both
    /// `database_only` and `memory_only`.
    nonisolated static func assetLockOutpointDisplay(
        txid: Data,
        vout: UInt32
    ) -> String {
        let raw = PersistentTxo.makeOutpoint(txid: txid, vout: vout)
        // `encodeOutPoint` traps on a malformed outpoint. Diagnostics must
        // survive corrupt input, so fall back to a clearly non-matching marker
        // that shows up as `memory_only` instead of taking the process down.
        // Reachable only through a `txid` that is not 32 bytes: `makeOutpoint`
        // appends whatever it is given, and the FFI copies a fixed 32-byte
        // array, so this guards the Rust side's word, not this file's.
        guard raw.count == 36 else {
            return "invalid_outpoint:\(raw.count)_bytes:\(vout)"
        }
        return PersistentAssetLock.encodeOutPoint(rawBytes: raw)
    }
}
