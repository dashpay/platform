import DashSDKFFI
import Foundation

// MARK: - Core TXO store reconcile
//
// Why this exists: the SwiftData TXO store is rebuilt from the engine's
// per-round deltas and is itself the source the engine is restored from
// at every launch (`loadWalletList` hands back every `isSpent == false`
// row). Two failure classes leave the two sides disagreeing after a full
// scan — a coin the engine credited whose row the store never got, and a
// row the store holds unspent for a coin the engine never credited
// (rust-dashcore#992: the spender had no wallet-owned output and was
// discarded before the coin was known). The restore then hands the
// phantom back, and the engine shows a balance it had just corrected
// (dashpay/platform#4575). This pass compares the store with the engine
// and repairs exactly what positive evidence supports:
//
// - insert an engine-held coin the store lacks, when the engine's heal
//   verdict calls it eligible and the store resolves its account;
// - mark a store row spent when the engine says the owning account
//   recorded its funding transaction, owns its script, and does not hold
//   the coin (`CoreOutpointClass.knownUncredited`).
//
// It never deletes a row, never un-marks a spent row, and never acts on
// absence alone. It is idempotent (a consistent store yields a report with
// zero mutations), wallet-scoped, paged in both directions, and stops
// between pages when the manager shuts down or a wallet is deleted.
//
// Every decision is `platform-wallet`'s (`manager::txo_reconcile`, through
// the reconcile C API) — the same answers the Kotlin SDK gets: which
// wallets are due and when, whether a run may start (one per wallet, SPV
// steady with a scan tip, no latched sync fault, the wallet scanned to the
// tip), which engine coins may be healed, which store rows are proven
// spent, and how long a stale page is retried. This file keeps only what
// needs the store: paging its rows, resolving the account that owns each,
// its writes, and the fences that keep Rust's persistence rounds and
// teardown out of a run's writes.
//
// Lock order, the engine's own: the native handle registry (read only
// inside one native call, to resolve the manager), the engine's wallet lock
// (held for reading by a page's classification until its apply is
// written), the persistence queue, the SwiftData save. The engine calls the
// persistence callbacks — which run on that queue — while it holds its
// wallet lock for writing, so a classification is asked for off the
// persistence queue, never from a callback, and its guard is released after
// the apply has left the queue and before anything resolves the manager
// again: the next page's classification or the run's finish.

extension PlatformWalletManager {
    /// Rows per engine page / per store page.
    nonisolated static let coreTxoReconcilePageSize = 512
    /// How many times a store step is deferred behind an open Rust
    /// persistence round before the run gives up (each deferral waits
    /// `coreTxoReconcileRetryDelay`). The store's own fence: saving
    /// mid-round would commit the round's staged rows early. Never waited
    /// on while the run holds an engine guard.
    nonisolated static let coreTxoReconcileMaxRetries = 200
    nonisolated static let coreTxoReconcileRetryDelay: TimeInterval = 0.05

