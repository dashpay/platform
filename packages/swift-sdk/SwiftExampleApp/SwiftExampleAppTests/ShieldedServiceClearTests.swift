import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class ShieldedServiceClearTests: XCTestCase {

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
}
