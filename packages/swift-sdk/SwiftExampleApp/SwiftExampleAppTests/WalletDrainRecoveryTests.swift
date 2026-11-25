import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Tests the Swift side of Core-to-shielded Max transfers: the durable
/// operation-id marker, the rule that decides when a marker is forgotten,
/// the row action (resume only by the outpoint of a `pending` report) and
/// status line, the progress lock match and steps (a resume builds
/// nothing), the filter that keeps one asset lock off two resume
/// paths, and the rule that keeps a new Max off mainnet.
final class WalletDrainRecoveryTests: XCTestCase {

    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("WalletDrainRecoveryTests-\(UUID().uuidString)", isDirectory: true)
    }

    override func tearDownWithError() throws {
        // A test may leave the directory read-only.
        try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: directory.path)
        try? FileManager.default.removeItem(at: directory)
    }

    private func record(
        _ operationId: UUID = UUID(),
        walletId: Data = Data(repeating: 0xA1, count: 32)
    ) -> WalletDrainOperationRecord {
        WalletDrainOperationRecord(
            operationId: operationId,
            walletId: walletId,
            networkRaw: Network.testnet.rawValue,
            fundingAccountIndex: 0,
            recipientRaw43: Data(repeating: 0x5C, count: 43),
            createdAt: Date(timeIntervalSince1970: 1_800_000_000)
        )
    }

    private func bytes(_ uuid: UUID) -> Data {
        withUnsafeBytes(of: uuid.uuid) { Data($0) }
    }

    // MARK: - Store

    func testShouldReadARecordBackFromANewStoreOnTheSameDirectory() throws {
        let stored = record()
        try WalletDrainOperationStore(directory: directory).insert(stored)

        XCTAssertEqual(try WalletDrainOperationStore(directory: directory).records(), [stored])
    }

    func testShouldReadAMissingFileAsNoRecords() throws {
        XCTAssertEqual(try WalletDrainOperationStore(directory: directory).records(), [])
    }

    func testShouldRemoveOnlyTheNamedRecord() throws {
        let store = WalletDrainOperationStore(directory: directory)
        let first = record()
        let second = record()
        try store.insert(first)
        try store.insert(second)

        try store.remove(operationId: first.operationId)

        XCTAssertEqual(try store.records(), [second])
    }

    func testShouldRemoveOnlyTheNamedWalletsRecords() throws {
        let store = WalletDrainOperationStore(directory: directory)
        let deletedWallet = Data(repeating: 0xD1, count: 32)
        let first = record(walletId: deletedWallet)
        let kept = record()
        let second = record(walletId: deletedWallet)
        try store.insert(first)
        try store.insert(kept)
        try store.insert(second)

        try store.remove(walletId: deletedWallet)

        XCTAssertEqual(try store.records(), [kept])
    }

    func testShouldLeaveTheFileUntouchedWhenTheWalletHasNoRecords() throws {
        let store = WalletDrainOperationStore(directory: directory)
        let kept = record()
        try store.insert(kept)

        try store.remove(walletId: Data(repeating: 0xD1, count: 32))

        XCTAssertEqual(try store.records(), [kept])
    }

    func testShouldThrowOnACorruptFileInsteadOfReadingItAsEmpty() throws {
        let store = WalletDrainOperationStore(directory: directory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data("not json".utf8).write(to: store.fileURL)

        XCTAssertThrowsError(try store.records())
        XCTAssertThrowsError(try store.insert(record()))
    }

    func testShouldThrowFromInsertWhenTheDirectoryCannotBeWritten() throws {
        // A regular file where the directory should be: nothing can be
        // created under it.
        try FileManager.default.createDirectory(
            at: directory.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("file".utf8).write(to: directory)
        let store = WalletDrainOperationStore(directory: directory)

        XCTAssertThrowsError(try store.insert(record()))
    }

    // MARK: - Marker rule

    func testShouldForgetTheMarkerWhenRustReportsCompleted() {
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .reported(.completed), attemptOverlappedRead: false),
            .forget
        )
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .reported(.completed), attemptOverlappedRead: true),
            .forget
        )
    }

    func testShouldForgetTheMarkerWhenRustReportsNotSent() {
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .reported(.notSent), attemptOverlappedRead: false),
            .forget
        )
    }

    func testShouldKeepTheMarkerWhenAnAttemptOverlappedANotSentRead() {
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .reported(.notSent), attemptOverlappedRead: true),
            .keep
        )
    }

    func testShouldKeepTheMarkerWhenRustReportsPending() {
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .reported(.pending), attemptOverlappedRead: false),
            .keep
        )
    }

    func testShouldKeepTheMarkerWhenTheStatusReadFailed() {
        XCTAssertEqual(
            WalletDrainCoordinator.markerDecision(for: .failed, attemptOverlappedRead: false),
            .keep
        )
    }

    // MARK: - Attempt result

    func testShouldTreatACompletedOutcomeWithAnErrorAsDone() {
        let outcome = WalletDrainOutcome(
            state: .completed,
            outpoint: nil,
            error: .walletOperation("already consumed")
        )
        XCTAssertNil(WalletDrainCoordinator.attemptError(for: outcome))
    }

    func testShouldTellTheUserToResumeAPendingOutcome() throws {
        let outcome = WalletDrainOutcome(state: .pending, outpoint: nil)
        let error = try XCTUnwrap(WalletDrainCoordinator.attemptError(for: outcome))
        XCTAssertTrue(error.message.contains("resume it from the wallet screen"))
    }

    // MARK: - Row action

    private let outpoint = WalletDrainOutpoint(txidWire: Data(repeating: 0x11, count: 32), vout: 1)

    private let idlePhases: [ShieldedFundFromAssetLockController.Phase?] = [nil, .idle, .completed, .failed("x")]

    func testShouldShowInProgressWhileTheControllerIsInFlight() {
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(controllerPhase: .inFlight, reportedState: .pending, outpoint: outpoint),
            .inProgress
        )
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(controllerPhase: .inFlight, reportedState: nil, outpoint: nil),
            .inProgress
        )
    }

    func testShouldResumeTheExactOutpointOfAPendingReportWhenNoAttemptIsRunning() {
        for phase in idlePhases {
            XCTAssertEqual(
                WalletDrainCoordinator.rowAction(controllerPhase: phase, reportedState: .pending, outpoint: outpoint),
                .resume(outpoint),
                "phase \(String(describing: phase))"
            )
        }
    }

    func testShouldOnlyCheckAgainWithoutAnOutpoint() {
        for phase in idlePhases {
            XCTAssertEqual(
                WalletDrainCoordinator.rowAction(controllerPhase: phase, reportedState: .pending, outpoint: nil),
                .checkAgain,
                "phase \(String(describing: phase))"
            )
        }
    }

    /// A finished operation whose marker is left over, an unread marker and
    /// a failed read never offer a spend, even with an outpoint.
    func testShouldOnlyCheckAgainWithoutAPendingReport() {
        let states: [WalletDrainOutcome.State?] = [nil, .completed, .notSent]
        for phase in idlePhases {
            for state in states {
                XCTAssertEqual(
                    WalletDrainCoordinator.rowAction(controllerPhase: phase, reportedState: state, outpoint: outpoint),
                    .checkAgain,
                    "phase \(String(describing: phase)), state \(String(describing: state))"
                )
            }
        }
    }

    // MARK: - Row status

    private func entry(
        reportedState: WalletDrainOutcome.State? = nil,
        outpoint: WalletDrainOutpoint? = nil,
        readError: String? = nil,
        removalError: String? = nil,
        statusRead: Bool = true
    ) -> WalletDrainCoordinator.Entry {
        WalletDrainCoordinator.Entry(
            record: record(),
            outpoint: outpoint,
            reportedState: reportedState,
            readError: readError,
            removalError: removalError,
            statusRead: statusRead
        )
    }

    private func status(
        _ entry: WalletDrainCoordinator.Entry,
        phase: ShieldedFundFromAssetLockController.Phase? = nil
    ) -> WalletDrainCoordinator.RowStatus {
        WalletDrainCoordinator.rowStatus(controllerPhase: phase, entry: entry)
    }

    func testShouldShowInProgressWhateverTheLastReadWhileAnAttemptRuns() {
        for candidate in [
            entry(statusRead: false),
            entry(reportedState: .pending, outpoint: outpoint),
            entry(readError: "offline"),
            entry(reportedState: .completed, removalError: "disk full"),
        ] {
            XCTAssertEqual(status(candidate, phase: .inFlight), .inProgress)
        }
    }

    func testShouldCallATransferUnfinishedOnlyWhenRustReportedItPending() {
        let pending = status(entry(reportedState: .pending, outpoint: outpoint))
        XCTAssertEqual(pending, .unfinished)
        XCTAssertEqual(
            pending.text,
            "Unfinished: resume to finish it. A new Max cannot start until it does."
        )
        for phase in idlePhases {
            XCTAssertEqual(
                status(entry(reportedState: .pending, outpoint: outpoint), phase: phase),
                .unfinished
            )
        }
        // Pending without a lock: no Resume, so no "resume to finish it".
        let noLock = status(entry(reportedState: .pending))
        XCTAssertEqual(noLock, .unfinishedLockUnknown)
        XCTAssertFalse(noLock.text.contains("resume to finish"))
    }

    func testShouldSayTheStatusIsBeingCheckedBeforeTheFirstRead() {
        let unread = status(entry(statusRead: false))
        XCTAssertEqual(unread, .checking)
        XCTAssertEqual(unread.text, "Checking status…")
    }

    func testShouldShowTheReadErrorWhenTheStatusReadFailed() {
        let failed = status(entry(readError: "offline"))
        XCTAssertEqual(failed, .readFailed("offline"))
        XCTAssertEqual(failed.text, "Status could not be read: offline")
        XCTAssertFalse(failed.text.contains("Unfinished"))
    }

    func testShouldSayACompletedTransferWhoseRecordCouldNotBeClearedIsFinished() {
        let candidate = entry(reportedState: .completed, outpoint: outpoint, removalError: "disk full")
        let finished = status(candidate)
        XCTAssertEqual(finished, .finishedNotCleared("disk full"))
        XCTAssertEqual(finished.text, "Finished; the saved record could not be cleared: disk full")
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(
                controllerPhase: nil,
                reportedState: candidate.reportedState,
                outpoint: candidate.outpoint
            ),
            .checkAgain
        )
    }

    /// Nothing was sent, so the row must not read as finished.
    func testShouldSayANotSentTransferWhoseRecordCouldNotBeClearedWasNotSent() {
        let candidate = entry(reportedState: .notSent, outpoint: outpoint, removalError: "disk full")
        let notSent = status(candidate)
        XCTAssertEqual(notSent, .notSentNotCleared("disk full"))
        XCTAssertEqual(notSent.text, "Not sent; the saved record could not be cleared: disk full")
        XCTAssertFalse(notSent.text.contains("Finished"))
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(
                controllerPhase: nil,
                reportedState: candidate.reportedState,
                outpoint: candidate.outpoint
            ),
            .checkAgain
        )
    }

    func testShouldNotCallANotSentReportKeptForAStartingAttemptUnfinished() {
        let kept = status(entry(reportedState: .notSent))
        XCTAssertEqual(kept, .notSentWhileAttemptStarted)
        XCTAssertFalse(kept.text.contains("Unfinished"))
        XCTAssertFalse(kept.text.contains("A new Max cannot start"))
    }

    /// Cold launch: the marker is on disk and no reconcile has read it yet.
    /// Its lock is already hidden from the generic list, so it must be
    /// listed here, without an outpoint and so without a Resume.
    @MainActor
    func testShouldListASavedMaxTransferBeforeItsStatusIsRead() throws {
        let stored = record()
        try WalletDrainOperationStore(directory: directory).insert(stored)

        let coordinator = WalletDrainCoordinator(store: WalletDrainOperationStore(directory: directory))

        XCTAssertEqual(coordinator.markedOperationIds, [stored.operationId])
        XCTAssertEqual(coordinator.unfinished.count, 1)
        let entry = try XCTUnwrap(coordinator.unfinished.first)
        XCTAssertEqual(entry.record, stored)
        XCTAssertNil(entry.outpoint)
        XCTAssertNil(entry.readError)
        XCTAssertFalse(entry.statusRead)
        XCTAssertNil(entry.reportedState)
        XCTAssertEqual(WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry), .checking)
        for phase in idlePhases {
            let action = WalletDrainCoordinator.rowAction(
                controllerPhase: phase,
                reportedState: entry.reportedState,
                outpoint: entry.outpoint
            )
            if case .resume = action {
                XCTFail("an unread marker must not offer Resume (phase \(String(describing: phase)))")
            }
            XCTAssertEqual(action, .checkAgain, "phase \(String(describing: phase))")
        }
    }

    // MARK: - Reconcile

    @MainActor
    func testShouldRecordAPendingReportWithItsOutpoint() async throws {
        let store = WalletDrainOperationStore(directory: directory)
        let stored = record()
        try store.insert(stored)
        let coordinator = WalletDrainCoordinator(store: store)
        let lock = outpoint

        await coordinator.reconcile(networkRaw: stored.networkRaw, walletIds: [stored.walletId]) { _ in
            WalletDrainOutcome(state: .pending, outpoint: lock)
        }

        let entry = try XCTUnwrap(coordinator.unfinished.first { $0.id == stored.operationId })
        XCTAssertEqual(entry.reportedState, .pending)
        XCTAssertEqual(entry.outpoint, lock)
        XCTAssertEqual(WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry), .unfinished)
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(
                controllerPhase: nil,
                reportedState: entry.reportedState,
                outpoint: entry.outpoint
            ),
            .resume(lock)
        )
    }

    /// A failed read keeps the marker but drops what an earlier read
    /// reported, so the row neither claims the transfer is unfinished nor
    /// offers the old outpoint.
    @MainActor
    func testShouldDropTheEarlierReportWhenALaterStatusReadFails() async throws {
        let store = WalletDrainOperationStore(directory: directory)
        let stored = record()
        try store.insert(stored)
        let coordinator = WalletDrainCoordinator(store: store)
        let lock = outpoint
        await coordinator.reconcile(networkRaw: stored.networkRaw, walletIds: [stored.walletId]) { _ in
            WalletDrainOutcome(state: .pending, outpoint: lock)
        }

        await coordinator.reconcile(networkRaw: stored.networkRaw, walletIds: [stored.walletId]) { _ in
            throw WalletDrainAttemptError(message: "offline")
        }

        let entry = try XCTUnwrap(coordinator.unfinished.first { $0.id == stored.operationId })
        XCTAssertNil(entry.reportedState)
        XCTAssertNil(entry.outpoint)
        XCTAssertEqual(entry.readError, "offline")
        XCTAssertEqual(
            WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry),
            .readFailed("offline")
        )
    }

    /// Rust reported the transfer completed, but the marker file cannot be
    /// rewritten (a read-only file in a read-only directory: it still
    /// reads, but neither an atomic nor a direct rewrite can replace it).
    /// The row keeps Rust's report, is not called unfinished and offers
    /// Check again, which retries the removal.
    @MainActor
    func testShouldKeepRustsTerminalReportWhenTheSavedRecordCannotBeRemoved() async throws {
        let store = WalletDrainOperationStore(directory: directory)
        let stored = record()
        try store.insert(stored)
        let coordinator = WalletDrainCoordinator(store: store)
        try FileManager.default.setAttributes([.posixPermissions: 0o444], ofItemAtPath: store.fileURL.path)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: directory.path)
        XCTAssertThrowsError(
            try store.remove(operationId: stored.operationId),
            "precondition: removal must fail in a read-only directory"
        )
        let lock = outpoint

        await coordinator.reconcile(networkRaw: stored.networkRaw, walletIds: [stored.walletId]) { _ in
            WalletDrainOutcome(state: .completed, outpoint: lock)
        }

        XCTAssertEqual(try store.records(), [stored])
        let entry = try XCTUnwrap(coordinator.unfinished.first { $0.id == stored.operationId })
        XCTAssertEqual(entry.reportedState, .completed)
        XCTAssertNotNil(entry.removalError)
        XCTAssertNil(entry.readError)
        let rowStatus = WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry)
        guard case .finishedNotCleared(let removalError?) = rowStatus else {
            return XCTFail("expected a finished row, got \(rowStatus)")
        }
        XCTAssertEqual(removalError, entry.removalError)
        XCTAssertTrue(rowStatus.text.hasPrefix("Finished; the saved record could not be cleared: "))
        XCTAssertFalse(rowStatus.text.contains("Unfinished"))
        XCTAssertFalse(rowStatus.text.contains("A new Max cannot start"))
        for phase in idlePhases {
            let action = WalletDrainCoordinator.rowAction(
                controllerPhase: phase,
                reportedState: entry.reportedState,
                outpoint: entry.outpoint
            )
            XCTAssertEqual(action, .checkAgain, "phase \(String(describing: phase))")
        }

        // Check again, once the file can be written, clears it.
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: directory.path)
        try FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: store.fileURL.path)
        await coordinator.reconcile(networkRaw: stored.networkRaw, walletIds: [stored.walletId]) { _ in
            WalletDrainOutcome(state: .completed, outpoint: lock)
        }
        XCTAssertEqual(coordinator.unfinished, [])
        XCTAssertEqual(try store.records(), [])
    }

    private struct NeverHeld: Error {}

    /// A status read held open until the test opens it.
    @MainActor
    private final class ReadGate {
        private var continuation: CheckedContinuation<Void, Never>?
        private var isOpen = false

        func hold() async {
            guard !isOpen else { return }
            await withCheckedContinuation { continuation = $0 }
        }

        /// Suspend until a read is held, failing instead of hanging.
        func waitUntilHeld() async throws {
            let deadline = ContinuousClock.now + .seconds(10)
            while continuation == nil {
                guard ContinuousClock.now < deadline else { throw NeverHeld() }
                await Task.yield()
            }
        }

        /// Also covers a read that has not reached `hold()` yet.
        func open() {
            isOpen = true
            continuation?.resume()
            continuation = nil
        }
    }

    /// Launch, wallet-detail and post-attempt reconciles can each wait on a
    /// read of the same operation. Read A starts first and reports
    /// `pending` with an outpoint; read B starts later and reports
    /// `completed`, whose record cannot be removed. B finishes first, then
    /// A: A's older report is dropped, so the row keeps B's report and
    /// offers no Resume.
    @MainActor
    func testShouldKeepTheNewerReportWhenAnOlderStatusReadFinishesLast() async throws {
        let store = WalletDrainOperationStore(directory: directory)
        let stored = record()
        try store.insert(stored)
        let coordinator = WalletDrainCoordinator(store: store)
        let lock = outpoint
        let networkRaw = stored.networkRaw
        let walletId = stored.walletId
        let gate = ReadGate()
        // A failing assertion must not leave read A suspended forever.
        defer { gate.open() }

        let olderRead = Task {
            await coordinator.reconcile(networkRaw: networkRaw, walletIds: [walletId]) { _ in
                await gate.hold()
                return WalletDrainOutcome(state: .pending, outpoint: lock)
            }
        }
        try await gate.waitUntilHeld()

        // As in the terminal-report test: the file still reads but cannot
        // be rewritten, so B's removal fails and its report is kept.
        try FileManager.default.setAttributes([.posixPermissions: 0o444], ofItemAtPath: store.fileURL.path)
        try FileManager.default.setAttributes([.posixPermissions: 0o555], ofItemAtPath: directory.path)
        await coordinator.reconcile(networkRaw: networkRaw, walletIds: [walletId]) { _ in
            WalletDrainOutcome(state: .completed, outpoint: lock)
        }
        let newer = try XCTUnwrap(coordinator.unfinished.first { $0.id == stored.operationId })
        XCTAssertEqual(newer.reportedState, .completed)
        let removalError = try XCTUnwrap(newer.removalError)

        gate.open()
        await olderRead.value

        let entry = try XCTUnwrap(coordinator.unfinished.first { $0.id == stored.operationId })
        XCTAssertEqual(entry, newer)
        XCTAssertEqual(entry.reportedState, .completed)
        XCTAssertEqual(
            WalletDrainCoordinator.rowStatus(controllerPhase: nil, entry: entry),
            .finishedNotCleared(removalError)
        )
        for phase in idlePhases {
            let action = WalletDrainCoordinator.rowAction(
                controllerPhase: phase,
                reportedState: entry.reportedState,
                outpoint: entry.outpoint
            )
            if case .resume = action {
                XCTFail("an older pending read must not bring back Resume (phase \(String(describing: phase)))")
            }
            XCTAssertEqual(action, .checkAgain, "phase \(String(describing: phase))")
        }
        XCTAssertEqual(try store.records(), [stored])
    }

    // MARK: - Unreadable marker file

    private func writeCorruptMarkerFile(for store: WalletDrainOperationStore) throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data("not json".utf8).write(to: store.fileURL)
    }

    @MainActor
    func testShouldSetTheStoreErrorWhenTheMarkerFileIsCorruptAtLaunch() throws {
        let store = WalletDrainOperationStore(directory: directory)
        try writeCorruptMarkerFile(for: store)

        let coordinator = WalletDrainCoordinator(store: store)

        XCTAssertNotNil(coordinator.storeError)
        XCTAssertEqual(coordinator.unfinished, [])
    }

    @MainActor
    func testShouldClearTheStoreErrorWhenARefreshReadsTheFixedFile() throws {
        let store = WalletDrainOperationStore(directory: directory)
        try writeCorruptMarkerFile(for: store)
        let coordinator = WalletDrainCoordinator(store: store)
        XCTAssertNotNil(coordinator.storeError)

        let stored = record()
        try JSONEncoder().encode([stored]).write(to: store.fileURL)
        coordinator.refreshFromStore()

        XCTAssertNil(coordinator.storeError)
        XCTAssertEqual(coordinator.unfinished.map(\.record), [stored])
        XCTAssertEqual(coordinator.markedOperationIds, [stored.operationId])
    }

    @MainActor
    func testShouldClearTheStoreErrorWhenARefreshFindsTheFileDeleted() throws {
        let store = WalletDrainOperationStore(directory: directory)
        try writeCorruptMarkerFile(for: store)
        let coordinator = WalletDrainCoordinator(store: store)
        XCTAssertNotNil(coordinator.storeError)

        try FileManager.default.removeItem(at: store.fileURL)
        coordinator.refreshFromStore()

        XCTAssertNil(coordinator.storeError)
        XCTAssertEqual(coordinator.unfinished, [])
    }

    /// A later failed read keeps the rows an earlier read listed, which is
    /// why the section labels them as last known.
    @MainActor
    func testShouldKeepTheLastKnownRowsWhenALaterReadFails() throws {
        let store = WalletDrainOperationStore(directory: directory)
        let stored = record()
        try store.insert(stored)
        let coordinator = WalletDrainCoordinator(store: store)
        XCTAssertNil(coordinator.storeError)

        try Data("not json".utf8).write(to: store.fileURL)
        coordinator.refreshFromStore()

        XCTAssertNotNil(coordinator.storeError)
        XCTAssertEqual(coordinator.unfinished.map(\.record), [stored])
    }

    /// The count is of saved records: a record can be listed before its
    /// status is read or after Rust reported it finished, so the heading
    /// does not call them unfinished.
    func testShouldCountSavedRecordsOnlyAfterASuccessfulRead() throws {
        XCTAssertNil(UnfinishedWalletDrainsHeading.make(count: 0, storeErrorPresent: false))

        let read = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 2, storeErrorPresent: false))
        XCTAssertEqual(read.title, "Saved Max transfers (2)")
        XCTAssertFalse(read.title.contains("Unfinished"))
        XCTAssertNil(read.lastKnownNotice)
        XCTAssertFalse(read.showsRetry)
    }

    func testShouldGiveNoCountWhileTheStoreCannotBeRead() throws {
        // Cold launch: nothing was ever read, so zero rows is not "none".
        let empty = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 0, storeErrorPresent: true))
        XCTAssertEqual(empty.title, "Saved Max transfers")
        XCTAssertNil(empty.lastKnownNotice)
        XCTAssertTrue(empty.showsRetry)

        // Rows from an earlier read are labelled as last known.
        let stale = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 3, storeErrorPresent: true))
        XCTAssertEqual(stale.title, "Saved Max transfers")
        XCTAssertFalse(stale.title.contains("3"))
        XCTAssertNotNil(stale.lastKnownNotice)
        XCTAssertTrue(stale.showsRetry)
    }

    func testShouldReadTheDrainIdBackFromItsControllerOperationId() {
        let id = UUID()
        XCTAssertEqual(
            WalletDrainCoordinator.drainOperationId(
                fromControllerOperationId: WalletDrainCoordinator.controllerOperationId(id)
            ),
            id
        )
        XCTAssertNil(WalletDrainCoordinator.drainOperationId(fromControllerOperationId: "shield:\(id.uuidString)"))
        XCTAssertNil(WalletDrainCoordinator.drainOperationId(fromControllerOperationId: "drain:not-a-uuid"))
    }

    // MARK: - Progress lock match

    private struct FakeLock: ShieldedProgressLockRow, Equatable {
        let outPointHex: String
        let operationId: Data?
    }

    func testShouldMatchADrainControllerToTheLockWithItsOperationId() {
        let id = UUID()
        let mine = FakeLock(outPointHex: "bb:0", operationId: bytes(id))
        let rows = [
            FakeLock(outPointHex: "aa:0", operationId: nil),
            FakeLock(outPointHex: "cc:0", operationId: bytes(UUID())),
            mine,
        ]
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: WalletDrainCoordinator.controllerOperationId(id),
                builtOutPointHex: "aa:0",
                newestFirst: rows
            ),
            mine
        )
    }

    func testShouldMatchNoLockForADrainWhoseLockIsNotStoredYet() {
        let rows = [
            FakeLock(outPointHex: "aa:0", operationId: nil),
            FakeLock(outPointHex: "cc:0", operationId: bytes(UUID())),
        ]
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: WalletDrainCoordinator.controllerOperationId(UUID()),
                builtOutPointHex: nil,
                newestFirst: rows
            )
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "drain:not-a-uuid",
                builtOutPointHex: nil,
                newestFirst: rows
            )
        )
    }

    func testShouldMatchAResumeControllerToTheLockWithItsOutpoint() {
        let mine = FakeLock(outPointHex: "bb:1", operationId: bytes(UUID()))
        let rows = [FakeLock(outPointHex: "aa:0", operationId: nil), mine]
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "resume:bb:1",
                builtOutPointHex: "aa:0",
                newestFirst: rows
            ),
            mine
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "resume:dd:0",
                builtOutPointHex: nil,
                newestFirst: rows
            )
        )
    }

    func testShouldMatchNoLockForAFreshShieldBeforeItsOutpointIsRecorded() {
        // Untagged rows on either side of the attempt, such as a historical
        // lock first stored during catch-up: none of them is this attempt's.
        let rows = [
            FakeLock(outPointHex: "aa:0", operationId: nil),
            FakeLock(outPointHex: "bb:0", operationId: bytes(UUID())),
            FakeLock(outPointHex: "cc:0", operationId: nil),
        ]
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                builtOutPointHex: nil,
                newestFirst: rows
            )
        )
        // No submission yet: nothing can belong to the attempt.
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: nil,
                builtOutPointHex: nil,
                newestFirst: rows
            )
        )
    }

    func testShouldMatchAFreshShieldOnlyToTheLockAtItsRecordedOutpoint() {
        let mine = FakeLock(outPointHex: "bb:0", operationId: nil)
        // Other untagged rows sort before and after it.
        let rows = [
            FakeLock(outPointHex: "aa:0", operationId: nil),
            mine,
            FakeLock(outPointHex: "cc:0", operationId: nil),
        ]
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                builtOutPointHex: "bb:0",
                newestFirst: rows
            ),
            mine
        )
        // Recorded but its row is not stored yet: still step 1.
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                builtOutPointHex: "dd:0",
                newestFirst: rows
            )
        )
    }

    func testShouldReadBackTheOutpointARecorderWasGiven() {
        let recorder = BuiltOutPointRecorder()
        XCTAssertNil(recorder.outPointHex)

        recorder.record("bb:0")

        XCTAssertEqual(recorder.outPointHex, "bb:0")
    }

    // MARK: - Generic list filter

    func testShouldHideALockWhoseOperationIdHasAMarker() {
        let marked = UUID()
        XCTAssertTrue(
            PendingPlatformFundFromAssetLocksList.isClaimedByDrainMarker(
                operationId: bytes(marked),
                markedOperationIds: [marked]
            )
        )
    }

    func testShouldKeepALockWhoseOperationIdHasNoMarker() {
        XCTAssertFalse(
            PendingPlatformFundFromAssetLocksList.isClaimedByDrainMarker(
                operationId: bytes(UUID()),
                markedOperationIds: [UUID()]
            )
        )
    }

    func testShouldKeepALockWithNoOperationId() {
        XCTAssertFalse(
            PendingPlatformFundFromAssetLocksList.isClaimedByDrainMarker(
                operationId: nil,
                markedOperationIds: [UUID()]
            )
        )
    }

    private func offered(
        fundingType: Int = 5,
        status: Int,
        operationId: Data?,
        marked: Set<UUID> = []
    ) -> Bool {
        PendingPlatformFundFromAssetLocksList.isOfferedForResume(
            fundingTypeRaw: fundingType,
            statusRaw: status,
            operationId: operationId,
            markedOperationIds: marked
        )
    }

    func testShouldOfferAMarkerlessBuiltDrainCheckpoint() {
        XCTAssertTrue(offered(status: 0, operationId: bytes(UUID()), marked: [UUID()]))
    }

    func testShouldHideABuiltDrainCheckpointThatHasAMarker() {
        let id = UUID()
        XCTAssertFalse(offered(status: 0, operationId: bytes(id), marked: [id]))
    }

    func testShouldHideAnOrdinaryBuiltShieldedLock() {
        XCTAssertFalse(offered(status: 0, operationId: nil))
        XCTAssertFalse(offered(status: 0, operationId: Data(repeating: 0x01, count: 15)))
    }

    func testShouldHideABuiltAddressTopUpEvenWithAnOperationId() {
        XCTAssertFalse(offered(fundingType: 4, status: 0, operationId: bytes(UUID())))
    }

    func testShouldKeepTheStatusRuleForLocksPastBuilt() {
        XCTAssertTrue(offered(status: 2, operationId: nil))
        XCTAssertTrue(offered(fundingType: 4, status: 1, operationId: nil))
        XCTAssertTrue(offered(status: 5, operationId: bytes(UUID())))
        XCTAssertFalse(offered(status: 4, operationId: nil))
        XCTAssertFalse(offered(fundingType: 2, status: 2, operationId: nil))
        let id = UUID()
        XCTAssertFalse(offered(status: 2, operationId: bytes(id), marked: [id]))
    }

    // MARK: - Progress steps

    private typealias Progress = ShieldedFundFromAssetLockProgressSection

    private func steps(
        _ phase: ShieldedFundFromAssetLockController.Phase,
        lockStatusRaw: Int?,
        seconds: TimeInterval = 0,
        isResume: Bool = false
    ) -> [Progress.StepState] {
        let step = Progress.currentStep(
            phase: phase,
            lockStatusRaw: lockStatusRaw,
            secondsSinceLockUpdate: seconds,
            isResumingExistingLock: isResume
        )
        let isFailed: Bool
        if case .failed = phase { isFailed = true } else { isFailed = false }
        return (1...5).map {
            Progress.stepState($0, currentStep: step, isFailed: isFailed, lockStatusRaw: lockStatusRaw)
        }
    }

    func testShouldFailOnStepOneWhenAFailedAttemptHasNoLock() {
        XCTAssertEqual(
            Progress.currentStep(
                phase: .failed("refused"),
                lockStatusRaw: nil,
                secondsSinceLockUpdate: 0,
                isResumingExistingLock: false
            ),
            1
        )
        XCTAssertEqual(
            steps(.failed("refused"), lockStatusRaw: nil),
            [.failed, .pending, .pending, .pending, .pending]
        )
    }

    func testShouldFailOnTheStepTheLockReached() {
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 0), [.failed, .pending, .pending, .pending, .pending])
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 1, seconds: 10), [.done, .done, .failed, .pending, .pending])
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 2), [.done, .done, .done, .notAsserted, .failed])
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 3), [.done, .done, .done, .done, .failed])
    }

    /// Once the attempt has stopped the clock must not move its failure:
    /// a Broadcast row fails on the finality wait at any age, including
    /// inside the broadcasting window and past the old InstantSend timeout.
    func testShouldKeepAFailedBroadcastLockOnStepThreeWhateverTheElapsedTime() {
        for seconds: TimeInterval in [0, 1, 2, 10, 299, 300, 400, 86_400] {
            XCTAssertEqual(
                Progress.currentStep(
                    phase: .failed("x"),
                    lockStatusRaw: 1,
                    secondsSinceLockUpdate: seconds,
                    isResumingExistingLock: false
                ),
                3,
                "seconds \(seconds)"
            )
            XCTAssertEqual(
                steps(.failed("x"), lockStatusRaw: 1, seconds: seconds),
                [.done, .done, .failed, .pending, .pending],
                "seconds \(seconds)"
            )
        }
    }

    func testShouldFollowTheLockWhileInFlight() {
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: nil), [.active, .pending, .pending, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 0), [.active, .pending, .pending, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 1, seconds: 10), [.done, .done, .active, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 2), [.done, .done, .done, .notAsserted, .active])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 3), [.done, .done, .done, .done, .active])
    }

    func testShouldShowBroadcastingOnlyForTheFirstWindowOfAFreshBroadcastLock() {
        for seconds: TimeInterval in [0, 1, 1.9] {
            XCTAssertEqual(
                steps(.inFlight, lockStatusRaw: 1, seconds: seconds),
                [.done, .active, .pending, .pending, .pending],
                "seconds \(seconds)"
            )
        }
        XCTAssertEqual(
            steps(.inFlight, lockStatusRaw: 1, seconds: 2),
            [.done, .done, .active, .pending, .pending]
        )
    }

    /// A resumed lock's `updatedAt` is as old as its last status change, so
    /// age says nothing about the finality lane: an old Broadcast row waits
    /// on step 3, never step 4, and no lane is claimed.
    func testShouldWaitOnStepThreeForAnOldBroadcastLockWhileInFlight() {
        for seconds: TimeInterval in [300, 400, 86_400] {
            XCTAssertEqual(
                Progress.currentStep(
                    phase: .inFlight,
                    lockStatusRaw: 1,
                    secondsSinceLockUpdate: seconds,
                    isResumingExistingLock: false
                ),
                3,
                "seconds \(seconds)"
            )
            XCTAssertEqual(
                steps(.inFlight, lockStatusRaw: 1, seconds: seconds),
                [.done, .done, .active, .pending, .pending],
                "seconds \(seconds)"
            )
        }
    }

    /// Step 3 names the proof the row records; at InstantSend step 4 is
    /// neither a check nor skipped, at ChainLock it is done.
    func testShouldTitleTheFinalityStepByTheRecordedProof() {
        XCTAssertEqual(
            Progress.stepTitle(3, isResumingExistingLock: false, lockStatusRaw: 2),
            "InstantSend proof recorded"
        )
        XCTAssertEqual(
            Progress.stepTitle(3, isResumingExistingLock: false, lockStatusRaw: 3),
            "ChainLock proof recorded"
        )
        for status: Int? in [nil, 0, 1, 5, 6] {
            XCTAssertEqual(
                Progress.stepTitle(3, isResumingExistingLock: false, lockStatusRaw: status),
                "Waiting for InstantSend or ChainLock proof",
                "status \(String(describing: status))"
            )
        }
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 3)[3], .done)
        for phase: ShieldedFundFromAssetLockController.Phase in [.inFlight, .failed("x"), .completed] {
            XCTAssertEqual(steps(phase, lockStatusRaw: 2)[3], .notAsserted, "phase \(phase)")
        }
    }

    func testShouldMarkTheRecordedLaneWhenCompleted() {
        XCTAssertEqual(steps(.completed, lockStatusRaw: 2), [.done, .done, .done, .notAsserted, .done])
        XCTAssertEqual(steps(.completed, lockStatusRaw: 3), [.done, .done, .done, .done, .done])
    }

    /// RecoveredFromChain (5): the lock is confirmed on Core and only the
    /// shield remains. Its proof is gone, so neither lane is named: both
    /// are passed with the proof kind unknown, never two green checks.
    func testShouldPointAtTheShieldForARecoveredLock() {
        XCTAssertEqual(
            Progress.currentStep(
                phase: .inFlight,
                lockStatusRaw: 5,
                secondsSinceLockUpdate: 0,
                isResumingExistingLock: false
            ),
            5
        )
        XCTAssertEqual(
            steps(.inFlight, lockStatusRaw: 5),
            [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .active]
        )
        XCTAssertEqual(
            steps(.failed("x"), lockStatusRaw: 5),
            [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .failed]
        )
        XCTAssertEqual(
            steps(.completed, lockStatusRaw: 5),
            [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .done]
        )
    }

    /// DrainCompleted (6): Rust recorded the shield as done, so every step
    /// is passed in any phase, and neither finality lane is named.
    func testShouldPassEveryStepWithoutNamingALaneForACompletedDrainLock() {
        XCTAssertEqual(
            Progress.currentStep(
                phase: .inFlight,
                lockStatusRaw: 6,
                secondsSinceLockUpdate: 0,
                isResumingExistingLock: false
            ),
            6
        )
        XCTAssertEqual(
            Progress.currentStep(
                phase: .failed("x"),
                lockStatusRaw: 6,
                secondsSinceLockUpdate: 0,
                isResumingExistingLock: false
            ),
            6
        )
        let passed: [Progress.StepState] = [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .done]
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 6), passed)
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 6), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 6), passed)
    }

    /// A completed controller with no row, or whose row still reads Built
    /// (0) or Broadcast (1), does not show which lane ran, so neither is
    /// claimed.
    func testShouldNotSkipALaneForACompletedAttemptWhoseRowLags() {
        let passed: [Progress.StepState] = [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .done]
        XCTAssertEqual(steps(.completed, lockStatusRaw: nil), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 1), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 0), passed)
    }

    // MARK: - Resumed lock progress

    func testShouldTreatAResumeIdOrAMarkedControllerAsAResume() {
        let drainId = WalletDrainCoordinator.controllerOperationId(UUID())
        XCTAssertTrue(Progress.isResumingExistingLock(controllerOperationId: "resume:aa:0", markedAsResume: false))
        XCTAssertTrue(Progress.isResumingExistingLock(controllerOperationId: drainId, markedAsResume: true))
        XCTAssertFalse(Progress.isResumingExistingLock(controllerOperationId: drainId, markedAsResume: false))
        XCTAssertFalse(
            Progress.isResumingExistingLock(controllerOperationId: "shield:\(UUID().uuidString)", markedAsResume: false)
        )
        XCTAssertFalse(Progress.isResumingExistingLock(controllerOperationId: nil, markedAsResume: false))
    }

    /// A resume builds nothing: its Built row is already past step 1, and
    /// step 2 resumes that lock. Rust may find it already final and send
    /// nothing, so neither the title nor the footer says it is sent again.
    func testShouldPointAResumeWithABuiltLockAtResumingIt() {
        XCTAssertEqual(
            Progress.currentStep(
                phase: .inFlight,
                lockStatusRaw: 0,
                secondsSinceLockUpdate: 86_400,
                isResumingExistingLock: true
            ),
            2
        )
        XCTAssertEqual(
            steps(.inFlight, lockStatusRaw: 0, isResume: true),
            [.done, .active, .pending, .pending, .pending]
        )
        XCTAssertEqual(
            steps(.failed("x"), lockStatusRaw: 0, isResume: true),
            [.done, .failed, .pending, .pending, .pending]
        )
        XCTAssertEqual(Progress.stepTitle(1, isResumingExistingLock: true, lockStatusRaw: 0), "Asset lock built")
        XCTAssertEqual(
            Progress.stepTitle(2, isResumingExistingLock: true, lockStatusRaw: 0),
            "Resuming the existing asset lock"
        )
        let footer = Progress.footerText(
            step: 2,
            isFailed: false,
            finalityKindUnknown: false,
            isResumingExistingLock: true
        )
        XCTAssertEqual(footer, "The asset lock already exists; no new one is built.")
        for step in 1...6 {
            let text = Progress.footerText(
                step: step,
                isFailed: false,
                finalityKindUnknown: false,
                isResumingExistingLock: true
            )
            XCTAssertFalse(text.contains("from wallet funds"), "step \(step)")
            XCTAssertFalse(text.contains("again"), "step \(step)")
        }
    }

    func testShouldPointAFreshAttemptWithABuiltLockAtBuildingIt() {
        XCTAssertEqual(
            Progress.currentStep(
                phase: .inFlight,
                lockStatusRaw: 0,
                secondsSinceLockUpdate: 0,
                isResumingExistingLock: false
            ),
            1
        )
        XCTAssertEqual(
            steps(.inFlight, lockStatusRaw: 0, isResume: false),
            [.active, .pending, .pending, .pending, .pending]
        )
        XCTAssertEqual(
            Progress.stepTitle(1, isResumingExistingLock: false, lockStatusRaw: 0),
            "Building asset-lock transaction"
        )
        XCTAssertEqual(Progress.stepTitle(2, isResumingExistingLock: false, lockStatusRaw: 0), "Broadcasting")
        XCTAssertEqual(
            Progress.footerText(step: 1, isFailed: false, finalityKindUnknown: false, isResumingExistingLock: false),
            "Building a Core asset-lock transaction from wallet funds."
        )
    }

    /// After Platform rejects a first proof the worker can be waiting for a
    /// ChainLock, so step 5 names that instead of claiming live proving.
    func testShouldDescribeTheShieldStepWithoutClaimingLiveProving() {
        for isResume in [false, true] {
            XCTAssertEqual(
                Progress.stepTitle(5, isResumingExistingLock: isResume, lockStatusRaw: 2),
                "Shielding"
            )
            let footer = Progress.footerText(
                step: 5,
                isFailed: false,
                finalityKindUnknown: false,
                isResumingExistingLock: isResume
            )
            XCTAssertEqual(
                footer,
                "The asset lock has a proof; the wallet is proving the shield (about 30 s) "
                    + "and submitting it to Platform, which can include waiting for a ChainLock if "
                    + "Platform asks for one. The shielded note appears in your balance after the "
                    + "next sync."
            )
        }
    }

    /// Past Built, a resume follows the same lanes as a fresh attempt.
    func testShouldFollowTheSameStepsForAResumePastBuilt() {
        for status in [1, 2, 3, 5, 6] {
            XCTAssertEqual(
                steps(.inFlight, lockStatusRaw: status, seconds: 10, isResume: true),
                steps(.inFlight, lockStatusRaw: status, seconds: 10, isResume: false),
                "status \(status)"
            )
        }
    }

    // MARK: - Resume errors

    func testShouldExplainRecoveryWhenAResumeFindsNoTrackedLock() throws {
        let mapped = WalletDrainCoordinator.resumeError(
            for: PlatformWalletError.assetLockNotTracked("deadbeef:0")
        )
        let error = try XCTUnwrap(mapped as? WalletDrainAttemptError)
        XCTAssertEqual(error.message, WalletDrainCoordinator.assetLockNotTrackedMessage)
        XCTAssertTrue(error.message.contains("not tracking this asset lock right now"))
        XCTAssertTrue(error.message.contains("Restart the app"))
        XCTAssertTrue(error.message.contains("check this transfer again"))
        // A stored row may still be rebroadcast after a restart, so the
        // message must not promise any of these.
        let lowered = error.message.lowercased()
        XCTAssertFalse(lowered.contains("nothing was sent"))
        XCTAssertFalse(lowered.contains("release"))
        XCTAssertFalse(lowered.contains("new max"))
    }

    private struct OtherError: Error, Equatable {
        let code: Int
    }

    func testShouldPassAnyOtherResumeErrorThroughUnchanged() throws {
        let other = OtherError(code: 7)
        XCTAssertEqual(WalletDrainCoordinator.resumeError(for: other) as? OtherError, other)

        let consumed = WalletDrainCoordinator.resumeError(
            for: PlatformWalletError.assetLockAlreadyConsumed("deadbeef:0")
        )
        guard case PlatformWalletError.assetLockAlreadyConsumed(let detail)? = consumed as? PlatformWalletError else {
            return XCTFail("expected the original error, got \(consumed)")
        }
        XCTAssertEqual(detail, "deadbeef:0")
    }

    // MARK: - Mainnet

    func testShouldRefuseANewMaxOnMainnetOnly() {
        XCTAssertFalse(ShieldedFundFromAssetLockView.isNewMaxAllowed(on: .mainnet))
        XCTAssertTrue(ShieldedFundFromAssetLockView.isNewMaxAllowed(on: .testnet))
    }

    // MARK: - SDK rebuild

    /// Every funding submit (Max and its Resume, resumed lock, fresh
    /// shield, Platform address top-up, and identity registration from
    /// any source, fresh or resumed) goes through this rule before it
    /// starts anything.
    func testShouldRefuseAFundingOnlyWhileAnSdkRebuildRuns() {
        XCTAssertEqual(
            AppState.fundingRefusal(isRebuildingSdk: true),
            WalletDrainCoordinator.rebuildInProgressMessage
        )
        XCTAssertNil(AppState.fundingRefusal(isRebuildingSdk: false))
    }
}
