import Foundation
import SwiftDashSDK

/// Error a drain attempt throws when Rust did not report it completed.
/// The text is Rust's; the controller shows it as `.failed(message)`.
struct WalletDrainAttemptError: LocalizedError, Equatable {
    let message: String
    var errorDescription: String? { message }
}

/// Starts, resumes and reconciles Core-to-shielded "Max" transfers.
///
/// Swift decides nothing about amounts, inputs or fees. It keeps a durable
/// marker of each operation id (`WalletDrainOperationStore`), calls the SDK
/// wrappers, and lists what Rust reports. Execution runs through the
/// existing `ShieldedFundFromAssetLockCoordinator` so the per-wallet
/// single-flight rule and the progress UI are shared with the sibling flow.
///
/// Only `start` calls `shieldedFundFromWalletDrain`. A resume calls
/// `shieldedResumeFundFromAssetLock` with the exact outpoint the last
/// status read returned, which can never build a new lock; a repeated
/// drain call could, when Rust's state for the id is still "not sent".
///
/// Markers are removed by `reconcile`, from a status Rust returned, and by
/// `forgetWallet` when the wallet is deleted (one more exception: a start
/// the funding coordinator refused before running anything).
@MainActor
final class WalletDrainCoordinator: ObservableObject {
    struct Entry: Identifiable, Equatable {
        let record: WalletDrainOperationRecord
        /// The asset lock the last successful status read named. `nil`
        /// when the read failed or named none. Only a `pending` report's
        /// outpoint allows a spend (`rowAction`).
        let outpoint: WalletDrainOutpoint?
        /// What the last status read reported. `nil` before any read has
        /// finished and when the last read failed: a marker is kept in
        /// either case, so its presence alone says nothing about Rust.
        let reportedState: WalletDrainOutcome.State?
        /// The last status read's error, when it failed.
        let readError: String?
        /// Set when Rust reported `completed` or `notSent` but removing the
        /// marker failed. The marker is all that is left; reconcile retries
        /// the removal.
        let removalError: String?
        /// `false` until a status read for this operation has finished,
        /// successfully or not. A stored marker is listed before that (its
        /// lock is already hidden from the generic list), as a row that
        /// says its status is being checked and offers no spend.
        let statusRead: Bool
        var id: UUID { record.operationId }

        /// A stored marker no status read has reported on yet.
        static func unread(_ record: WalletDrainOperationRecord) -> Entry {
            Entry(
                record: record,
                outpoint: nil,
                reportedState: nil,
                readError: nil,
                removalError: nil,
                statusRead: false
            )
        }
    }

    /// The status line a saved-transfer row shows. Each case claims only
    /// what the entry's last read (or the running attempt) supports.
    enum RowStatus: Equatable {
        /// An attempt for this operation is running.
        case inProgress
        /// Rust reported `pending` with the lock to resume.
        case unfinished
        /// Rust reported `pending` without naming a lock.
        case unfinishedLockUnknown
        /// No status read has finished yet.
        case checking
        /// The last status read failed.
        case readFailed(String)
        /// Rust reported `completed`; the marker could not be removed
        /// (`nil` only if no removal was tried, which reconcile never
        /// leaves behind).
        case finishedNotCleared(String?)
        /// Rust reported `notSent`; the marker could not be removed. Kept
        /// apart from `finishedNotCleared` so nothing reads as finished.
        case notSentNotCleared(String)
        /// Rust reported `notSent` while an attempt was starting, so the
        /// marker was kept: that attempt may have sent after the read.
        case notSentWhileAttemptStarted

        var text: String {
            switch self {
            case .inProgress:
                return "In progress."
            case .unfinished:
                return "Unfinished: resume to finish it. A new Max cannot start until it does."
            case .unfinishedLockUnknown:
                return "Unfinished, but the status named no asset lock yet: check again."
            case .checking:
                return "Checking status…"
            case .readFailed(let error):
                return "Status could not be read: \(error)"
            case .finishedNotCleared(let error?):
                return "Finished; the saved record could not be cleared: \(error)"
            case .finishedNotCleared(nil):
                return "Finished; the saved record is not cleared yet."
            case .notSentNotCleared(let error):
                return "Not sent; the saved record could not be cleared: \(error)"
            case .notSentWhileAttemptStarted:
                return "Nothing was sent when last read, but an attempt was starting: check again."
            }
        }
    }

