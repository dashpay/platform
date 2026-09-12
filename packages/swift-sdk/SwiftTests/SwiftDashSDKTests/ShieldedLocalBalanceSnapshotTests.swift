import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

@MainActor
final class ShieldedLocalBalanceSnapshotTests: XCTestCase {
    private static let walletId = Data(repeating: 7, count: 32)

    /// These allocations belong to Swift and are released by the injected
    /// free call. Only nil-message FFI results use the real result destructor.
    private final class NativeFixture: @unchecked Sendable {
        let rows: [ShieldedLocalAccountBalanceFFI]
        let status: ShieldedLocalBalanceStatusFFI
        let code: PlatformWalletFFIResultCode
        let countOverride: UInt?
        let gate: DispatchSemaphore?
        private let lock = NSLock()
        private var recordedEvents: [String] = []
        private var recordedWalletIds: [Data] = []
        private var recordedOffMain: [Bool] = []

        init(
            rows: [ShieldedLocalAccountBalanceFFI] = [],
            status: ShieldedLocalBalanceStatusFFI = SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY,
            code: PlatformWalletFFIResultCode = PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS,
            countOverride: UInt? = nil,
            gate: DispatchSemaphore? = nil
        ) {
            self.rows = rows
            self.status = status
            self.code = code
            self.countOverride = countOverride
            self.gate = gate
        }

        var events: [String] { lock.withLock { recordedEvents } }
        var walletIds: [Data] { lock.withLock { recordedWalletIds } }
        var offMain: [Bool] { lock.withLock { recordedOffMain } }

        func record(_ event: String) {
            lock.withLock { recordedEvents.append(event) }
        }

        func read(handle: Handle, walletId: Data)
            -> (result: PlatformWalletFFIResult, snapshot: ShieldedLocalBalanceSnapshotFFI) {
            lock.withLock {
                recordedEvents.append("read:\(handle)")
                recordedWalletIds.append(walletId)
                recordedOffMain.append(!Thread.isMainThread)
            }
            if let gate {
                // A broken test must not leave the shared native queue hung.
                _ = gate.wait(timeout: .now() + 5)
            }
            let entries: UnsafeMutablePointer<ShieldedLocalAccountBalanceFFI>?
            if rows.isEmpty {
                entries = nil
            } else {
                entries = .allocate(capacity: rows.count)
                for (index, row) in rows.enumerated() {
                    entries?.advanced(by: index).initialize(to: row)
                }
            }
            return (
                PlatformWalletFFIResult(code: code, message: nil),
                ShieldedLocalBalanceSnapshotFFI(
                    status: status,
                    accounts: entries.map { UnsafePointer($0) },
                    accounts_count: countOverride ?? UInt(rows.count)))
        }

        func free(_ snapshot: UnsafeMutablePointer<ShieldedLocalBalanceSnapshotFFI>) {
            if let entries = snapshot.pointee.accounts {
                let allocation = UnsafeMutablePointer(mutating: entries)
                allocation.deinitialize(count: rows.count)
                allocation.deallocate()
            }
            snapshot.pointee = ShieldedLocalBalanceSnapshotFFI()
            record("free")
        }
    }

    private func makeManager(_ fixture: NativeFixture) -> PlatformWalletManager {
        let step: PlatformWalletNativeTeardownCalls.Call = { _ in
            fixture.record("teardown")
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let manager = PlatformWalletManager.makeForTesting(
            handle: Handle.max,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: step, platformAddressSyncStop: step, shieldedSyncStop: step,
                dashPaySyncStop: step, dpnsSyncStop: step, destroy: step))
        manager.nativeShieldedLocalBalanceCalls = PlatformWalletNativeShieldedLocalBalanceCalls(
            read: { handle, walletId in fixture.read(handle: handle, walletId: walletId) },
            free: { snapshot in fixture.free(snapshot) })
        return manager
    }

    private func row(
        _ account: UInt32,
        credits: UInt64 = 0,
        index: UInt64? = nil,
        source: ShieldedBalanceSourceFFI = SHIELDED_BALANCE_SOURCE_FFI_RESTORED
    ) -> ShieldedLocalAccountBalanceFFI {
        ShieldedLocalAccountBalanceFFI(
            account_index: account, spendable_credits: credits,
            last_scanned_index: index ?? 0, has_last_scanned_index: index != nil,
            source: source)
    }