    /// Reconcile the SwiftData TXO store of `walletId` against the engine —
    /// see the file comment for what it repairs and what it refuses to
    /// touch.
    ///
    /// Whether the run may start is `platform-wallet`'s start gate
    /// (`platform_wallet_manager_txo_reconcile_begin`): one run per wallet
    /// at a time, SPV in steady state with a scan tip, no latched sync
    /// fault, and the wallet's own scan watermark, as the engine holds it,
    /// within the engine's margin of the tip. Each refusal is a `.skipped`
    /// outcome rather than an error; so are an unconfigured or shutting-down
    /// manager and a wallet this manager has not loaded, checked here first.
    ///
    /// The whole run — begin, both passes, the release of the last engine
    /// guard and the finish that releases the wallet's claim — is one
    /// synchronous block on `coreTxoReconcileQueue`, so nothing comes
    /// between a granted begin and its finish: not a cancelled caller, not
    /// a failed page, not a shutdown. Store steps run on the persistence
    /// queue, one step per closure. The run stops between pages when
    /// `shutdown()` or `deleteWallet` bumps `coreTxoReconcileEpoch`, or the
    /// calling task is cancelled, and reports `completed == false`; a
    /// cancelled caller whose run never started gets `CancellationError`.
    /// Each store step checks the stop again once it is on the persistence
    /// queue, so a page whose engine guard teardown may already have
    /// released writes nothing.
    ///
    /// Runs automatically whenever the engine's schedule reports the wallet
    /// due (see `launchDueCoreTxoReconciles`); hosts may also call it.
    /// Logs one skip event or one summary event — counts and keyed
    /// references only.
    public func reconcileCoreTxoStore(for walletId: Data) async throws -> CoreTxoReconcileOutcome {
        guard isConfigured, handle != NULL_HANDLE, let handler = persistence else {
            return logSkip(.notConfigured, walletId: walletId)
        }
        guard !shutdownRequested else { return logSkip(.shutdownRequested, walletId: walletId) }
        guard walletId.count == 32, wallets[walletId] != nil else {
            return logSkip(.walletUnknown, walletId: walletId)
        }

        let epoch = coreTxoReconcileEpoch
        let generation = epoch.current()
        let managerHandle = handle
        let calls = nativeTxoReconcileCalls
        let queue = coreTxoReconcileQueue
        // Moved when the calling task is cancelled; the run reads it
        // between pages, like the epoch.
        let callerCancelled = SyncGenerationCounter()

        // `withCheckedContinuation` is not interrupted by cancellation, so
        // the run's result — and with it a granted begin — is never dropped
        // on the way back.
        let result: Result<CoreTxoReconcileOutcome?, any Error> = await withTaskCancellationHandler {
            await withCheckedContinuation { continuation in
                queue.async {
                    continuation.resume(returning: Result {
                        try Self.performCoreTxoReconcileRun(
                            handle: managerHandle,
                            walletId: walletId,
                            calls: calls,
                            handler: handler,
                            isCancelled: { epoch.current() != generation || callerCancelled.current() != 0 }
                        )
                    })
                }
            }
        } onCancel: {
            callerCancelled.bump()
        }

        switch try result.get() {
        case .reconciled(let report):
            return .reconciled(report)
        case .skipped(let reason):
            return logSkip(reason, walletId: walletId)
        case nil:
            // Stopped before the walk began: shut down, a wallet deleted
            // meanwhile, or the caller cancelled.
            if shutdownRequested || handle == NULL_HANDLE {
                return logSkip(.shutdownRequested, walletId: walletId)
            }
            if callerCancelled.current() != 0 {
                throw CancellationError()
            }
            return logSkip(.walletUnknown, walletId: walletId)
        }
    }

    /// One run's native protocol, synchronous on the reconcile queue: claim
    /// the wallet, walk it, release it. `nil` when `isCancelled` stopped it
    /// before the walk began — a granted claim is released first.
    ///
    /// A `.run` begin is always followed by a finish, in a `defer`, after
    /// the engine guard the last page may still hold is released: a claim
    /// left behind would refuse every later run of the wallet as
    /// `alreadyRunning`. A skip claims nothing and is not finished. A gate
    /// code this build does not know is a contract break with the native
    /// library it shipped with, not a reason to guess, and throws.
    /// `nonisolated static` so tests can drive it with injected native
    /// calls and handler, without a native manager.
    nonisolated static func performCoreTxoReconcileRun(
        handle: Handle,
        walletId: Data,
        calls: PlatformWalletNativeTxoReconcileCalls,
        handler: PlatformWalletPersistenceHandler,
        pageSize: Int = coreTxoReconcilePageSize,
        isCancelled: @Sendable () -> Bool
    ) throws -> CoreTxoReconcileOutcome? {
        guard !isCancelled() else { return nil }
        let tipHeight: UInt32
        switch try calls.begin(handle, walletId) {
        case .skip(let code):
            guard let reason = CoreTxoReconcileSkipReason(gateCode: code) else {
                throw PlatformWalletError.walletOperation("unknown TXO reconcile gate code \(code)")
            }
            return .skipped(reason)
        case .run(let tip):
            tipHeight = tip
        }

        let engine = calls.makeEngine(handle, walletId, tipHeight)
        var ran = false
        defer {
            endCoreTxoReconcileRun(walletId: walletId, engine: engine, quiet: isCancelled()) {
                try calls.finish(handle, walletId, ran)
            }
        }
        guard !isCancelled() else { return nil }
        let report = runCoreTxoReconcile(
            walletId: walletId,
            pageSize: pageSize,
            engine: engine,
            handler: handler,
            isCancelled: isCancelled
        )
        ran = true
        return .reconciled(report)
    }