    /// What one status read produced.
    enum StatusRead: Equatable {
        case reported(WalletDrainOutcome.State)
        case failed
    }

    enum MarkerDecision: Equatable {
        case forget
        case keep
    }

    enum StartResult {
        case started(ShieldedFundFromAssetLockController)
        case blockedByOtherWalletFunding(ShieldedFundFromAssetLockController)
        case markerWriteFailed(String)
    }

    /// The one action an unfinished-transfer row offers.
    enum RowAction: Equatable {
        /// An attempt for this operation is running.
        case inProgress
        /// Resume the asset lock at this exact outpoint.
        case resume(WalletDrainOutpoint)
        /// No `pending` report with an outpoint: read the status again,
        /// never spend.
        case checkAgain
    }

    /// Every stored marker, on every network, with its last reported state
    /// (`statusRead == false` before any read has reported on it). A marker
    /// is not by itself an unfinished transfer: see `Entry.reportedState`.
    @Published private(set) var unfinished: [Entry] = []
    /// Every operation id with a stored marker, read or not yet read.
    @Published private(set) var markedOperationIds: Set<UUID> = []
    /// Set when the marker file cannot be read, cleared by the next
    /// successful read. Shown rather than treated as "no saved
    /// transfers". While set, `unfinished` is what the last successful
    /// read listed (empty if none has succeeded), so it may be stale.
    @Published private(set) var storeError: String?

    private let store: WalletDrainOperationStore
    private var latest: [UUID: Entry] = [:]
    /// Attempts started per operation, to detect a status read that an
    /// attempt overlapped.
    private var attemptCounts: [UUID: Int] = [:]
    /// Attempts whose body may still be running per operation.
    private var runningCounts: [UUID: Int] = [:]
    /// Status reads started per operation. Launch, wallet-detail and
    /// post-attempt reconciles can all wait on a read for the same id, and
    /// they can finish in any order.
    private var readsStarted: [UUID: Int] = [:]
    /// The start number of the newest read applied per operation. A read
    /// that started before it is dropped, so an older `pending` cannot
    /// replace a newer terminal report and offer Resume again.
    private var newestAppliedRead: [UUID: Int] = [:]

    init(store: WalletDrainOperationStore = .shared) {
        self.store = store
        refreshFromStore()
    }

    nonisolated static let controllerOperationIdPrefix = "drain:"

    /// Shown when an asset-lock funding (a shield, a Max or its Resume, a
    /// Platform address top-up or a Core-funded identity registration) is
    /// tapped while an SDK rebuild runs (`AppState.coreFundingRefusal`).
    /// The rebuild stops SPV, which a funding needs for its ChainLock, so
    /// the views refuse to start one until it ends.
    nonisolated static let rebuildInProgressMessage =
        "Nothing was sent: the app is applying a network or settings change. "
            + "Try again once it finishes."

    /// Shown when a Resume finds no tracked asset lock for the outpoint.
    /// "Not tracked" only means the outpoint is not in the wallet's live
    /// map: a stored row may still exist and be rebroadcast after a
    /// restart, so this claims nothing about what was sent or which coins
    /// are free, and points at the status read instead.
    nonisolated static let assetLockNotTrackedMessage =
        "The wallet is not tracking this asset lock right now. Restart the app, then "
            + "check this transfer again and follow the status it reports."

    /// The error a resume throws for `error`: a lock the wallet does not
    /// track gets a message that says how to recover, any other error
    /// passes through unchanged.
    nonisolated static func resumeError(for error: Error) -> Error {
        if case PlatformWalletError.assetLockNotTracked = error {
            return WalletDrainAttemptError(message: assetLockNotTrackedMessage)
        }
        return error
    }

    nonisolated static func controllerOperationId(_ operationId: UUID) -> String {
        controllerOperationIdPrefix + operationId.uuidString
    }

    /// Inverse of `controllerOperationId`. `nil` for any other operation id
    /// (fresh shields, generic resumes) and for a malformed UUID.
    nonisolated static func drainOperationId(fromControllerOperationId value: String) -> UUID? {
        guard value.hasPrefix(controllerOperationIdPrefix) else { return nil }
        return UUID(uuidString: String(value.dropFirst(controllerOperationIdPrefix.count)))
    }

