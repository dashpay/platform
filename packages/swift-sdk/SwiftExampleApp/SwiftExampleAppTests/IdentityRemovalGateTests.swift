//
//  IdentityRemovalGateTests.swift
//  SwiftExampleAppTests
//
//  `IdentityRemovalGate`: the key health sheet's identity removal runs
//  at most once per identity at a time, deletes only on Rust's
//  `.notOwned`, and deletes the row the store holds after the check.
//

import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class IdentityRemovalGateTests: XCTestCase {

    private let identityId = Data(repeating: 0x11, count: 32)

    /// An ownership check that waits until the test answers it, standing
    /// in for a slow Platform refresh.
    @MainActor
    private final class PendingCheck {
        private var continuation: CheckedContinuation<ManagedPlatformWallet.IdentityWalletOwnership, Error>?
        private(set) var calls = 0

        var isWaiting: Bool { continuation != nil }

        func check() async throws -> ManagedPlatformWallet.IdentityWalletOwnership {
            calls += 1
            return try await withCheckedThrowingContinuation { continuation = $0 }
        }

        func answer(_ ownership: ManagedPlatformWallet.IdentityWalletOwnership) {
            continuation?.resume(returning: ownership)
            continuation = nil
        }

        func fail() {
            continuation?.resume(throwing: CheckError())
            continuation = nil
        }

        /// Let the removal's task run until it is waiting on the check.
        func untilWaiting() async {
            for _ in 0..<1_000 where !isWaiting {
                await Task.yield()
            }
            XCTAssertTrue(isWaiting, "the removal never reached the check")
        }
    }

    private struct CheckError: Error {}
    private struct DeleteError: Error {}

    /// The store: whether the identity's row is still there, and how
    /// many times it was deleted.
    @MainActor
    private final class Store {
        var hasRow = true
        private(set) var deletes = 0

        func liveRow() -> String? { hasRow ? "row" : nil }

        func delete(_ row: String) -> String {
            deletes += 1
            hasRow = false
            return row
        }
    }

    private func start(
        _ gate: IdentityRemovalGate,
        _ pending: PendingCheck,
        _ store: Store
    ) -> Task<IdentityRemovalOutcome<String>, Never>? {
        gate.start(
            identityId,
            check: { try await pending.check() },
            liveRow: { store.liveRow() },
            delete: { store.delete($0) }
        )
    }

    func testShouldDeleteAtMostOnceWhenRemovalIsConfirmedAgainWhileTheCheckIsPending() async throws {
        let gate = IdentityRemovalGate()
        let pending = PendingCheck()
        let store = Store()

        let first = try XCTUnwrap(start(gate, pending, store))
        XCTAssertTrue(gate.isInFlight(identityId))
        // Confirming again before the task has started, and again while the
        // check is pending, starts nothing.
        XCTAssertNil(start(gate, pending, store))
        await pending.untilWaiting()
        XCTAssertNil(start(gate, pending, store))
        XCTAssertNil(start(gate, pending, store))

        pending.answer(.notOwned)
        guard case .deleted(let row) = await first.value else {
            return XCTFail("expected the first removal to delete")
        }
        XCTAssertEqual(row, "row")
        XCTAssertEqual(store.deletes, 1)
        XCTAssertEqual(pending.calls, 1)
        XCTAssertFalse(gate.isInFlight(identityId))

        // A removal after the first finished finds the row gone and deletes
        // nothing.
        let again = try XCTUnwrap(start(gate, pending, store))
        await pending.untilWaiting()
        pending.answer(.notOwned)
        guard case .alreadyRemoved = await again.value else {
            return XCTFail("expected an already removed row")
        }
        XCTAssertEqual(store.deletes, 1)
        XCTAssertFalse(gate.isInFlight(identityId))
    }

    func testShouldNotDeleteUnlessTheCheckSaysNotOwned() async throws {
        for ownership in [ManagedPlatformWallet.IdentityWalletOwnership.owned, .undetermined] {
            let gate = IdentityRemovalGate()
            let pending = PendingCheck()
            let store = Store()

            let removal = try XCTUnwrap(start(gate, pending, store))
            await pending.untilWaiting()
            pending.answer(ownership)

            guard case .kept(let kept) = await removal.value else {
                return XCTFail("expected \(ownership) to keep the identity")
            }
            XCTAssertEqual(kept, ownership)
            XCTAssertEqual(store.deletes, 0)
            XCTAssertFalse(gate.isInFlight(identityId))
        }
    }

    func testShouldNotDeleteWhenTheCheckFails() async throws {
        let gate = IdentityRemovalGate()
        let pending = PendingCheck()
        let store = Store()

        let removal = try XCTUnwrap(start(gate, pending, store))
        await pending.untilWaiting()
        pending.fail()

        guard case .checkFailed = await removal.value else {
            return XCTFail("expected a failed check")
        }
        XCTAssertEqual(store.deletes, 0)
        XCTAssertFalse(gate.isInFlight(identityId))
    }

    func testShouldTreatARowRemovedDuringTheCheckAsANoOp() async throws {
        let gate = IdentityRemovalGate()
        let pending = PendingCheck()
        let store = Store()

        let removal = try XCTUnwrap(start(gate, pending, store))
        await pending.untilWaiting()
        // The row goes away while the check is pending.
        store.hasRow = false
        pending.answer(.notOwned)

        guard case .alreadyRemoved = await removal.value else {
            return XCTFail("expected an already removed row")
        }
        XCTAssertEqual(store.deletes, 0)
        XCTAssertFalse(gate.isInFlight(identityId))
    }

    func testShouldReleaseTheGuardWhenTheDeleteThrows() async throws {
        let gate = IdentityRemovalGate()
        let pending = PendingCheck()

        let removal = try XCTUnwrap(
            gate.start(
                identityId,
                check: { try await pending.check() },
                liveRow: { "row" },
                delete: { (_: String) throws -> String in throw DeleteError() }
            )
        )
        await pending.untilWaiting()
        pending.answer(.notOwned)

        guard case .deleteFailed = await removal.value else {
            return XCTFail("expected a failed delete")
        }
        XCTAssertFalse(gate.isInFlight(identityId))
    }

    func testShouldRunRemovalsOfDifferentIdentitiesIndependently() async throws {
        let gate = IdentityRemovalGate()
        let pending = PendingCheck()
        let store = Store()
        let otherId = Data(repeating: 0x22, count: 32)

        let first = try XCTUnwrap(start(gate, pending, store))
        let other = try XCTUnwrap(
            gate.start(
                otherId,
                check: { .owned },
                liveRow: { "other" },
                delete: { $0 }
            )
        )
        guard case .kept(.owned) = await other.value else {
            return XCTFail("expected the other identity to be kept")
        }
        XCTAssertTrue(gate.isInFlight(identityId))
        XCTAssertFalse(gate.isInFlight(otherId))

        await pending.untilWaiting()
        pending.answer(.undetermined)
        _ = await first.value
        XCTAssertFalse(gate.isInFlight(identityId))
    }
}