    /// End a run: free the engine guard its last page may still hold, then
    /// release the wallet's claim. The guard goes first and without the
    /// handle registry — `finish` resolves the manager there, which must
    /// never be waited for while a guard is held. A failed finish cannot
    /// skip the release; it is logged by wallet reference and error only,
    /// at debug when `quiet` (the run was staled: a shutdown that destroyed
    /// the manager first already released the claim with it).
    nonisolated static func endCoreTxoReconcileRun(
        walletId: Data,
        engine: any CoreTxoReconcileEngine,
        quiet: Bool = false,
        finish: () throws -> Void
    ) {
        engine.releaseVerdicts()
        do {
            try finish()
        } catch {
            SDKLogger.event(
                "persistence_txo_reconcile_release_failed",
                category: .persistence,
                severity: quiet ? .debug : .warning,
                fields: ["wallet_reference": .reference(walletId)],
                error: error
            )
        }
    }

    /// The walk of one claimed run: engine reads on the calling thread (the
    /// reconcile queue), store steps on the persistence queue. Logs the
    /// run's summary. Synchronous and `nonisolated static` so tests can
    /// drive it with an injected engine and handler, without a native
    /// manager.
    nonisolated static func runCoreTxoReconcile(
        walletId: Data,
        pageSize: Int = coreTxoReconcilePageSize,
        engine: any CoreTxoReconcileEngine,
        handler: PlatformWalletPersistenceHandler,
        isCancelled: @Sendable () -> Bool
    ) -> CoreTxoReconcileReport {
        var report = CoreTxoReconcileReport()
        let limit = max(pageSize, 1)
        if healMissingTxos(walletId, limit, engine, handler, isCancelled, &report) {
            classifyStoreRows(walletId, limit, engine, handler, isCancelled, &report)
        }
        logSummary(report, walletId: walletId)
        return report
    }

    /// Heal pass: every engine coin, a page at a time, each page applied in
    /// one store step. Returns whether the run may go on to the classify
    /// pass.
    private nonisolated static func healMissingTxos(
        _ walletId: Data,
        _ limit: Int,
        _ engine: any CoreTxoReconcileEngine,
        _ handler: PlatformWalletPersistenceHandler,
        _ isCancelled: @Sendable () -> Bool,
        _ report: inout CoreTxoReconcileReport
    ) -> Bool {
        var cursor: CoreEngineUtxo?
        while true {
            if isCancelled() {
                report.completed = false
                return false
            }
            let page: (rows: [CoreTxoHealRow], hasMore: Bool)
            do {
                page = try engine.healPage(after: cursor, limit: limit)
            } catch {
                report.transportFailures += 1
                report.completed = false
                return false
            }
            report.engineRows += page.rows.count
            if !page.rows.isEmpty {
                guard let counts = withRoundRetry(&report, isCancelled: isCancelled, {
                    handler.reconcileHealMissingTxos(walletId: walletId, rows: page.rows, isCancelled: isCancelled)
                }) else { return false }
                report.inserted += counts.inserted
                report.insertedDuffs = report.insertedDuffs.addingReportingOverflow(counts.insertedDuffs).0
                report.alreadyPresent += counts.alreadyPresent
                report.healedSpent += counts.healedSpent
                report.skippedImmature += counts.skippedImmature
                report.skippedUnresolvedAccount += counts.skippedUnresolvedAccount
                report.skippedInvalid += counts.skippedInvalid
            }
            guard page.hasMore, let last = page.rows.last else { return true }
            cursor = last.utxo
        }
    }