    private func waitForRead(_ fixture: NativeFixture) async throws {
        for _ in 0..<200 {
            if !fixture.walletIds.isEmpty { return }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTFail("Native read did not start")
    }

    func testShouldPreserveAmountsAccountsAndZeroVersusAbsentScanIndex() async throws {
        let fixture = NativeFixture(rows: [
            row(0, source: SHIELDED_BALANCE_SOURCE_FFI_NO_HISTORY),
            row(1, index: 0),
            row(2, credits: UInt64.max),
            row(9, credits: 400, index: 71, source: SHIELDED_BALANCE_SOURCE_FFI_SCANNED_THIS_SESSION)
        ])
        let manager = makeManager(fixture)
        let state = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
        XCTAssertEqual(state, .ready(ShieldedLocalBalanceSnapshot(accounts: [
            0: ShieldedLocalAccountBalance(spendableCredits: 0, lastScannedIndex: nil, source: .noHistory),
            1: ShieldedLocalAccountBalance(spendableCredits: 0, lastScannedIndex: 0, source: .restored),
            2: ShieldedLocalAccountBalance(spendableCredits: UInt64.max, lastScannedIndex: nil, source: .restored),
            9: ShieldedLocalAccountBalance(spendableCredits: 400, lastScannedIndex: 71, source: .scannedThisSession)
        ])))
        XCTAssertEqual(fixture.walletIds, [Self.walletId])
        XCTAssertEqual(fixture.offMain, [true])
        XCTAssertEqual(fixture.events, ["read:\(Handle.max)", "free"])
        await manager.shutdown()
    }

    func testShouldKeepUnboundAndIncompleteStatesDistinct() async throws {
        let cases: [(ShieldedLocalBalanceStatusFFI, ShieldedLocalBalanceState)] = [
            (SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND, .unbound),
            (SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE, .restoreIncomplete)
        ]
        for (status, expected) in cases {
            let fixture = NativeFixture(status: status)
            let manager = makeManager(fixture)
            let state = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTAssertEqual(state, expected)
            XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
            await manager.shutdown()
        }
    }

    func testShouldMapNativeFailureAndReleaseItsOutput() async {
        let fixture = NativeFixture(
            rows: [row(0, credits: 900)], code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION)
        let manager = makeManager(fixture)
        do {
            _ = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTFail("Expected native wallet operation error")
        } catch let error as PlatformWalletError {
            guard case .walletOperation = error else { return XCTFail("Unexpected error: \(error)") }
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
    }

    func testShouldRejectMalformedNativeSnapshotsAndReleaseEachOnce() async {
        let fixtures = [
            NativeFixture(status: ShieldedLocalBalanceStatusFFI(rawValue: 99)),
            NativeFixture(rows: [row(0, source: ShieldedBalanceSourceFFI(rawValue: 99))]),
            NativeFixture(rows: [row(1), row(1)]),
            NativeFixture(countOverride: 1),
            NativeFixture(countOverride: UInt.max),
            NativeFixture(),
            NativeFixture(rows: [row(0)], status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND),
            NativeFixture(rows: [row(0)], status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE)
        ]
        for fixture in fixtures {
            let manager = makeManager(fixture)
            do {
                _ = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
                XCTFail("Malformed native snapshot was accepted")
            } catch let error as PlatformWalletError {
                if case .deserialization = error {} else { XCTFail("Unexpected error: \(error)") }
            } catch {
                XCTFail("Unexpected error: \(error)")
            }
            XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
            await manager.shutdown()
        }
    }

    func testShouldRejectInvalidWalletIdBeforeNativeRead() async {
        let fixture = NativeFixture()
        let manager = makeManager(fixture)
        do {
            _ = try await manager.localShieldedBalanceSnapshot(walletId: Data(repeating: 7, count: 31))
            XCTFail("Expected invalid wallet id error")
        } catch let error as PlatformWalletError {
            if case .invalidParameter = error {} else { XCTFail("Unexpected error: \(error)") }
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
        XCTAssertTrue(fixture.events.isEmpty)
        await manager.shutdown()
    }

    func testShouldDrainAdmittedReadAndRejectNewReadsDuringShutdown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        let shutdown = Task { await manager.shutdown() }
        for _ in 0..<200 {
            if manager.shutdownRequested { break }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTAssertTrue(manager.shutdownRequested)
        XCTAssertEqual(manager.handle, Handle.max, "The native handle must remain live during the read")
        XCTAssertFalse(fixture.events.contains("teardown"))
        do {
            _ = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTFail("New read was admitted during shutdown")
        } catch let error as PlatformWalletError {
            if case .invalidHandle = error {} else { XCTFail("Unexpected error: \(error)") }
        }
        gate.signal()
        let state = try await read.value
        XCTAssertEqual(state, .ready(ShieldedLocalBalanceSnapshot(accounts: [
            0: ShieldedLocalAccountBalance(spendableCredits: 900, lastScannedIndex: nil, source: .restored)
        ])))
        _ = await shutdown.value
        XCTAssertEqual(fixture.walletIds.count, 1)
        XCTAssertEqual(Array(fixture.events.prefix(3)), ["read:\(Handle.max)", "free", "teardown"])
        XCTAssertEqual(manager.handle, NULL_HANDLE)

        do {
            _ = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTFail("Read was admitted after shutdown")
        } catch let error as PlatformWalletError {
            if case .invalidHandle = error {} else { XCTFail("Unexpected error: \(error)") }
        }
        XCTAssertEqual(fixture.walletIds.count, 1)
    }

    func testShouldDiscardSnapshotIfShieldedGenerationChangesBeforeDelivery() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        manager.shieldedSyncGeneration.bump()
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("Obsolete snapshot was returned after clear/stop")
        } catch is CancellationError {
            // The old ledger is deliberately unavailable to the caller.
        }
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
    }

    func testShouldFinishAndReleaseNativeReadBeforeHonoringCancellation() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        read.cancel()
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("Cancelled snapshot was returned")
        } catch is CancellationError {}
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
        XCTAssertEqual(manager.handle, NULL_HANDLE, "Cancellation must release native-op admission")
    }

    func testShouldDiscardSnapshotOnRebindWithoutChangingSyncCallbackGeneration() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let syncGeneration = manager.shieldedSyncGeneration.current()
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        manager.shieldedLocalBalanceGeneration.bump()
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("Snapshot for replaced account set was returned")
        } catch is CancellationError {}
        XCTAssertEqual(manager.shieldedSyncGeneration.current(), syncGeneration)
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
    }

    func testShouldRetainManagerUntilNativeReadAndFreeFinish() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        var manager: PlatformWalletManager? = makeManager(fixture)
        weak var retainedManager: PlatformWalletManager?
        retainedManager = manager
        let read = Task { try await manager!.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        manager = nil
        XCTAssertNotNil(retainedManager, "The admitted method must retain its native manager")
        XCTAssertFalse(fixture.events.contains("teardown"))
        gate.signal()
        _ = try await read.value
        // This deliberately exercises the existing deinit fallback. Only
        // the injected teardown table runs against the fake native handle.
        for _ in 0..<200 {
            if fixture.events.contains("teardown") { break }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTAssertNil(retainedManager)
        XCTAssertEqual(Array(fixture.events.prefix(3)), ["read:\(Handle.max)", "free", "teardown"])
    }

    func testShouldInvalidateQueuedSnapshotWhenNativeBindOrClearFailsAfterMutation() async throws {
        for operation in ["bind", "clear"] {
            let gate = DispatchSemaphore(value: 0)
            let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
            let manager = makeManager(fixture)
            let syncGeneration = manager.shieldedSyncGeneration.current()
            let localGeneration = manager.shieldedLocalBalanceGeneration.current()
            let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
            defer { gate.signal() }
            try await waitForRead(fixture)

            // Exercise the same mutation boundary as bindShielded and
            // clearShielded without passing a fake handle into Rust. Both
            // native paths may alter the ledger before returning an error.
            XCTAssertThrowsError(try manager.withShieldedLocalBalanceMutation { () throws -> Void in
                XCTAssertNotEqual(manager.shieldedLocalBalanceGeneration.current(), localGeneration)
                throw PlatformWalletError.walletOperation("\(operation) failed after mutating the ledger")
            })
            gate.signal()
            do {
                _ = try await read.value
                XCTFail("Old ready snapshot was returned after failed \(operation)")
            } catch is CancellationError {}
            XCTAssertEqual(manager.shieldedSyncGeneration.current(), syncGeneration)
            XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
            await manager.shutdown()
        }
    }
}
