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
/// Markers are removed only by `reconcile`, from a status Rust returned
/// (one exception: a start the funding coordinator refused before running
/// anything).
@MainActor
final class WalletDrainCoordinator: ObservableObject {
    struct Entry: Identifiable, Equatable {
        let record: WalletDrainOperationRecord
        let outpoint: WalletDrainOutpoint?
        let readError: String?
        var id: UUID { record.operationId }
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

    /// Unfinished operations with their last reported state.
    @Published private(set) var unfinished: [Entry] = []
    /// Every operation id with a stored marker, read or not yet read.
    @Published private(set) var markedOperationIds: Set<UUID> = []
    /// Set when the marker file cannot be read. Shown rather than treated
    /// as "no unfinished transfers".
    @Published private(set) var storeError: String?

    private let store: WalletDrainOperationStore
    private var latest: [UUID: Entry] = [:]
    /// Attempts started per operation, to detect a status read that an
    /// attempt overlapped.
    private var attemptCounts: [UUID: Int] = [:]
    /// Attempts whose body may still be running per operation.
    private var runningCounts: [UUID: Int] = [:]

    init(store: WalletDrainOperationStore = .shared) {
        self.store = store
        refreshFromStore()
    }

    nonisolated static func controllerOperationId(_ operationId: UUID) -> String {
        "drain:\(operationId.uuidString)"
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
    /// `walletIds`, and forgets or lists it by `markerDecision`.
    func reconcile(
        walletManager: PlatformWalletManager,
        networkRaw: UInt32,
        walletIds: [Data]
    ) async {
        let records: [WalletDrainOperationRecord]
        do {
            records = try store.records()
            storeError = nil
        } catch {
            storeError = "Could not read unfinished Max transfers: \(error.localizedDescription)"
            return
        }
        let loaded = Set(walletIds)
        for record in records where record.networkRaw == networkRaw && loaded.contains(record.walletId) {
            let id = record.operationId
            let attemptsBefore = attemptCounts[id, default: 0]
            let runningBefore = runningCounts[id, default: 0] > 0
            let read: StatusRead
            var outpoint: WalletDrainOutpoint?
            var readError: String?
            do {
                let outcome = try await walletManager.shieldedWalletDrainStatus(
                    walletId: record.walletId,
                    operationId: id
                )
                read = .reported(outcome.state)
                outpoint = outcome.outpoint
            } catch {
                read = .failed
                readError = error.localizedDescription
            }
            let overlapped = runningBefore
                || runningCounts[id, default: 0] > 0
                || attemptCounts[id, default: 0] != attemptsBefore
            switch Self.markerDecision(for: read, attemptOverlappedRead: overlapped) {
            case .forget:
                do {
                    try store.remove(operationId: id)
                    latest.removeValue(forKey: id)
                } catch {
                    latest[id] = Entry(
                        record: record,
                        outpoint: outpoint,
                        readError: "Could not clear the finished marker: \(error.localizedDescription)"
                    )
                }
            case .keep:
                latest[id] = Entry(record: record, outpoint: outpoint, readError: readError)
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
        switch run(record, walletManager: walletManager) {
        case .started(let controller):
            latest[record.operationId] = Entry(record: record, outpoint: nil, readError: nil)
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

    /// Runs the stored operation again with the same id and recipient, so
    /// Rust resumes the same asset lock.
    func resume(
        _ record: WalletDrainOperationRecord,
        walletManager: PlatformWalletManager
    ) -> ShieldedFundFromAssetLockCoordinator.StartFundingResult {
        run(record, walletManager: walletManager)
    }

    private func run(
        _ record: WalletDrainOperationRecord,
        walletManager: PlatformWalletManager
    ) -> ShieldedFundFromAssetLockCoordinator.StartFundingResult {
        let id = record.operationId
        attemptCounts[id, default: 0] += 1
        runningCounts[id, default: 0] += 1
        let body: () async throws -> Void = { [weak self] in
            let outcome = await walletManager.shieldedFundFromWalletDrain(
                walletId: record.walletId,
                operationId: id,
                fundingAccountIndex: record.fundingAccountIndex,
                recipientRaw43: record.recipientRaw43
            )
            await MainActor.run { self?.finishRunning(id) }
            await self?.reconcile(
                walletManager: walletManager,
                networkRaw: record.networkRaw,
                walletIds: [record.walletId]
            )
            if let error = Self.attemptError(for: outcome) {
                throw error
            }
        }
        let coordinator = walletManager.shieldedFundFromAssetLockCoordinator
        let prior = coordinator.controller(walletId: record.walletId, recipientRaw43: record.recipientRaw43)
        let priorSubmittedAt = prior?.lastSubmittedAt
        let result = coordinator.startFunding(
            walletId: record.walletId,
            recipientRaw43: record.recipientRaw43,
            operationId: Self.controllerOperationId(id),
            body: body
        )
        // Undo the running count when the body was not accepted: a blocked
        // start, or a re-tap that re-presents an existing controller.
        switch result {
        case .started(let controller):
            let accepted = controller !== prior || controller.lastSubmittedAt != priorSubmittedAt
            if !accepted { finishRunning(id) }
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

    private func refreshFromStore() {
        let records: [WalletDrainOperationRecord]
        do {
            records = try store.records()
        } catch {
            storeError = "Could not read unfinished Max transfers: \(error.localizedDescription)"
            return
        }
        let ids = Set(records.map(\.operationId))
        latest = latest.filter { ids.contains($0.key) }
        markedOperationIds = ids
        unfinished = records
            .sorted { $0.createdAt < $1.createdAt }
            .compactMap { latest[$0.operationId] }
    }
}