    /// Classify pass: every unspent store row of the wallet, a page at a
    /// time. Each page is read on the persistence queue together with the
    /// store's round generation, classified off it under an engine guard,
    /// and applied back on it — only if no round committed in between and
    /// none is open. Otherwise nothing is written, the guard is released,
    /// and the page is read and classified again for as long as the engine
    /// allows (`retryStalePage`); then the run stops incomplete.
    ///
    /// The guard is what makes a verdict safe to write: the store's fences
    /// see its persistence rounds, not the engine, and a fork can move the
    /// engine with no round at all. It is held for one page — from the
    /// classification to the end of that page's step — and released on
    /// every path out of it, before the next page is read.
    private nonisolated static func classifyStoreRows(
        _ walletId: Data,
        _ limit: Int,
        _ engine: any CoreTxoReconcileEngine,
        _ handler: PlatformWalletPersistenceHandler,
        _ isCancelled: @Sendable () -> Bool,
        _ report: inout CoreTxoReconcileReport
    ) {
        var offset = 0
        var staleInARow = 0
        while true {
            if isCancelled() {
                report.completed = false
                return
            }
            guard let page = withRoundRetry(&report, isCancelled: isCancelled, {
                handler.reconcileUnspentTxoPage(
                    walletId: walletId, offset: offset, limit: limit, isCancelled: isCancelled)
            }) else { return }
            report.storeRows += page.rows.count
            report.unresolvedAccount += page.unresolvedAccount
            var flippedThisPage = 0
            if !page.rows.isEmpty {
                defer { engine.releaseVerdicts() }
                let classes: [CoreOutpointClass]
                do {
                    classes = try engine.classifyHeld(page.rows.map(\.query))
                } catch {
                    report.transportFailures += 1
                    report.completed = false
                    return
                }
                guard classes.count == page.rows.count else {
                    // No verdicts, no classification: stop rather than guess.
                    report.transportFailures += 1
                    report.completed = false
                    return
                }
                if isCancelled() {
                    report.completed = false
                    return
                }
                let counts: CoreTxoFlipCounts
                switch handler.reconcileApplyEngineClasses(
                    walletId: walletId,
                    rows: page.rows,
                    classes: classes,
                    expectedGeneration: page.generation,
                    isCancelled: isCancelled
                ) {
                case .done(let value):
                    counts = value
                case .stopped:
                    // Stopped after the check above, before the apply reached
                    // the persistence queue: nothing written.
                    report.completed = false
                    return
                case .failed:
                    report.storeFailures += 1
                    report.completed = false
                    return
                case .retryLater:
                    // The apply answers an open round as a stale page; were
                    // it ever to defer instead, the run must still not wait
                    // for a round while it holds the guard.
                    var stale = CoreTxoFlipCounts()
                    stale.staleGeneration = true
                    counts = stale
                }
                if counts.staleGeneration {
                    staleInARow += 1
                    report.staleRetries += 1
                    guard engine.retryStalePage(staleInARow: staleInARow) else {
                        report.completed = false
                        return
                    }
                    // Same offset: nothing was written, so the page is read
                    // again and classified against the store as it is now.
                    report.storeRows -= page.rows.count
                    report.unresolvedAccount -= page.unresolvedAccount
                    continue
                }
                staleInARow = 0
                report.flipped += counts.flipped
                report.flippedDuffs = report.flippedDuffs.addingReportingOverflow(counts.flippedDuffs).0
                report.unspent += counts.unspent
                report.unknown += counts.unknown
                report.notOwned += counts.notOwned
                flippedThisPage = counts.flipped
            }
            guard page.hasMore else { return }
            // Flipped rows left the `isSpent == false` predicate, so the
            // next page starts that many rows earlier — at the same offset
            // when the whole page flipped, which still makes progress: the
            // rows now at that offset are ones this walk has not seen.
            offset += page.fetched - flippedThisPage
        }
    }

