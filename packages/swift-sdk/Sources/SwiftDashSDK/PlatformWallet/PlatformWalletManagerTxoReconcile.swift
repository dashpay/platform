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
// once the scan has reached a trustworthy steady state and repairs
// exactly what positive evidence supports:
//
// - insert an engine-held coin the store lacks (validated, owned,
//   ≥ 100 confirmations);
// - mark a store row spent when the engine says the owning account
//   recorded its funding transaction, owns its script, and does not hold
//   the coin (`CoreOutpointClass.knownUncredited`).
//
// It never deletes a row, never un-marks a spent row, and never acts on
// absence alone. It is idempotent (a consistent store yields a report with
// zero mutations), wallet-scoped, paged in both directions, and stops
// between pages when the manager shuts down or the wallet is deleted.

extension PlatformWalletManager {
    /// Confirmations an engine coin needs before the heal pass inserts it.
    /// Coinbase maturity; also well past any plausible reorg.
    nonisolated static let coreTxoReconcileMinConfirmations: UInt32 = 100
    /// How far the wallet's durable scan watermark may trail the scan tip
    /// for the scan to count as complete for this wallet.
    nonisolated static let coreTxoReconcileTipMargin: UInt32 = 6
    /// Cadence of the automatic run while the client stays in steady state.
    nonisolated static let coreTxoReconcileCadence: Duration = .seconds(30 * 60)
    /// Rows per engine page / per store page.
    nonisolated static let coreTxoReconcilePageSize = 512
    /// How many times a step is deferred behind open Rust rounds before the
    /// run gives up (each deferral waits `coreTxoReconcileRetryDelay`).
    nonisolated static let coreTxoReconcileMaxRetries = 200
    nonisolated static let coreTxoReconcileRetryDelay: TimeInterval = 0.05
    /// How many times in a row one page is classified again because a
    /// persistence round committed between its read and its apply, before
    /// the run gives up.
    nonisolated static let coreTxoReconcileMaxStaleRetries = 5

    /// Reconcile the SwiftData TXO store of `walletId` against the engine,
    /// once the SPV scan has reached a trustworthy steady state — see the
    /// file comment for what it repairs and what it refuses to touch.
    ///
    /// Gates, each a `.skipped` outcome rather than an error: the manager
    /// must be configured and not shutting down, the wallet loaded and not
    /// already being reconciled, SPV running and in steady state (dash-spv's
    /// fully-synced state is `waitForEvents` with the filter phase at its
    /// target; `.synced` is transient), no sync fault latched
    /// (`syncFaultDetected()` — a rejected round means rows are missing by
    /// design and a rescan is pending), and the wallet's own durable scan
    /// watermark within `coreTxoReconcileTipMargin` of the scan tip. The
    /// engine reads run on `coreTxoReconcileQueue` (they park on the wallet
    /// lock); the store writes run on the persistence queue, one step per
    /// closure, deferred while a Rust round is open. The run stops between
    /// pages when `shutdown()` or `deleteWallet` bumps
    /// `coreTxoReconcileEpoch`, reporting `completed == false`.
    ///
    /// Runs automatically on the steady-state transition and every
    /// `coreTxoReconcileCadence`; hosts may also call it directly.
    public func reconcileCoreTxoStore(for walletId: Data) async throws -> CoreTxoReconcileOutcome {
        guard isConfigured, handle != NULL_HANDLE, let handler = persistence else {
            return logSkip(.notConfigured, walletId: walletId)
        }
        guard !shutdownRequested else { return logSkip(.shutdownRequested, walletId: walletId) }
        guard walletId.count == 32, wallets[walletId] != nil else {
            return logSkip(.walletUnknown, walletId: walletId)
        }
        guard !coreTxoReconcileInFlight.contains(walletId) else {
            return logSkip(.alreadyRunning, walletId: walletId)
        }
        guard spvIsRunning else { return logSkip(.spvNotRunning, walletId: walletId) }
        let progress = spvProgress
        guard Self.isSteadySyncState(progress) else { return logSkip(.notSteadyState, walletId: walletId) }
        guard let tipHeight = Self.scanTipHeight(progress) else {
            return logSkip(.tipUnavailable, walletId: walletId)
        }
        if try syncFaultDetected() { return logSkip(.syncFaultDetected, walletId: walletId) }

        coreTxoReconcileInFlight.insert(walletId)
        defer { coreTxoReconcileInFlight.remove(walletId) }

        let epoch = coreTxoReconcileEpoch
        let generation = epoch.current()
        let managerHandle = handle
        let queue = coreTxoReconcileQueue

        // The wallet's own durable watermark, read off-main: the scan tip
        // says how far the CLIENT got, not how far this wallet's rows are
        // committed, and a wallet added behind the tip is still being
        // scanned.
        let state: CoreWalletStateFFI? = await withCheckedContinuation { continuation in
            queue.async {
                guard epoch.current() == generation else {
                    continuation.resume(returning: nil)
                    return
                }
                continuation.resume(returning: Self.readCoreWalletState(managerHandle, walletId: walletId))
            }
        }
        guard handle != NULL_HANDLE, !shutdownRequested else {
            return logSkip(.shutdownRequested, walletId: walletId)
        }
        guard let state else { return logSkip(.walletUnknown, walletId: walletId) }
        guard state.synced_height &+ Self.coreTxoReconcileTipMargin >= tipHeight else {
            return logSkip(.walletBehindTip, walletId: walletId)
        }

        let engine = FFICoreTxoEngineInventory(handle: managerHandle, walletId: walletId)
        let report: CoreTxoReconcileReport = await withCheckedContinuation { continuation in
            queue.async {
                let report = Self.runCoreTxoReconcile(
                    walletId: walletId,
                    tipHeight: tipHeight,
                    engine: engine,
                    handler: handler,
                    isCancelled: { epoch.current() != generation }
                )
                continuation.resume(returning: report)
            }
        }
        coreTxoReconcileLastRunAt[walletId] = ContinuousClock.now
        Self.logSummary(report, walletId: walletId)
        return .reconciled(report)
    }

