import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class ShieldedServiceClearTests: XCTestCase {

    private struct ResetFailure: LocalizedError {
        var errorDescription: String? { "injected Rust reset failure" }
    }

    private struct HostPersistenceFailure: LocalizedError {
        var errorDescription: String? { "injected host persistence failure" }
    }

    /// Without a manager there is no way to clear the Rust coordinator/tree.
    /// The host rows must survive so a failed reset cannot create split-brain
    /// persistence where SwiftData is empty but the tree file is not.
    func testClearLocalState_withoutWalletManager_preservesRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        let service = ShieldedService()
        service.isBound = true

        await service.clearLocalState(modelContext: context)

        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
        XCTAssertFalse(
            service.isBound,
            "A failed clear must require a fresh Rust binding before the next sync"
        )
        XCTAssertEqual(
            service.lastError,
            "Failed to reset shielded state: no wallet manager is bound."
        )
    }

    /// A throwing `clearShielded()` equivalent is also load-bearing: surface
    /// the error and leave every host row intact for a safe retry.
    func testClearSequence_whenRustResetThrows_preservesRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        var didClearHostState = false
        var resetAttempts = 0

        XCTAssertThrowsError(
            try ShieldedService.executeClearPersistenceSequence(
                resetRustState: { throw ResetFailure() },
                afterResetAttempt: { resetAttempts += 1 },
                clearHostState: {
                    didClearHostState = true
                    try clearSyncStates(in: context)
                }
            )
        ) { thrownError in
            guard case ShieldedService.ClearLocalStateFailure.rustReset(let error) =
                thrownError
            else {
                return XCTFail("Expected the Rust reset failure to be preserved")
            }
            XCTAssertEqual(error.localizedDescription, "injected Rust reset failure")
        }

        XCTAssertFalse(didClearHostState)
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
        let context = try contextWithSyncState(in: container)
        var didClearHostState = false
        var resetAttempts = 0

        XCTAssertThrowsError(
            try ShieldedService.executeClearPersistenceSequence(
                resetRustState: {
                    throw PlatformWalletError.shieldedOperationInProgress(
                        "A shielded operation is in progress; clear the shielded state once it has finished"
                    )
                },
                afterResetAttempt: { resetAttempts += 1 },
                clearHostState: {
                    didClearHostState = true
                    try clearSyncStates(in: context)
                }
            )
        ) { thrownError in
            guard case ShieldedService.ClearLocalStateFailure.refusedDuringShieldedOperation =
                thrownError
            else {
                return XCTFail("Expected the refusal, not a reset failure")
            }
        }

        XCTAssertEqual(resetAttempts, 0, "A refused Clear must not prepare for a rebind")
        XCTAssertFalse(didClearHostState)
        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
        XCTAssertEqual(
            ShieldedService.clearRefusedDuringShieldedOperationMessage,
            "A shielded payment or wallet setup is in progress. Try Clear again when it finishes."
        )
    }

    /// Once the Rust reset succeeds, the host wipe proceeds normally.
    func testClearSequence_afterRustResetSucceeds_deletesRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        var steps: [String] = []

        try ShieldedService.executeClearPersistenceSequence(
            resetRustState: { steps.append("reset") },
            afterResetAttempt: { steps.append("prepare") },
            clearHostState: {
                steps.append("wipe")
                try clearSyncStates(in: context)
            }
        )

        XCTAssertEqual(steps, ["reset", "prepare", "wipe"])
        XCTAssertTrue(try fetchSyncStates(in: container).isEmpty)
    }

    /// A host failure after Rust reset must remain distinguishable so the
    /// service can report it while requiring a fresh binding.
    func testClearSequence_whenHostPersistenceThrows_reportsPostResetFailure() throws {
        let container = try DashModelContainer.createInMemory()
        _ = try contextWithSyncState(in: container)
        var didResetRustState = false
        var resetAttempts = 0

        XCTAssertThrowsError(
            try ShieldedService.executeClearPersistenceSequence(
                resetRustState: { didResetRustState = true },
                afterResetAttempt: { resetAttempts += 1 },
                clearHostState: { throw HostPersistenceFailure() }
            )
        ) { thrownError in
            guard case ShieldedService.ClearLocalStateFailure.hostPersistence(let error) =
                thrownError
            else {
                return XCTFail("Expected the host persistence failure to be preserved")
            }
            XCTAssertEqual(error.localizedDescription, "injected host persistence failure")
        }

        XCTAssertTrue(didResetRustState)
        XCTAssertEqual(resetAttempts, 1)
        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
    }

    private func contextWithSyncState(in container: ModelContainer) throws -> ModelContext {
        let context = ModelContext(container)
        context.insert(
            PersistentShieldedSyncState(
                walletId: Data(repeating: 0x42, count: 32),
                accountIndex: 0,
                lastSyncedIndex: 123
            )
        )
        try context.save()
        return context
    }

    private func fetchSyncStates(
        in container: ModelContainer
    ) throws -> [PersistentShieldedSyncState] {
        try ModelContext(container).fetch(FetchDescriptor<PersistentShieldedSyncState>())
    }

    private func clearSyncStates(in context: ModelContext) throws {
        try context.delete(model: PersistentShieldedSyncState.self)
        try context.save()
    }
}