    /// Run one store step, retrying while a Rust round is open. `nil` when
    /// the run must stop: cancelled, the step failed, or the round never
    /// closed within the retry budget — `report` says which. Only for steps
    /// taken with no engine guard held.
    private nonisolated static func withRoundRetry<T: Sendable>(
        _ report: inout CoreTxoReconcileReport,
        isCancelled: @Sendable () -> Bool,
        _ step: () -> CoreTxoReconcileStep<T>
    ) -> T? {
        var attempts = 0
        while true {
            if isCancelled() {
                report.completed = false
                return nil
            }
            switch step() {
            case .done(let value):
                return value
            case .stopped:
                report.completed = false
                return nil
            case .failed:
                report.storeFailures += 1
                report.completed = false
                return nil
            case .retryLater:
                attempts += 1
                report.retries += 1
                if attempts > coreTxoReconcileMaxRetries {
                    report.completed = false
                    return nil
                }
                Thread.sleep(forTimeInterval: coreTxoReconcileRetryDelay)
            }
        }
    }

    /// Automatic trigger, fed by the manager's 1 Hz progress poll with the
    /// wallets the engine's schedule reports due
    /// (`platform_wallet_manager_txo_reconcile_due_wallets`, which owns the
    /// steady-state edge and the cadence). Launches one run per loaded
    /// wallet — never inline, so the tick stays cheap — and no second one
    /// while this manager's previous launch for the wallet has not returned:
    /// a report consumes nothing until a run claims the wallet, so a run
    /// still queued behind another wallet's is reported again on every
    /// tick. A wallet the engine holds but this manager has not loaded yet
    /// is reported again on the next tick. A run that throws is logged and
    /// retried when the schedule next reports it due. Called on the main
    /// actor from `applyManagerSnapshot`.
    func launchDueCoreTxoReconciles(_ due: [Data]) {
        guard !shutdownRequested, handle != NULL_HANDLE else { return }
        for walletId in due where wallets[walletId] != nil && !coreTxoReconcileLaunched.contains(walletId) {
            coreTxoReconcileLaunched.insert(walletId)
            Task { [weak self] in
                guard let self else { return }
                defer { self.coreTxoReconcileLaunched.remove(walletId) }
                do {
                    _ = try await self.reconcileCoreTxoStore(for: walletId)
                } catch {
                    SDKLogger.event(
                        "persistence_txo_reconcile_failed",
                        category: .persistence,
                        severity: .warning,
                        fields: ["wallet_reference": .reference(walletId)],
                        error: error
                    )
                }
            }
        }
    }

    private func logSkip(_ reason: CoreTxoReconcileSkipReason, walletId: Data) -> CoreTxoReconcileOutcome {
        SDKLogger.event(
            "persistence_txo_reconcile_skipped",
            category: .persistence,
            severity: .debug,
            fields: [
                "reason": .publicText(reason.rawValue),
                "wallet_reference": .reference(walletId),
            ]
        )
        return .skipped(reason)
    }

    /// The one summary event per run: counts, `completed` and the wallet's
    /// reference — no amount, not even a total.
    private nonisolated static func logSummary(_ report: CoreTxoReconcileReport, walletId: Data) {
        SDKLogger.event(
            "persistence_txo_reconcile_summary",
            category: .persistence,
            severity: report.mutations == 0 && report.completed ? .info : .warning,
            fields: [
                "already_present_count": .integer(Int64(report.alreadyPresent)),
                "completed": .boolean(report.completed),
                "engine_row_count": .integer(Int64(report.engineRows)),
                "flipped_count": .integer(Int64(report.flipped)),
                "healed_spent_count": .integer(Int64(report.healedSpent)),
                "inserted_count": .integer(Int64(report.inserted)),
                "not_owned_count": .integer(Int64(report.notOwned)),
                "retry_count": .integer(Int64(report.retries)),
                "stale_retry_count": .integer(Int64(report.staleRetries)),
                "skipped_immature_count": .integer(Int64(report.skippedImmature)),
                "skipped_invalid_count": .integer(Int64(report.skippedInvalid)),
                "skipped_unresolved_account_count": .integer(Int64(report.skippedUnresolvedAccount)),
                "store_failure_count": .integer(Int64(report.storeFailures)),
                "store_row_count": .integer(Int64(report.storeRows)),
                "transport_failure_count": .integer(Int64(report.transportFailures)),
                "unknown_count": .integer(Int64(report.unknown)),
                "unresolved_account_count": .integer(Int64(report.unresolvedAccount)),
                "unspent_count": .integer(Int64(report.unspent)),
                "wallet_reference": .reference(walletId),
            ]
        )
    }
}
