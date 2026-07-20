import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the ordering invariant of `stopSpvCancellingPendingStarts`: the
/// pending-start generation must already be bumped by the time the stop
/// actually runs, so a start suspended in peer resolution can't resume
/// and revive sync after the stop. Uses the injected-`stop` seam so no
/// real (network-locked) `PlatformWalletManager` is exercised —
/// `WalletManagerStore.init` is cheap (`PlatformWalletManager()` is
/// `public init() {}`), so bare construction against an in-memory
/// container is side-effect-free.
@MainActor
final class WalletManagerStoreStopSpvTests: XCTestCase {

    func testStopSpvCancellingPendingStartsBumpsGenerationBeforeStopping() throws {
        let container = try DashModelContainer.createInMemory()
        let store = WalletManagerStore(modelContainer: container)

        let before = store.spvStartGeneration
        var stopInvocations = 0
        var generationWhenStopRan: UInt64?

        store.stopSpvCancellingPendingStarts(stop: {
            stopInvocations += 1
            // Read the live generation from inside the stop closure: if
            // invalidation is (correctly) ordered first, it already reads
            // the bumped value.
            generationWhenStopRan = store.spvStartGeneration
        })

        XCTAssertEqual(stopInvocations, 1, "stop must be invoked exactly once")
        XCTAssertEqual(
            generationWhenStopRan,
            before &+ 1,
            "generation must already be bumped by the time stop runs"
        )
        XCTAssertEqual(
            store.spvStartGeneration,
            before &+ 1,
            "generation is bumped exactly once"
        )
    }
}