    /// The row rule. A running attempt wins; otherwise only an outpoint
    /// from a `pending` report allows a spend, and that spend is an
    /// exact-outpoint resume. Any other row can only read the status again,
    /// which for a finished one also retries removing its marker.
    nonisolated static func rowAction(
        controllerPhase: ShieldedFundFromAssetLockController.Phase?,
        reportedState: WalletDrainOutcome.State?,
        outpoint: WalletDrainOutpoint?
    ) -> RowAction {
        if case .inFlight? = controllerPhase {
            return .inProgress
        }
        if reportedState == .pending, let outpoint {
            return .resume(outpoint)
        }
        return .checkAgain
    }

    /// The row's status line, from the entry and the live controller phase.
    nonisolated static func rowStatus(
        controllerPhase: ShieldedFundFromAssetLockController.Phase?,
        entry: Entry
    ) -> RowStatus {
        if case .inFlight? = controllerPhase {
            return .inProgress
        }
        guard entry.statusRead else {
            return .checking
        }
        if let readError = entry.readError {
            return .readFailed(readError)
        }
        switch entry.reportedState {
        case .pending?:
            return entry.outpoint == nil ? .unfinishedLockUnknown : .unfinished
        case .completed?:
            return .finishedNotCleared(entry.removalError)
        case .notSent?:
            if let removalError = entry.removalError {
                return .notSentNotCleared(removalError)
            }
            return .notSentWhileAttemptStarted
        case nil:
            // A finished read always sets a state or an error.
            return .checking
        }
    }

    /// The marker rule. `completed` is done. `notSent` from a successful
    /// durable read means nothing was dispatched, unless an attempt started
    /// while the read was in progress (its dispatch may postdate the read).
    /// `pending` and a failed read keep the marker.
    nonisolated static func markerDecision(
        for read: StatusRead,
        attemptOverlappedRead: Bool
    ) -> MarkerDecision {
        switch read {
        case .reported(.completed):
            return .forget
        case .reported(.notSent):
            return attemptOverlappedRead ? .keep : .forget
        case .reported(.pending), .failed:
            return .keep
        }
    }

    /// The error a finished attempt throws, or `nil` when Rust reported it
    /// completed (even if it also carried an error, such as "already
    /// consumed" for a repeated operation id).
    nonisolated static func attemptError(for outcome: WalletDrainOutcome) -> WalletDrainAttemptError? {
        let rustText = outcome.error?.localizedDescription
        switch outcome.state {
        case .completed:
            return nil
        case .notSent:
            return WalletDrainAttemptError(message: rustText ?? "Nothing was sent.")
        case .pending:
            let notice = "This transfer is not finished: resume it from the wallet screen "
                + "and do not start a new Max."
            let message = rustText.map { "\($0) \(notice)" } ?? notice
            return WalletDrainAttemptError(message: message)
        }
    }

    /// Reads every stored marker of `networkRaw` whose wallet is in
    /// `walletIds`, and forgets or lists it by `markerDecision`. When reads
    /// of one operation overlap, the one that started last wins: an older
    /// result that finishes after it is dropped.
    func reconcile(
        walletManager: PlatformWalletManager,
        networkRaw: UInt32,
        walletIds: [Data]
    ) async {
        await reconcile(networkRaw: networkRaw, walletIds: walletIds) { record in
            try await walletManager.shieldedWalletDrainStatus(
                walletId: record.walletId,
                operationId: record.operationId
            )
        }
    }

