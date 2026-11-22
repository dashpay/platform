import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Tests the Swift side of Core-to-shielded Max transfers: the durable
/// operation-id marker, the rule that decides when a marker is forgotten,
/// the row action (resume only by a known outpoint), the progress lock
/// match and steps, the filter that keeps one asset lock off two resume
/// paths, and the rule that keeps a new Max off mainnet.
final class WalletDrainRecoveryTests: XCTestCase {

    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("WalletDrainRecoveryTests-\(UUID().uuidString)", isDirectory: true)
    }

    override func tearDownWithError() throws {
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

    func testShouldShowInProgressWhileTheControllerIsInFlight() {
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(controllerPhase: .inFlight, outpoint: outpoint),
            .inProgress
        )
        XCTAssertEqual(
            WalletDrainCoordinator.rowAction(controllerPhase: .inFlight, outpoint: nil),
            .inProgress
        )
    }

    func testShouldResumeTheExactOutpointWhenNoAttemptIsRunning() {
        for phase: ShieldedFundFromAssetLockController.Phase? in [nil, .idle, .completed, .failed("x")] {
            XCTAssertEqual(
                WalletDrainCoordinator.rowAction(controllerPhase: phase, outpoint: outpoint),
                .resume(outpoint),
                "phase \(String(describing: phase))"
            )
        }
    }

    func testShouldOnlyCheckAgainWithoutAnOutpoint() {
        for phase: ShieldedFundFromAssetLockController.Phase? in [nil, .idle, .completed, .failed("x")] {
            XCTAssertEqual(
                WalletDrainCoordinator.rowAction(controllerPhase: phase, outpoint: nil),
                .checkAgain,
                "phase \(String(describing: phase))"
            )
        }
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
        for phase: ShieldedFundFromAssetLockController.Phase? in [nil, .idle, .completed, .failed("x")] {
            let action = WalletDrainCoordinator.rowAction(controllerPhase: phase, outpoint: entry.outpoint)
            if case .resume = action {
                XCTFail("an unread marker must not offer Resume (phase \(String(describing: phase)))")
            }
            XCTAssertEqual(action, .checkAgain, "phase \(String(describing: phase))")
        }
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

    func testShouldCountAndPromiseNothingLostOnlyAfterASuccessfulRead() throws {
        XCTAssertNil(UnfinishedWalletDrainsHeading.make(count: 0, storeErrorPresent: false))

        let read = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 2, storeErrorPresent: false))
        XCTAssertEqual(read.title, "Unfinished Max Transfers (2)")
        XCTAssertTrue(read.showsNothingLostNotice)
        XCTAssertNil(read.lastKnownNotice)
        XCTAssertFalse(read.showsRetry)
    }

    func testShouldGiveNoCountOrSafetyClaimWhileTheStoreCannotBeRead() throws {
        // Cold launch: nothing was ever read, so zero rows is not "none".
        let empty = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 0, storeErrorPresent: true))
        XCTAssertEqual(empty.title, "Unfinished Max Transfers")
        XCTAssertFalse(empty.showsNothingLostNotice)
        XCTAssertNil(empty.lastKnownNotice)
        XCTAssertTrue(empty.showsRetry)

        // Rows from an earlier read are labelled as last known.
        let stale = try XCTUnwrap(UnfinishedWalletDrainsHeading.make(count: 3, storeErrorPresent: true))
        XCTAssertEqual(stale.title, "Unfinished Max Transfers")
        XCTAssertFalse(stale.title.contains("3"))
        XCTAssertFalse(stale.showsNothingLostNotice)
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
        seconds: TimeInterval = 0
    ) -> [Progress.StepState] {
        let step = Progress.currentStep(
            phase: phase,
            lockStatusRaw: lockStatusRaw,
            secondsSinceLockUpdate: seconds
        )
        let isFailed: Bool
        if case .failed = phase { isFailed = true } else { isFailed = false }
        return (1...5).map {
            Progress.stepState($0, currentStep: step, isFailed: isFailed, lockStatusRaw: lockStatusRaw)
        }
    }

    func testShouldFailOnStepOneWhenAFailedAttemptHasNoLock() {
        XCTAssertEqual(
            Progress.currentStep(phase: .failed("refused"), lockStatusRaw: nil, secondsSinceLockUpdate: 0),
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
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 2), [.done, .done, .done, .skipped, .failed])
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 3), [.done, .done, .done, .done, .failed])
    }

    /// Once the attempt has stopped the clock must not move its failure:
    /// a Broadcast row fails on the finality wait at any age, including
    /// inside the broadcasting window and past the old InstantSend timeout.
    func testShouldKeepAFailedBroadcastLockOnStepThreeWhateverTheElapsedTime() {
        for seconds: TimeInterval in [0, 1, 2, 10, 299, 300, 400, 86_400] {
            XCTAssertEqual(
                Progress.currentStep(phase: .failed("x"), lockStatusRaw: 1, secondsSinceLockUpdate: seconds),
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
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 2), [.done, .done, .done, .skipped, .active])
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
    /// on step 3, never step 4, and no lane is skipped.
    func testShouldWaitOnStepThreeForAnOldBroadcastLockWhileInFlight() {
        for seconds: TimeInterval in [300, 400, 86_400] {
            XCTAssertEqual(
                Progress.currentStep(phase: .inFlight, lockStatusRaw: 1, secondsSinceLockUpdate: seconds),
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

    func testShouldMarkTheRecordedLaneWhenCompleted() {
        XCTAssertEqual(steps(.completed, lockStatusRaw: 2), [.done, .done, .done, .skipped, .done])
        XCTAssertEqual(steps(.completed, lockStatusRaw: 3), [.done, .done, .done, .done, .done])
    }

    /// RecoveredFromChain (5): the lock is confirmed on Core and only the
    /// shield remains. Its proof is gone, so neither lane is named: both
    /// are passed with the proof kind unknown, never two green checks.
    func testShouldPointAtTheShieldForARecoveredLock() {
        XCTAssertEqual(
            Progress.currentStep(phase: .inFlight, lockStatusRaw: 5, secondsSinceLockUpdate: 0),
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
            Progress.currentStep(phase: .inFlight, lockStatusRaw: 6, secondsSinceLockUpdate: 0),
            6
        )
        XCTAssertEqual(
            Progress.currentStep(phase: .failed("x"), lockStatusRaw: 6, secondsSinceLockUpdate: 0),
            6
        )
        let passed: [Progress.StepState] = [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .done]
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 6), passed)
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 6), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 6), passed)
    }

    /// A completed controller with no row, or whose row still reads Built
    /// (0) or Broadcast (1), does not show which lane ran, so neither is
    /// skipped and neither is claimed.
    func testShouldNotSkipALaneForACompletedAttemptWhoseRowLags() {
        let passed: [Progress.StepState] = [.done, .done, .finalityKindUnknown, .finalityKindUnknown, .done]
        XCTAssertEqual(steps(.completed, lockStatusRaw: nil), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 1), passed)
        XCTAssertEqual(steps(.completed, lockStatusRaw: 0), passed)
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

    /// Every Core-funded submit (Max, resumed lock, fresh shield) goes
    /// through this rule before it starts anything.
    func testShouldRefuseACoreFundingOnlyWhileAnSdkRebuildRuns() {
        XCTAssertEqual(
            ShieldedFundFromAssetLockView.coreFundingRefusal(isRebuildingSdk: true),
            WalletDrainCoordinator.rebuildInProgressMessage
        )
        XCTAssertNil(ShieldedFundFromAssetLockView.coreFundingRefusal(isRebuildingSdk: false))
    }
}
