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
// spent, how long a stale page is retried, how many pages one run may read
// (the heal pass's and the classify pass's together), and where the
// wallet's next run picks up. The begin says where this run starts; the
// engine keeps the run's place in its own inventory and counts its pages,
// and the run reports each page applied once its store step has committed
// (`healPageApplied`, `classifyPageApplied`). A run that has read its
// budget stops between pages (`stoppedOnBudget`), and the next one resumes
// at the first page it did not apply. This file keeps only what needs the
// store: paging its rows (by offset, which it hands the engine as its
// opaque store cursor), resolving the account that owns each, its writes,
// and the fences that keep Rust's persistence rounds and teardown out of a
// run's writes.
//
// Lock order, the engine's own: the native handle registry (read only
// inside one native call, to resolve the manager), the engine's wallet lock
// (held for reading by a page's classification until its apply is
// written), the persistence queue, the SwiftData save. The engine calls the
// persistence callbacks — which run on that queue — while it holds its
// wallet lock for writing, so a classification is asked for off the
// persistence queue, never from a callback, and its guard is released after
// the apply has left the queue and before anything resolves the manager
// again: the page's applied report, the next page's read, or the run's
// finish.

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
    /// How much the run does is native too: it reads at most the engine's
    /// page budget and stops between pages when that is spent, reporting
    /// `stoppedOnBudget`; the wallet's next run picks up where it stopped
    /// (`resumedFromCheckpoint`).
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
        let start: CoreTxoReconcileStart
        switch try calls.begin(handle, walletId) {
        case .skip(let code):
            guard let reason = CoreTxoReconcileSkipReason(gateCode: code) else {
                throw PlatformWalletError.walletOperation("unknown TXO reconcile gate code \(code)")
            }
            return .skipped(reason)
        case .run(_, let granted):
            start = granted
        }

        let engine = calls.makeEngine(handle, walletId)
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
            start: start,
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

    /// The walk of one claimed run, from `start` (the begin's answer):
    /// engine reads on the calling thread (the reconcile queue), store steps
    /// on the persistence queue. Logs the run's summary. Synchronous and
    /// `nonisolated static` so tests can drive it with an injected engine
    /// and handler, without a native manager.
    ///
    /// Each page goes: engine read, store step, guard released, then — only
    /// when the step committed or there was nothing to write — the page
    /// reported applied, whose answer says whether the run may read
    /// another. A page that failed, was deferred, came back stale, or was
    /// stopped is never reported applied, so the wallet's next run reads it
    /// again. The run stops on the budget when a read says the budget is
    /// spent, or when an applied report says no budget is left before the
    /// cycle's end; the classify pass's last store page is reported as the
    /// last, which ends the cycle.
    nonisolated static func runCoreTxoReconcile(
        walletId: Data,
        pageSize: Int = coreTxoReconcilePageSize,
        start: CoreTxoReconcileStart = .heal(resumed: false),
        engine: any CoreTxoReconcileEngine,
        handler: PlatformWalletPersistenceHandler,
        isCancelled: @Sendable () -> Bool
    ) -> CoreTxoReconcileReport {
        var report = CoreTxoReconcileReport()
        report.resumedFromCheckpoint = start.isResumed
        let limit = max(pageSize, 1)
        switch start {
        case .heal:
            if healMissingTxos(walletId, limit, engine, handler, isCancelled, &report) {
                classifyStoreRows(walletId, limit, 0, engine, handler, isCancelled, &report)
            }
        case .classify(let after):
            classifyStoreRows(
                walletId, limit, coreTxoStoreOffset(cursor: after), engine, handler, isCancelled, &report)
        }
        logSummary(report, walletId: walletId)
        return report
    }

    /// The store cursor the classify pass hands the engine for the page
    /// starting at `offset`: the offset as 8 bytes, little-endian. The
    /// engine keeps it unread and hands it back to the wallet's next run.
    /// An offset resumed in a later run may skip or repeat a few rows when
    /// the store changed between the runs (rows inserted, flipped or
    /// written by another wallet ahead of it); the next cycle walks them.
    nonisolated static func coreTxoStoreCursor(offset: Int) -> Data {
        withUnsafeBytes(of: UInt64(clamping: max(offset, 0)).littleEndian) { Data($0) }
    }

    /// The offset `coreTxoStoreCursor(offset:)` encoded; 0 — the store's
    /// first row — for an empty or malformed cursor.
    nonisolated static func coreTxoStoreOffset(cursor: Data) -> Int {
        guard cursor.count == 8 else { return 0 }
        let value = cursor.reduce(into: (value: UInt64(0), shift: UInt64(0))) { acc, byte in
            acc.value |= UInt64(byte) << acc.shift
            acc.shift += 8
        }.value
        return Int(exactly: value) ?? 0
    }

    /// The run read its budget, or was told none is left, before the end of
    /// the cycle: it stops cleanly between pages.
    private nonisolated static func stopOnBudget(_ report: inout CoreTxoReconcileReport) {
        report.stoppedOnBudget = true
        report.completed = false
    }

    /// Report the page just written applied — after its guard was released:
    /// the call resolves the manager handle. Returns whether the run may
    /// read another page, or `nil` when the run must stop: cancelled (the
    /// page is left unreported, so the next run reads it again; its step is
    /// idempotent) or the call failed.
    private nonisolated static func reportPageApplied(
        _ report: inout CoreTxoReconcileReport,
        isCancelled: @Sendable () -> Bool,
        _ applied: () throws -> Bool
    ) -> Bool? {
        if isCancelled() {
            report.completed = false
            return nil
        }
        do {
            return try applied()
        } catch {
            report.transportFailures += 1
            report.completed = false
            return nil
        }
    }

    /// Heal pass: the engine's coins from where the run starts (the engine
    /// keeps that place), a page at a time, each page applied in one store
    /// step and then reported applied. Returns whether the run may go on to
    /// the classify pass: the inventory's end was reached with budget left.
    ///
    /// Each page is read under an engine guard, held until its step is done:
    /// a fork that truncated the engine between the read and the write would
    /// leave a healed row unspent in the store for a coin the engine no
    /// longer owns — the classify pass answers `unknown` for it and leaves
    /// it, and the next launch restores it as spendable. A step deferred for
    /// an open round releases the guard before it waits, then reads the
    /// same page again under a new one (another page of the budget), without
    /// reporting the first read applied.
    private nonisolated static func healMissingTxos(
        _ walletId: Data,
        _ limit: Int,
        _ engine: any CoreTxoReconcileEngine,
        _ handler: PlatformWalletPersistenceHandler,
        _ isCancelled: @Sendable () -> Bool,
        _ report: inout CoreTxoReconcileReport
    ) -> Bool {
        var deferrals = 0
        while true {
            if isCancelled() {
                report.completed = false
                return false
            }
            let page: (rows: [CoreTxoHealRow], hasMore: Bool)
            let step: CoreTxoReconcileStep<CoreTxoHealCounts>?
            do {
                defer { engine.releaseVerdicts() }
                let read: CoreTxoReconcileRead<(rows: [CoreTxoHealRow], hasMore: Bool)>
                do {
                    read = try engine.healPage(limit: limit)
                } catch {
                    report.transportFailures += 1
                    report.completed = false
                    return false
                }
                guard case .page(let value) = read else {
                    stopOnBudget(&report)
                    return false
                }
                page = value
                if page.rows.isEmpty {
                    step = nil
                } else if isCancelled() {
                    report.completed = false
                    return false
                } else {
                    step = handler.reconcileHealMissingTxos(
                        walletId: walletId, rows: page.rows, isCancelled: isCancelled)
                }
            }
            if let step {
                let counts: CoreTxoHealCounts
                switch step {
                case .done(let value):
                    counts = value
                case .stopped:
                    report.completed = false
                    return false
                case .failed:
                    report.storeFailures += 1
                    report.completed = false
                    return false
                case .retryLater:
                    // The guard is already released: wait for the round
                    // guard-free, then read the page again. Not applied, so
                    // the engine serves the same page.
                    deferrals += 1
                    report.retries += 1
                    if deferrals > coreTxoReconcileMaxRetries {
                        report.completed = false
                        return false
                    }
                    Thread.sleep(forTimeInterval: coreTxoReconcileRetryDelay)
                    continue
                }
                deferrals = 0
                report.inserted += counts.inserted
                report.insertedDuffs = report.insertedDuffs.addingReportingOverflow(counts.insertedDuffs).0
                report.alreadyPresent += counts.alreadyPresent
                report.healedSpent += counts.healedSpent
                report.skippedImmature += counts.skippedImmature
                report.skippedUnresolvedAccount += counts.skippedUnresolvedAccount
                report.skippedInvalid += counts.skippedInvalid
            }
            report.engineRows += page.rows.count
            // The step committed (or there was nothing to write) and the
            // guard is released: report the page applied.
            guard let mayContinue = reportPageApplied(&report, isCancelled: isCancelled, {
                try engine.healPageApplied()
            }) else { return false }
            // At the inventory's end the engine moved the wallet's next run
            // on to the classify pass; without budget left, this run stops
            // before it.
            guard mayContinue else {
                stopOnBudget(&report)
                return false
            }
            if !page.hasMore || page.rows.isEmpty { return true }
        }
    }

    /// Classify pass: the unspent store rows from `startOffset` (0, or the
    /// offset an earlier run's last applied page left), a page at a time.
    /// Each page is read on the persistence queue together with the store's
    /// round generation, classified off it under an engine guard — an empty
    /// page too, since each is one page of the run's budget — and applied
    /// back on it, only if no round committed in between and none is open.
    /// Otherwise nothing is written, the guard is released, and the page is
    /// read and classified again (another page of the budget) for as long
    /// as the engine allows (`retryStalePage`); then the run stops
    /// incomplete. Only the attempt that applies is reported applied, with
    /// the store cursor past it, or as the last page when the store has no
    /// more.
    ///
    /// The guard is what makes a verdict safe to write: the store's fences
    /// see its persistence rounds, not the engine, and a fork can move the
    /// engine with no round at all. It is held for one page — from the
    /// classification to the end of that page's step — and released on
    /// every path out of it, before the page is reported applied and before
    /// the next page is read.
    private nonisolated static func classifyStoreRows(
        _ walletId: Data,
        _ limit: Int,
        _ startOffset: Int,
        _ engine: any CoreTxoReconcileEngine,
        _ handler: PlatformWalletPersistenceHandler,
        _ isCancelled: @Sendable () -> Bool,
        _ report: inout CoreTxoReconcileReport
    ) {
        var offset = startOffset
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
            do {
                defer { engine.releaseVerdicts() }
                let read: CoreTxoReconcileRead<[CoreOutpointClass]>
                do {
                    read = try engine.classifyHeld(page.rows.map(\.query))
                } catch {
                    report.transportFailures += 1
                    report.completed = false
                    return
                }
                guard case .page(let classes) = read else {
                    // Not classified: this run walked none of the page, and
                    // the next run reads it.
                    report.storeRows -= page.rows.count
                    report.unresolvedAccount -= page.unresolvedAccount
                    stopOnBudget(&report)
                    return
                }
                guard classes.count == page.rows.count else {
                    // No verdicts, no classification: stop rather than guess.
                    report.transportFailures += 1
                    report.completed = false
                    return
                }
                if !page.rows.isEmpty {
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
                        // Stopped after the check above, before the apply
                        // reached the persistence queue: nothing written.
                        report.completed = false
                        return
                    case .failed:
                        report.storeFailures += 1
                        report.completed = false
                        return
                    case .retryLater:
                        // The apply answers an open round as a stale page;
                        // were it ever to defer instead, the run must still
                        // not wait for a round while it holds the guard.
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
                        // Same offset, not reported applied: nothing was
                        // written, so the page is read again and classified
                        // against the store as it is now.
                        report.storeRows -= page.rows.count
                        report.unresolvedAccount -= page.unresolvedAccount
                        continue
                    }
                    report.flipped += counts.flipped
                    report.flippedDuffs = report.flippedDuffs.addingReportingOverflow(counts.flippedDuffs).0
                    report.unspent += counts.unspent
                    report.unknown += counts.unknown
                    report.notOwned += counts.notOwned
                    flippedThisPage = counts.flipped
                }
            }
            staleInARow = 0
            let isLast = !page.hasMore
            // Flipped rows left the `isSpent == false` predicate, so the
            // next page starts that many rows earlier — at the same offset
            // when the whole page flipped, which still makes progress: the
            // rows now at that offset are ones this walk has not seen.
            let next = offset + page.fetched - flippedThisPage
            // The apply committed (or there was nothing to write) and the
            // guard is released: report the page applied — the store's last
            // page ends the cycle.
            guard let mayContinue = reportPageApplied(&report, isCancelled: isCancelled, {
                try engine.classifyPageApplied(nextAfter: isLast ? nil : coreTxoStoreCursor(offset: next))
            }) else { return }
            if isLast { return }
            guard mayContinue else {
                stopOnBudget(&report)
                return
            }
            offset = next
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
        // A manager configured without persistence has no store to repair:
        // every run would skip before claiming the wallet, so the schedule
        // would report it due again on every tick.
        guard !shutdownRequested, handle != NULL_HANDLE, persistence != nil else { return }
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

    /// The summary's level: `info` for a run that wrote nothing and either
    /// completed or stopped on its budget — an expected state, the rest of
    /// the cycle is the next run's — and `warning` for any write or any
    /// other stop.
    nonisolated static func summarySeverity(_ report: CoreTxoReconcileReport) -> SDKLogSeverity {
        report.mutations == 0 && (report.completed || report.stoppedOnBudget) ? .info : .warning
    }

    /// The one summary event per run: counts, `completed`, how the run
    /// started and stopped, and the wallet's reference — no amount, not even
    /// a total.
    private nonisolated static func logSummary(_ report: CoreTxoReconcileReport, walletId: Data) {
        SDKLogger.event(
            "persistence_txo_reconcile_summary",
            category: .persistence,
            severity: summarySeverity(report),
            fields: [
                "already_present_count": .integer(Int64(report.alreadyPresent)),
                "completed": .boolean(report.completed),
                "engine_row_count": .integer(Int64(report.engineRows)),
                "flipped_count": .integer(Int64(report.flipped)),
                "healed_spent_count": .integer(Int64(report.healedSpent)),
                "inserted_count": .integer(Int64(report.inserted)),
                "not_owned_count": .integer(Int64(report.notOwned)),
                "resumed_from_checkpoint": .boolean(report.resumedFromCheckpoint),
                "retry_count": .integer(Int64(report.retries)),
                "stale_retry_count": .integer(Int64(report.staleRetries)),
                "skipped_immature_count": .integer(Int64(report.skippedImmature)),
                "skipped_invalid_count": .integer(Int64(report.skippedInvalid)),
                "skipped_unresolved_account_count": .integer(Int64(report.skippedUnresolvedAccount)),
                "store_failure_count": .integer(Int64(report.storeFailures)),
                "stopped_on_budget": .boolean(report.stoppedOnBudget),
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
