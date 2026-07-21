import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class ShieldedServiceClearTests: XCTestCase {

    private struct ResetFailure: LocalizedError {
        var errorDescription: String? { "injected Rust reset failure" }
    }

    /// Without a manager there is no way to clear the Rust coordinator/tree.
    /// The host rows must survive so a failed reset cannot create split-brain
    /// persistence where SwiftData is empty but the tree file is not.
    func testClearLocalState_withoutWalletManager_preservesRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        let service = ShieldedService()

        await service.clearLocalState(modelContext: context)

        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
        XCTAssertEqual(
            service.lastError,
            "Failed to reset shielded state: no wallet manager is bound."
        )
    }

    /// A throwing `clearShielded()` equivalent is also load-bearing: surface
    /// the error and leave every host row intact for a safe retry.
    func testClearLocalState_whenRustResetThrows_preservesRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        let service = ShieldedService()

        await service.clearLocalState(
            modelContext: context,
            resetRustStateForTesting: { throw ResetFailure() }
        )

        XCTAssertEqual(try fetchSyncStates(in: container).count, 1)
        XCTAssertEqual(
            service.lastError,
            "Failed to reset shielded state: injected Rust reset failure"
        )
    }

    /// Once the Rust reset succeeds, the host wipe proceeds normally.
    func testClearLocalState_afterRustResetSucceeds_deletesRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = try contextWithSyncState(in: container)
        let service = ShieldedService()
        var didResetRustState = false

        await service.clearLocalState(
            modelContext: context,
            resetRustStateForTesting: { didResetRustState = true }
        )

        XCTAssertTrue(didResetRustState)
        XCTAssertTrue(try fetchSyncStates(in: container).isEmpty)
        XCTAssertNil(service.lastError)
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
}
