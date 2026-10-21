import XCTest
@testable import SwiftDashSDK

final class PlatformWalletPreparationScopeTests: XCTestCase {
    func testShouldTransferAnExclusiveNativePointerToTheSynchronousWorker() async {
        // FFI wrappers capture isolated native handles. They need the same
        // region-transfer contract as Task.detached, not @Sendable captures.
        let pointer = UnsafeMutablePointer<Int>.allocate(capacity: 1)
        pointer.initialize(to: 42)
        let value = await PlatformWalletTask.detached {
            defer {
                pointer.deinitialize(count: 1)
                pointer.deallocate()
            }
            return pointer.pointee
        }.value
        XCTAssertEqual(value, 42)
    }

    func testShouldKeepOriginalPermitWhenDetachedExecutionIsDelayed() async {
        let original = PlatformWritePermit(instanceID: 1, generation: 1, protocolVersion: 13)
        let refreshed = PlatformWritePermit(instanceID: 1, generation: 2, protocolVersion: 13)
        let gate = DispatchSemaphore(value: 0)
        let queued = PlatformWalletTask.$permits.withValue([original]) {
            PlatformWalletTask.detached {
                gate.wait()
                return PlatformWalletTask.permits
            }
        }
        let newer = PlatformWalletTask.$permits.withValue([refreshed]) {
            PlatformWalletTask.detached { PlatformWalletTask.permits }
        }
        let newerPermits = await newer.value
        gate.signal()
        let originalPermits = await queued.value
        XCTAssertEqual(newerPermits, [refreshed])
        XCTAssertEqual(originalPermits, [original])
        XCTAssertTrue(PlatformWalletTask.permits.isEmpty)
    }

    func testShouldRestoreNestedScopesAndThrowingCallback() async {
        enum Expected: Error { case failure }
        let original = PlatformWritePermit(instanceID: 1, generation: 1, protocolVersion: 13)
        let nested = PlatformWritePermit(instanceID: 2, generation: 1, protocolVersion: 13)
        let failed = PlatformWalletTask.$permits.withValue([original, nested]) {
            PlatformWalletTask.detached { () throws -> Void in
                XCTAssertEqual(PlatformWalletTask.permits, [original, nested])
                throw Expected.failure
            }
        }
        do {
            try await failed.value
            XCTFail("The original callback error must propagate")
        } catch Expected.failure {
        } catch {
            XCTFail("Unexpected callback error: \(error)")
        }
        let unscoped = await PlatformWalletTask.detached { PlatformWalletTask.permits }.value
        XCTAssertTrue(unscoped.isEmpty)
        XCTAssertTrue(PlatformWalletTask.permits.isEmpty)
    }
}