    /// `reconcile` with Rust's status read passed in, so tests can supply
    /// the report without a loaded wallet.
    func reconcile(
        networkRaw: UInt32,
        walletIds: [Data],
        readStatus: @MainActor (WalletDrainOperationRecord) async throws -> WalletDrainOutcome
    ) async {
        let records: [WalletDrainOperationRecord]
        do {
            records = try store.records()
            storeError = nil
        } catch {
            storeError = "Could not read saved Max transfers: \(error.localizedDescription)"
            return
        }
        let loaded = Set(walletIds)
        for record in records where record.networkRaw == networkRaw && loaded.contains(record.walletId) {
            let id = record.operationId
            let attemptsBefore = attemptCounts[id, default: 0]
            let runningBefore = runningCounts[id, default: 0] > 0
            let readNumber = readsStarted[id, default: 0] + 1
            readsStarted[id] = readNumber
            let read: StatusRead
            var reportedState: WalletDrainOutcome.State?
            var outpoint: WalletDrainOutpoint?
            var readError: String?
            do {
                let outcome = try await readStatus(record)
                read = .reported(outcome.state)
                reportedState = outcome.state
                outpoint = outcome.outpoint
            } catch {
                read = .failed
                readError = error.localizedDescription
            }
            // A read that started later has already been applied: this one
            // is older news, for the entry and for the marker alike. The
            // post-attempt read starts after its attempt, so it is never
            // older than a read that attempt could have changed.
            guard readNumber > newestAppliedRead[id, default: 0] else { continue }
            newestAppliedRead[id] = readNumber
            let overlapped = runningBefore
                || runningCounts[id, default: 0] > 0
                || attemptCounts[id, default: 0] != attemptsBefore
            switch Self.markerDecision(for: read, attemptOverlappedRead: overlapped) {
            case .forget:
                do {
                    try store.remove(operationId: id)
                    latest.removeValue(forKey: id)
                } catch {
                    // Rust is done with it and only the marker is left. The
                    // terminal report is kept, so `rowAction` offers "Check
                    // again" (which retries this removal), never a spend.
                    latest[id] = Entry(
                        record: record,
                        outpoint: outpoint,
                        reportedState: reportedState,
                        readError: nil,
                        removalError: error.localizedDescription,
                        statusRead: true
                    )
                }
            case .keep:
                latest[id] = Entry(
                    record: record,
                    outpoint: outpoint,
                    reportedState: reportedState,
                    readError: readError,
                    removalError: nil,
                    statusRead: true
                )
            }
        }
        refreshFromStore()
    }

    /// Mints an operation id, stores it, then runs the drain. Nothing is
    /// called when the marker cannot be written.
    func start(
        walletId: Data,
        networkRaw: UInt32,
        fundingAccountIndex: UInt32,
        recipientRaw43: Data,
        walletManager: PlatformWalletManager
    ) -> StartResult {
        let record = WalletDrainOperationRecord(
            operationId: UUID(),
            walletId: walletId,
            networkRaw: networkRaw,
            fundingAccountIndex: fundingAccountIndex,
            recipientRaw43: recipientRaw43,
            createdAt: Date()
        )
        do {
            try store.insert(record)
        } catch {
            return .markerWriteFailed(
                "Could not save the transfer id, so nothing was sent: \(error.localizedDescription)"
            )
        }
        let result = run(record, isResume: false, walletManager: walletManager) {
            let outcome = await walletManager.shieldedFundFromWalletDrain(
                walletId: record.walletId,
                operationId: record.operationId,
                fundingAccountIndex: record.fundingAccountIndex,
                recipientRaw43: record.recipientRaw43
            )
            if let error = WalletDrainCoordinator.attemptError(for: outcome) {
                throw error
            }
        }
        switch result {
        case .started(let controller, _):
            // Listed as unread until the reconcile after the attempt reads it.
            refreshFromStore()
            return .started(controller)
        case .blockedByOtherWalletFunding(let blocker):
            // The body never ran, so nothing was sent under this id.
            do {
                try store.remove(operationId: record.operationId)
            } catch {
                // Left for reconcile, which will read it as not sent.
                SDKLogger.error("Could not remove unused wallet drain marker: \(error.localizedDescription)")
            }
            refreshFromStore()
            return .blockedByOtherWalletFunding(blocker)
        }
    }

    /// Finishes the stored operation from the asset lock at `outpoint`,
    /// paying the stored recipient. Rust writes the drain's completion
    /// receipt when it succeeds (or Platform reports the lock already
    /// consumed), so the reconcile that follows forgets the marker.
    func resume(
        _ record: WalletDrainOperationRecord,
        outpoint: WalletDrainOutpoint,
        walletManager: PlatformWalletManager
    ) -> ShieldedFundFromAssetLockCoordinator.StartFundingResult {
        run(record, isResume: true, walletManager: walletManager) {
            do {
                try await walletManager.shieldedResumeFundFromAssetLock(
                    walletId: record.walletId,
                    outPointTxid: outpoint.txidWire,
                    outPointVout: outpoint.vout,
                    recipients: [
                        ShieldedFundFromAssetLockRecipient(recipientRaw43: record.recipientRaw43)
                    ]
                )
            } catch {
                // Rust's "not tracked" text does not say what to do next.
                throw WalletDrainCoordinator.resumeError(for: error)
            }
        }
    }

