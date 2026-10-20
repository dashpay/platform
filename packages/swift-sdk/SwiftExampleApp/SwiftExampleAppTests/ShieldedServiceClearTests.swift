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

        XCTAssertThrowsError(
            try ShieldedService.executeClearPersistenceSequence(
                resetRustState: { throw ResetFailure() },
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
        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
    }

    /// Once the Rust reset succeeds, the host wipe proceeds normally.
    func testClearSequence_afterRustResetSucceeds_deletesRows() throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        var didResetRustState = false

        try ShieldedService.executeClearPersistenceSequence(
            resetRustState: { didResetRustState = true },
            clearHostState: { try clearSyncStates(in: context) }
        )

        XCTAssertTrue(didResetRustState)
        XCTAssertTrue(try fetchSyncStates(in: container).isEmpty)
    }

    /// A host failure after Rust reset must remain distinguishable so the
    /// service can report it while requiring a fresh binding.
    func testClearSequence_whenHostPersistenceThrows_reportsPostResetFailure() throws {
        let container = try DashModelContainer.createInMemory()
        _ = try contextWithSyncState(in: container)
        var didResetRustState = false

        XCTAssertThrowsError(
            try ShieldedService.executeClearPersistenceSequence(
                resetRustState: { didResetRustState = true },
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