    /// dash-spv's steady state for a fully synced client is `waitForEvents`
    /// with the filter phase at its target height; `.synced` is the
    /// transient window before it. Either counts.
    nonisolated static func isSteadySyncState(_ progress: PlatformSpvSyncProgress) -> Bool {
        switch progress.overallState {
        case .synced:
            return true
        case .waitForEvents:
            guard let filters = progress.filters else { return false }
            return filters.targetHeight > 0 && filters.currentHeight >= filters.targetHeight
        case .waitingForConnections, .syncing, .error:
            return false
        }
    }

    /// The scan tip: the filter phase's height (the wallet-relevant one),
    /// falling back to the header tip when the filter phase is absent.
    nonisolated static func scanTipHeight(_ progress: PlatformSpvSyncProgress) -> UInt32? {
        if let filters = progress.filters, filters.currentHeight > 0 {
            return filters.currentHeight
        }
        if let headers = progress.headers, headers.currentHeight > 0 {
            return headers.currentHeight
        }
        return nil
    }

    /// The wallet's durable core scan state, or `nil` when the manager does
    /// not know the wallet. Parks on the wallet lock — call off-main.
    nonisolated static func readCoreWalletState(_ handle: Handle, walletId: Data) -> CoreWalletStateFFI? {
        guard walletId.count == 32 else { return nil }
        var state = CoreWalletStateFFI()
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            platform_wallet_core_wallet_state(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &state
            )
        }
        return PlatformWalletResult(result).isSuccess ? state : nil
    }

    /// The reconcile itself: engine reads on the calling thread (the
    /// reconcile queue), store steps on the persistence queue. Synchronous
    /// and `nonisolated static` so tests can drive it with an injected
    /// engine and handler, without a native manager.
    nonisolated static func runCoreTxoReconcile(
        walletId: Data,
        tipHeight: UInt32,
        minConfirmations: UInt32 = coreTxoReconcileMinConfirmations,
        pageSize: Int = coreTxoReconcilePageSize,
        engine: any CoreTxoEngineInventory,
        handler: PlatformWalletPersistenceHandler,
        isCancelled: @Sendable () -> Bool
    ) -> CoreTxoReconcileReport {
        var report = CoreTxoReconcileReport()

        // Pass A — heal: every engine coin the store lacks.
        var cursor: CoreEngineUtxo?
        while true {
            if isCancelled() {
                report.completed = false
                return report
            }
            let page: (rows: [CoreEngineUtxo], hasMore: Bool)
            do {
                page = try engine.utxoPage(after: cursor, limit: pageSize)
            } catch {
                report.transportFailures += 1
                report.completed = false
                return report
            }
            report.engineRows += page.rows.count
            if !page.rows.isEmpty {
                guard let counts = withRoundRetry(&report, isCancelled: isCancelled, {
                    handler.reconcileHealMissingTxos(
                        walletId: walletId,
                        rows: page.rows,
                        tipHeight: tipHeight,
                        minConfirmations: minConfirmations
                    )
                }) else { return report }
                report.inserted += counts.inserted
                report.insertedDuffs = report.insertedDuffs.addingReportingOverflow(counts.insertedDuffs).0
                report.alreadyPresent += counts.alreadyPresent
                report.skippedImmature += counts.skippedImmature
                report.skippedUnresolvedAccount += counts.skippedUnresolvedAccount
                report.skippedInvalid += counts.skippedInvalid
            }
            guard page.hasMore, let last = page.rows.last else { break }
            cursor = last
        }

        // Pass B — classify: every unspent store row of the wallet. A page
        // whose verdicts were read before a round committed is classified
        // again (`staleGeneration`), at most `coreTxoReconcileMaxStaleRetries`
        // times in a row.
        var offset = 0
        var staleRetries = 0
        while true {
            if isCancelled() {
                report.completed = false
                return report
            }
            guard let page = withRoundRetry(&report, isCancelled: isCancelled, {
                handler.reconcileUnspentTxoPage(walletId: walletId, offset: offset, limit: pageSize)
            }) else { return report }
            report.storeRows += page.rows.count
            var flippedThisPage = 0
            if !page.rows.isEmpty {
                let classes: [CoreOutpointClass]
                do {
                    classes = try engine.classify(page.rows.map(\.query))
                } catch {
                    report.transportFailures += 1
                    report.completed = false
                    return report
                }
                guard classes.count == page.rows.count else {
                    report.transportFailures += 1
                    report.completed = false
                    return report
                }
                guard let counts = withRoundRetry(&report, isCancelled: isCancelled, {
                    handler.reconcileApplyEngineClasses(
                        walletId: walletId,
                        rows: page.rows,
                        classes: classes,
                        expectedGeneration: page.generation
                    )
                }) else { return report }
                if counts.staleGeneration {
                    staleRetries += 1
                    report.staleRetries += 1
                    guard staleRetries <= coreTxoReconcileMaxStaleRetries else {
                        report.completed = false
                        return report
                    }
                    // Same offset: nothing was written, the page is re-read
                    // and classified against the store as it is now.
                    report.storeRows -= page.rows.count
                    continue
                }
                staleRetries = 0
                report.flipped += counts.flipped
                report.flippedDuffs = report.flippedDuffs.addingReportingOverflow(counts.flippedDuffs).0
                report.unspent += counts.unspent
                report.unknown += counts.unknown
                report.notOwned += counts.notOwned
                flippedThisPage = counts.flipped
            }
            guard page.hasMore else { break }
            // Flipped rows left the `isSpent == false` predicate, so the
            // next page starts that many rows earlier — at the same offset
            // when the whole page flipped, which still makes progress: the
            // rows now at that offset are ones this walk has not seen.
            offset += page.fetched - flippedThisPage
        }
        return report
    }

    /// Run one store step, retrying while a Rust round is open. `nil` when
    /// the run must stop: cancelled, the step failed, or the round never
    /// closed within the retry budget — `report` says which.
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

    /// Automatic trigger, fed by the manager's 1 Hz progress poll: run for
    /// every loaded wallet on the transition into steady state, and again
    /// every `coreTxoReconcileCadence` while it lasts. Never inline — the
    /// tick is budget-sensitive — and never while a run for the same wallet
    /// is in flight. Called on the main actor from `applyManagerSnapshot`.
    func noteSpvProgressForCoreTxoReconcile(_ progress: PlatformSpvSyncProgress) {
        let steady = Self.isSteadySyncState(progress)
        let rising = steady && !coreTxoReconcileWasSteady
        coreTxoReconcileWasSteady = steady
        guard steady, !shutdownRequested, spvIsRunning else { return }
        let now = ContinuousClock.now
        for walletId in wallets.keys {
            guard !coreTxoReconcileInFlight.contains(walletId) else { continue }
            let due = rising || coreTxoReconcileLastRunAt[walletId].map {
                now - $0 >= Self.coreTxoReconcileCadence
            } ?? true
            guard due else { continue }
            // Stamped at schedule time so the next tick does not schedule
            // the same wallet again while this run is still gating.
            coreTxoReconcileLastRunAt[walletId] = now
            Task { [weak self] in
                guard let self else { return }
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
                "flipped_value_duffs": .unsignedInteger(report.flippedDuffs),
                "inserted_count": .integer(Int64(report.inserted)),
                "inserted_value_duffs": .unsignedInteger(report.insertedDuffs),
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
                "unspent_count": .integer(Int64(report.unspent)),
                "wallet_reference": .reference(walletId),
            ]
        )
    }
}