    /// Reads the operation's status again. The only action for a row
    /// without an outpoint: it never sends anything.
    func checkAgain(
        _ record: WalletDrainOperationRecord,
        walletManager: PlatformWalletManager
    ) async {
        await reconcile(
            walletManager: walletManager,
            networkRaw: record.networkRaw,
            walletIds: [record.walletId]
        )
    }

    /// Reads the marker file again after `storeError`, then reconciles the
    /// given wallets. Needs no row, since a failed read may list none.
    /// Sends nothing.
    func retryStoreRead(
        walletManager: PlatformWalletManager,
        networkRaw: UInt32,
        walletIds: [Data]
    ) async {
        // Lists the markers at once, before the status reads finish.
        refreshFromStore()
        await reconcile(
            walletManager: walletManager,
            networkRaw: networkRaw,
            walletIds: walletIds
        )
    }

    /// Drops every marker of a deleted wallet, on every network.
    func forgetWallet(walletId: Data) throws {
        try store.remove(walletId: walletId)
        latest = latest.filter { $0.value.record.walletId != walletId }
        refreshFromStore()
    }

    /// Runs `attempt` for `record` through the shielded-funding coordinator
    /// under the operation's controller id, then reconciles the operation
    /// and rethrows the attempt's error for the controller to show.
    /// `isResume` marks an attempt that only finishes an existing lock, so
    /// its progress never shows that lock being built.
    private func run(
        _ record: WalletDrainOperationRecord,
        isResume: Bool,
        walletManager: PlatformWalletManager,
        attempt: @escaping () async throws -> Void
    ) -> ShieldedFundFromAssetLockCoordinator.StartFundingResult {
        let id = record.operationId
        attemptCounts[id, default: 0] += 1
        runningCounts[id, default: 0] += 1
        let body: () async throws -> Void = { [weak self] in
            var failure: Error?
            do {
                try await attempt()
            } catch {
                failure = error
            }
            await MainActor.run { self?.finishRunning(id) }
            await self?.reconcile(
                walletManager: walletManager,
                networkRaw: record.networkRaw,
                walletIds: [record.walletId]
            )
            if let failure {
                throw failure
            }
        }
        let result = walletManager.shieldedFundFromAssetLockCoordinator.startFunding(
            walletId: record.walletId,
            recipientRaw43: record.recipientRaw43,
            operationId: Self.controllerOperationId(id),
            body: body
        )
        // Undo the running count when the body was not scheduled, as the
        // funding coordinator reports it: a blocked start, or a re-tap that
        // re-presents an existing controller. Only a scheduled body says
        // what this controller now runs; the body cannot have started yet,
        // since its task needs the main actor this code holds.
        switch result {
        case .started(let controller, let bodyScheduled):
            if bodyScheduled {
                controller.isResumingExistingLock = isResume
            } else {
                finishRunning(id)
            }
        case .blockedByOtherWalletFunding:
            finishRunning(id)
        }
        return result
    }

    private func finishRunning(_ id: UUID) {
        let remaining = runningCounts[id, default: 0] - 1
        if remaining > 0 {
            runningCounts[id] = remaining
        } else {
            runningCounts.removeValue(forKey: id)
        }
    }

    /// Lists every stored marker. On a failed read it sets `storeError` and
    /// leaves `unfinished` as the last successful read left it; a
    /// successful read clears `storeError`. Internal for tests.
    func refreshFromStore() {
        let records: [WalletDrainOperationRecord]
        do {
            records = try store.records()
        } catch {
            storeError = "Could not read saved Max transfers: \(error.localizedDescription)"
            return
        }
        storeError = nil
        let ids = Set(records.map(\.operationId))
        latest = latest.filter { ids.contains($0.key) }
        markedOperationIds = ids
        // Every marker is listed, read or not: `markedOperationIds` already
        // hides its lock from the generic list, so dropping an unread one
        // would hide the transfer everywhere until a read reports on it.
        unfinished = records
            .sorted { $0.createdAt < $1.createdAt }
            .map { latest[$0.operationId] ?? .unread($0) }
    }
}
