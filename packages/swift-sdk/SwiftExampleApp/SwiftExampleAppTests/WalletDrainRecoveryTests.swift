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
        var createdAt: Date = .distantPast
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
                submittedAt: .distantFuture,
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
                submittedAt: .distantPast,
                newestFirst: rows
            )
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "drain:not-a-uuid",
                submittedAt: .distantPast,
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
                submittedAt: .distantFuture,
                newestFirst: rows
            ),
            mine
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "resume:dd:0",
                submittedAt: .distantPast,
                newestFirst: rows
            )
        )
    }

    func testShouldMatchAFreshShieldToTheNewestUntaggedLockCreatedSinceItsSubmission() {
        let submittedAt = Date(timeIntervalSince1970: 1_000)
        let newestUnmarked = FakeLock(
            outPointHex: "bb:0",
            operationId: nil,
            createdAt: submittedAt.addingTimeInterval(5)
        )
        let rows = [
            FakeLock(outPointHex: "aa:0", operationId: bytes(UUID()), createdAt: submittedAt.addingTimeInterval(9)),
            newestUnmarked,
            FakeLock(outPointHex: "cc:0", operationId: nil, createdAt: submittedAt),
        ]
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                submittedAt: submittedAt,
                newestFirst: rows
            ),
            newestUnmarked
        )
        // Created in the same instant as the submission still belongs to it.
        let sameInstant = FakeLock(outPointHex: "dd:0", operationId: nil, createdAt: submittedAt)
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                submittedAt: submittedAt,
                newestFirst: [sameInstant]
            ),
            sameInstant
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                submittedAt: submittedAt,
                newestFirst: [FakeLock(outPointHex: "aa:0", operationId: bytes(UUID()), createdAt: submittedAt)]
            )
        )
    }

    func testShouldIgnoreAnUntaggedLockCreatedBeforeAFreshShieldWasSubmitted() {
        let submittedAt = Date(timeIntervalSince1970: 1_000)
        // An older untagged lock, such as a RecoveredFromChain one (status 5),
        // newest by `updatedAt` but created before this attempt.
        let olderUntagged = FakeLock(
            outPointHex: "aa:0",
            operationId: nil,
            createdAt: submittedAt.addingTimeInterval(-1)
        )
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                submittedAt: submittedAt,
                newestFirst: [olderUntagged]
            )
        )
        // No submission yet: nothing can belong to the attempt.
        XCTAssertNil(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: nil,
                submittedAt: nil,
                newestFirst: [olderUntagged]
            )
        )
        // The attempt's own lock is matched even when the older one sorts first.
        let mine = FakeLock(outPointHex: "bb:0", operationId: nil, createdAt: submittedAt.addingTimeInterval(2))
        XCTAssertEqual(
            ShieldedFundFromAssetLockProgressSection.matchingLock(
                controllerOperationId: "shield:\(UUID().uuidString)",
                submittedAt: submittedAt,
                newestFirst: [olderUntagged, mine]
            ),
            mine
        )
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
        XCTAssertEqual(steps(.failed("x"), lockStatusRaw: 3), [.done, .done, .skipped, .done, .failed])
    }

    func testShouldFollowTheLockWhileInFlight() {
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: nil), [.active, .pending, .pending, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 0), [.active, .pending, .pending, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 1, seconds: 1), [.done, .active, .pending, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 1, seconds: 10), [.done, .done, .active, .pending, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 1, seconds: 400), [.done, .done, .skipped, .active, .pending])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 2), [.done, .done, .done, .skipped, .active])
        XCTAssertEqual(steps(.inFlight, lockStatusRaw: 3), [.done, .done, .skipped, .done, .active])
    }

    func testShouldMarkTheRecordedLaneWhenCompleted() {
        XCTAssertEqual(steps(.completed, lockStatusRaw: 2), [.done, .done, .done, .skipped, .done])
        XCTAssertEqual(steps(.completed, lockStatusRaw: 3), [.done, .done, .skipped, .done, .done])
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
