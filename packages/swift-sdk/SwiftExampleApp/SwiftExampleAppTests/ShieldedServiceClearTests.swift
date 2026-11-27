import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class ShieldedServiceClearTests: XCTestCase {

    private struct ResetFailure: LocalizedError {
        var errorDescription: String? { "injected Rust reset failure" }
    }

    /// Without a manager there is no native Clear to run, and the service
    /// deletes nothing itself: the host rows survive and the error surfaces.
    func testClearLocalState_withoutWalletManager_preservesRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(
            PersistentShieldedSyncState(
                walletId: Data(repeating: 0x42, count: 32),
                accountIndex: 0,
                lastSyncedIndex: 123
            )
        )
        try context.save()
        let service = ShieldedService()
        service.isBound = true

        await service.clearLocalState()

        XCTAssertEqual(
            try ModelContext(container).fetch(FetchDescriptor<PersistentShieldedSyncState>()).count,
            1
        )
        XCTAssertFalse(
            service.isBound,
            "A failed clear must require a fresh Rust binding before the next sync"
        )
        XCTAssertEqual(
            service.lastError,
            "Failed to reset shielded state: no wallet manager is bound."
        )
    }

    /// A refusal notice from an earlier Clear must not outlive a later
    /// attempt, whatever that attempt's outcome, or the alert would repeat
    /// an instruction that no longer applies.
    func testClearLocalState_laterAttempt_dropsAnEarlierRefusalNotice() async throws {
        let service = ShieldedService()
        service.clearRefusalNotice = ShieldedService.clearRefusedDuringShieldedOperationMessage

        await service.clearLocalState()

        XCTAssertNil(service.clearRefusalNotice)
        XCTAssertEqual(
            service.lastError,
            "Failed to reset shielded state: no wallet manager is bound.",
            "The later attempt's own failure is reported through lastError"
        )
    }

    /// A throwing `clearShielded()` equivalent surfaces the error, and the
    /// attempt Rust acted on still requires a fresh binding.
    func testClearSequence_whenRustResetThrows_requiresRebind() throws {
        let container = try DashModelContainer.createInMemory()
        try insertSyncState(in: container)
        var resetAttempts = 0

        XCTAssertThrowsError(
            try ShieldedService.executeClearSequence(
                resetRustState: { throw ResetFailure() },
                afterResetAttempt: { resetAttempts += 1 }
            )
        ) { thrownError in
            guard case ShieldedService.ClearLocalStateFailure.rustReset(let error) =
                thrownError
            else {
                return XCTFail("Expected the Rust reset failure to be preserved")
            }
            XCTAssertEqual(error.localizedDescription, "injected Rust reset failure")
        }

        XCTAssertEqual(
            resetAttempts, 1,
            "A reset Rust attempted and failed still requires a fresh binding"
        )
        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
    }

    /// A Clear Rust refused because a shielded operation is in flight changed
    /// nothing, so the host keeps its binding and its rows and shows the
    /// retry text instead of a reset failure.
    func testClearSequence_whenRefusedDuringAShieldedOperation_changesNothing() throws {
        let container = try DashModelContainer.createInMemory()
        try insertSyncState(in: container)
        var resetAttempts = 0

        XCTAssertThrowsError(
            try ShieldedService.executeClearSequence(
                resetRustState: {
                    throw PlatformWalletError.shieldedOperationInProgress(
                        "A shielded operation is in progress; clear the shielded state once it has finished"
                    )
                },
                afterResetAttempt: { resetAttempts += 1 }
            )
        ) { thrownError in
            guard case ShieldedService.ClearLocalStateFailure.refusedDuringShieldedOperation =
                thrownError
            else {
                return XCTFail("Expected the refusal, not a reset failure")
            }
        }

        XCTAssertEqual(resetAttempts, 0, "A refused Clear must not prepare for a rebind")
        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
        XCTAssertEqual(
            ShieldedService.clearRefusedDuringShieldedOperationMessage,
            "A shielded payment or wallet setup is in progress. Try Clear again when it finishes."
        )
    }

    /// Once the Rust reset succeeds, the service prepares for a rebind.
    func testClearSequence_afterRustResetSucceeds_preparesForRebind() throws {
        var steps: [String] = []

        try ShieldedService.executeClearSequence(
            resetRustState: { steps.append("reset") },
            afterResetAttempt: { steps.append("prepare") }
        )

        XCTAssertEqual(steps, ["reset", "prepare"])
    }

    private func insertSyncState(in container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(
            PersistentShieldedSyncState(
                walletId: Data(repeating: 0x42, count: 32),
                accountIndex: 0,
                lastSyncedIndex: 123
            )
        )
        try context.save()
    }

    private func fetchSyncStates(
        in container: ModelContainer
    ) throws -> [PersistentShieldedSyncState] {
        try ModelContext(container).fetch(FetchDescriptor<PersistentShieldedSyncState>())
    }
}
