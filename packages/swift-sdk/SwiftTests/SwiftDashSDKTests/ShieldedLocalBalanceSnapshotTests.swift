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
        let didFree: DispatchSemaphore?
        private let lock = NSLock()
        private var recordedEvents: [String] = []
        private var recordedWalletIds: [Data] = []
        private var recordedOffMain: [Bool] = []
        private var recordedReadTimedOut = false

        init(
            rows: [ShieldedLocalAccountBalanceFFI] = [],
            status: ShieldedLocalBalanceStatusFFI = SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY,
            code: PlatformWalletFFIResultCode = PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS,
            countOverride: UInt? = nil,
            gate: DispatchSemaphore? = nil,
            didFree: DispatchSemaphore? = nil
        ) {
            self.rows = rows
            self.status = status
            self.code = code
            self.countOverride = countOverride
            self.gate = gate
            self.didFree = didFree
        }

        var events: [String] { lock.withLock { recordedEvents } }
        var walletIds: [Data] { lock.withLock { recordedWalletIds } }
        var offMain: [Bool] { lock.withLock { recordedOffMain } }
        var readTimedOut: Bool { lock.withLock { recordedReadTimedOut } }

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
                if gate.wait(timeout: .now() + 5) == .timedOut {
                    lock.withLock { recordedReadTimedOut = true }
                }
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
            didFree?.signal()
        }
    }

    /// Native reads and frees run in FIFO order on the SDK's serial queue.
    private final class NativeSequence: @unchecked Sendable {
        let fixtures: [NativeFixture]
        private let lock = NSLock()
        private var nextRead = 0
        private var nextFree = 0

        init(_ fixtures: [NativeFixture]) { self.fixtures = fixtures }

        var calls: PlatformWalletNativeShieldedLocalBalanceCalls {
            PlatformWalletNativeShieldedLocalBalanceCalls(
                read: { handle, walletId in
                    let fixture = self.lock.withLock {
                        let fixture = self.fixtures[min(self.nextRead, self.fixtures.count - 1)]
                        self.nextRead += 1
                        return fixture
                    }
                    return fixture.read(handle: handle, walletId: walletId)
                },
                free: { snapshot in
                    let fixture = self.lock.withLock {
                        let fixture = self.fixtures[min(self.nextFree, self.fixtures.count - 1)]
                        self.nextFree += 1
                        return fixture
                    }
                    fixture.free(snapshot)
                })
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
                spvStop: step, platformAddressSyncStop: step,
                shieldedSyncStop: { _ in
                    fixture.record("shielded_stop")
                    return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
                },
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
        // Neither an empty reset status nor populated output can override the
        // authoritative native error code. Both remain safe to free once.
        let fixtures = [
            NativeFixture(status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND,
                          code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION),
            NativeFixture(rows: [row(0, credits: 900)],
                          code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION)
        ]
        for fixture in fixtures {
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

    /// Models a native snapshot waiting behind a scan's store lock. Only
    /// stopping shielded sync releases that lock on the successful path.
    func testShouldStopShieldedSyncBeforeDrainingBlockedSnapshotExactlyOnce() async throws {
        let scanLock = DispatchSemaphore(value: 0)
        let allowStopToFinish = DispatchSemaphore(value: 0)
        let stopStarted = expectation(description: "Shielded stop runs while the snapshot is blocked")
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: scanLock)
        let manager = makeManager(fixture)
        let calls = manager.nativeTeardownCalls
        manager.nativeTeardownCalls = PlatformWalletNativeTeardownCalls(
            spvStop: calls.spvStop,
            platformAddressSyncStop: calls.platformAddressSyncStop,
            shieldedSyncStop: { handle in
                XCTAssertEqual(handle, Handle.max)
                XCTAssertFalse(Thread.isMainThread)
                fixture.record("shielded_stop")
                stopStarted.fulfill()
                XCTAssertEqual(allowStopToFinish.wait(timeout: .now() + 5), .success)
                scanLock.signal()
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            },
            dashPaySyncStop: calls.dashPaySyncStop,
            dpnsSyncStop: calls.dpnsSyncStop,
            destroy: calls.destroy)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        try await waitForRead(fixture)
        let first = Task { await manager.shutdown() }
        await fulfillment(of: [stopStarted], timeout: 1)
        let secondStarted = expectation(description: "Concurrent shutdown joins the early stop")
        let second = Task {
            secondStarted.fulfill()
            return await manager.shutdown()
        }
        await fulfillment(of: [secondStarted], timeout: 1)
        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertFalse(manager.isConfigured)
        XCTAssertFalse(fixture.events.contains("free"))
        XCTAssertFalse(fixture.events.contains("teardown"))
        allowStopToFinish.signal()

        let state = try await read.value
        let firstMetrics = await first.value
        let secondMetrics = await second.value

        XCTAssertFalse(fixture.readTimedOut, "Shutdown must release the scan before draining the blocked read")
        XCTAssertEqual(state, .ready(ShieldedLocalBalanceSnapshot(accounts: [
            0: ShieldedLocalAccountBalance(spendableCredits: 900, lastScannedIndex: nil, source: .restored)
        ])))
        XCTAssertEqual(Array(fixture.events.prefix(3)), ["read:\(Handle.max)", "shielded_stop", "free"])
        XCTAssertEqual(fixture.events.filter { $0 == "shielded_stop" }.count, 1)
        XCTAssertEqual(fixture.events.filter { $0 == "teardown" }.count, 5)
        XCTAssertEqual(firstMetrics.steps.map(\.name), secondMetrics.steps.map(\.name))
        XCTAssertEqual(firstMetrics.totalMilliseconds, secondMetrics.totalMilliseconds)
        XCTAssertEqual(manager.handle, NULL_HANDLE)
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
        XCTAssertEqual(manager.handle, NULL_HANDLE, "The admitted read retains its handle privately")
        XCTAssertFalse(fixture.events.contains("teardown"))
        // A completion already queued by early stop, plus late progress,
        // must stay suppressed while this admitted read keeps shutdown open.
        let generation = manager.shieldedSyncGeneration.current()
        manager.handleShieldedSyncCompleted(
            ShieldedSyncEvent(syncUnixSeconds: 1_000, walletResults: []), generation: generation)
        manager.handleShieldedSyncProgress(cumulativeScanned: 10, blockHeight: 20, generation: generation)
        manager.handleShieldedTreeProgress(committed: 30, total: 40, generation: generation)
        XCTAssertNil(manager.lastShieldedSyncEvent)
        XCTAssertNil(manager.currentShieldedSyncScanned)
        XCTAssertNil(manager.currentShieldedSyncBlockHeight)
        XCTAssertNil(manager.currentShieldedTreeCommitted)
        XCTAssertNil(manager.currentShieldedTreeTotal)
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
        let finalTeardown = fixture.events.filter { $0 != "shielded_stop" }
        XCTAssertEqual(Array(finalTeardown.prefix(3)), ["read:\(Handle.max)", "free", "teardown"])
        XCTAssertEqual(fixture.events.filter { $0 == "shielded_stop" }.count, 1)
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

    func testShouldReadAndDrainWhileAnotherManagerIsBlockedInDestroy() async throws {
        let gate = DispatchSemaphore(value: 0)
        let destroyEntered = expectation(description: "Other manager entered native destroy")
        let other = makeManager(NativeFixture())
        let calls = other.nativeTeardownCalls
        other.nativeTeardownCalls = PlatformWalletNativeTeardownCalls(
            spvStop: calls.spvStop,
            platformAddressSyncStop: calls.platformAddressSyncStop,
            shieldedSyncStop: calls.shieldedSyncStop,
            dashPaySyncStop: calls.dashPaySyncStop,
            dpnsSyncStop: calls.dpnsSyncStop,
            destroy: { _ in
                destroyEntered.fulfill()
                XCTAssertEqual(gate.wait(timeout: .now() + 5), .success)
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            })
        let otherShutdown = Task { await other.shutdown() }
        defer { gate.signal() }
        await fulfillment(of: [destroyEntered], timeout: 1)

        let fixture = NativeFixture(rows: [row(0, credits: 900)])
        let manager = makeManager(fixture)
        let delivered = expectation(description: "Snapshot delivered before unrelated destroy finishes")
        let read = Task {
            defer { delivered.fulfill() }
            return try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
        }
        await fulfillment(of: [delivered], timeout: 1)
        let shutdown = Task { await manager.shutdown() }
        for _ in 0..<100 {
            if manager.handle == NULL_HANDLE { break }
            try await Task.sleep(for: .milliseconds(5))
        }
        // Its native teardown may still queue behind the unrelated destroy,
        // but the admitted snapshot and drain must have completed already.
        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        gate.signal()
        _ = try await read.value
        _ = await shutdown.value
        _ = await otherShutdown.value
    }

    func testShouldNotDelayAnotherManagersSnapshotOrCreateBehindARead() async throws {
        let gate = DispatchSemaphore(value: 0)
        let blocked = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let firstManager = makeManager(blocked)
        let firstRead = Task { try await firstManager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(blocked)

        let manager = makeManager(NativeFixture(rows: [row(0, credits: 450)]))
        manager.nativeCreateCalls = PlatformWalletNativeCreateCalls(createFromMnemonic: { _, _ in
            (PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
             NULL_HANDLE, Data(repeating: 8, count: 32))
        })
        let snapshotFinished = expectation(description: "Another manager's snapshot finishes independently")
        let createFinished = expectation(description: "Create is not queued behind the parked snapshot")
        let secondRead = Task {
            defer { snapshotFinished.fulfill() }
            return try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
        }
        let create = Task {
            defer { createFinished.fulfill() }
            return try await manager.createWallet(mnemonic: "test", network: .testnet)
        }
        await fulfillment(of: [snapshotFinished, createFinished], timeout: 1)
        XCTAssertFalse(blocked.events.contains("free"))
        gate.signal()
        _ = try await firstRead.value
        _ = try await secondRead.value
        _ = try await create.value
        await firstManager.shutdown()
        await manager.shutdown()
    }

    func testShouldReportOverlappingBindsWithoutReissuingTheRead() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        manager.withShieldedLocalBalanceBind {}
        manager.withShieldedLocalBalanceBind {}
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("A successful bind must invalidate queued delivery")
        } catch let error as ShieldedLocalBalanceReadError {
            XCTAssertEqual(error, .bindingChanged)
            XCTAssertNotNil(error.errorDescription)
        }
        XCTAssertEqual(fixture.walletIds.count, 1, "The bridge must not decide to retry")
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
    }

    func testShouldAllowCallerToReadFreshStateAfterBindingChanged() async throws {
        // The caller can request a fresh ledger after either an idempotent
        // bind or a changed account set. The bridge must not reissue it itself.
        let replacements: [(UInt32, UInt64)] = [(0, 900), (7, 450)]
        for (replacementAccount, replacementCredits) in replacements {
            let gate = DispatchSemaphore(value: 0)
            let didFree = DispatchSemaphore(value: 0)
            let first = NativeFixture(rows: [row(0, credits: 900)], gate: gate, didFree: didFree)
            let second = NativeFixture(rows: [row(replacementAccount, credits: replacementCredits)])
            let sequence = NativeSequence([first, second])
            let manager = makeManager(first)
            manager.nativeShieldedLocalBalanceCalls = sequence.calls
            let syncGeneration = manager.shieldedSyncGeneration.current()
            let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
            defer { gate.signal() }
            try await waitForRead(first)

            // Model native bind waiting for the read's lifecycle guard:
            // the first read finishes while this synchronous MainActor call
            // is running, so its delivery must occur after bind returns.
            manager.withShieldedLocalBalanceBind {
                gate.signal()
                XCTAssertEqual(didFree.wait(timeout: .now() + 1), .success)
            }
            do {
                _ = try await read.value
                XCTFail("Obsolete snapshot was delivered")
            } catch let error as ShieldedLocalBalanceReadError {
                XCTAssertEqual(error, .bindingChanged)
            }
            XCTAssertTrue(second.walletIds.isEmpty, "No implicit retry")
            let state = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTAssertEqual(state, .ready(ShieldedLocalBalanceSnapshot(accounts: [
                replacementAccount: ShieldedLocalAccountBalance(
                    spendableCredits: replacementCredits, lastScannedIndex: nil, source: .restored)
            ])))
            XCTAssertEqual(manager.shieldedSyncGeneration.current(), syncGeneration)
            XCTAssertEqual(first.events.filter { $0 == "free" }.count, 1)
            XCTAssertEqual(second.events.filter { $0 == "free" }.count, 1)
            await manager.shutdown()
        }
    }

    func testShouldPreferDestructiveCancellationOverBindingChanged() async throws {
        for operation in ["clear", "stop", "failed bind"] {
            let gate = DispatchSemaphore(value: 0)
            let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
            let manager = makeManager(fixture)
            let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
            defer { gate.signal() }
            try await waitForRead(fixture)
            manager.withShieldedLocalBalanceBind {}
            switch operation {
            case "clear":
                manager.withShieldedLocalBalanceMutation {}
            case "stop":
                manager.shieldedSyncGeneration.bump()
            default:
                XCTAssertThrowsError(try manager.withShieldedLocalBalanceBind {
                    throw PlatformWalletError.walletOperation("Bind failed")
                })
            }
            gate.signal()
            do {
                _ = try await read.value
                XCTFail("Obsolete snapshot was delivered after \(operation)")
            } catch is CancellationError {
                // A successful bind must not hide a later destructive change.
            }
            XCTAssertEqual(fixture.walletIds.count, 1)
            XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
            await manager.shutdown()
            XCTAssertEqual(manager.handle, NULL_HANDLE)
        }
    }

    func testShouldDrainBindingChangedReadAndRejectCallerRetryDuringShutdown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)
        manager.withShieldedLocalBalanceBind {}
        let shutdown = Task { await manager.shutdown() }
        for _ in 0..<200 {
            if manager.shutdownRequested { break }
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTAssertTrue(manager.shutdownRequested)
        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertFalse(fixture.events.contains("teardown"))
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("Bind changed during the admitted read")
        } catch let error as ShieldedLocalBalanceReadError {
            XCTAssertEqual(error, .bindingChanged)
        }
        do {
            _ = try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId)
            XCTFail("Shutdown must reject a caller's fresh read")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("Unexpected shutdown error: \(error)")
            }
        }
        _ = await shutdown.value
        XCTAssertEqual(fixture.walletIds.count, 1)
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        XCTAssertEqual(fixture.events.filter { $0 == "shielded_stop" }.count, 1)
        XCTAssertEqual(manager.handle, NULL_HANDLE)
    }

    func testShouldReleaseSynchronousAdmissionAfterNativeBusyFailure() async throws {
        let gate = DispatchSemaphore(value: 0)
        let fixture = NativeFixture(
            code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION, gate: gate)
        let manager = makeManager(fixture)
        let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        defer { gate.signal() }
        try await waitForRead(fixture)

        // A synchronous mutation remains excluded until the bounded native
        // read completes. Invalid seed input stops before the real FFI call.
        XCTAssertThrowsError(try manager.createWallet(seed: Data(), network: .testnet)) { error in
            guard case PlatformWalletError.invalidHandle(let message) = error else {
                return XCTFail("Unexpected admission error: \(error)")
            }
            XCTAssertTrue(message.contains("async native operation"))
        }
        gate.signal()
        do {
            _ = try await read.value
            XCTFail("Expected the native busy failure")
        } catch let error as PlatformWalletError {
            guard case .walletOperation = error else { return XCTFail("Unexpected error: \(error)") }
        }
        XCTAssertThrowsError(try manager.createWallet(seed: Data(), network: .testnet)) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("Synchronous admission stayed closed after the read: \(error)")
            }
        }
        XCTAssertEqual(fixture.events.filter { $0 == "free" }.count, 1)
        await manager.shutdown()
    }

    func testShouldDiscardSnapshotOnClearWithoutChangingSyncCallbackGeneration() async throws {
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
            XCTFail("Snapshot for cleared account set was returned")
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
        let read = Task { [manager] in
            try await manager!.localShieldedBalanceSnapshot(walletId: Self.walletId)
        }
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

    func testShouldDiscardQueuedSnapshotWhenNativeBindOrClearFails() async throws {
        for operation in ["bind", "clear"] {
            let gate = DispatchSemaphore(value: 0)
            let fixture = NativeFixture(rows: [row(0, credits: 900)], gate: gate)
            let manager = makeManager(fixture)
            let syncGeneration = manager.shieldedSyncGeneration.current()
            let localGeneration = manager.shieldedLocalBalanceGeneration.current()
            let read = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
            defer { gate.signal() }
            try await waitForRead(fixture)

            // Exercise the same synchronous boundaries as the public
            // methods without passing a fake handle into Rust.
            let failedMutation = {
                throw PlatformWalletError.walletOperation("\(operation) could not certify the local ledger")
            }
            if operation == "bind" {
                XCTAssertThrowsError(try manager.withShieldedLocalBalanceBind(failedMutation))
            } else {
                XCTAssertThrowsError(try manager.withShieldedLocalBalanceMutation(failedMutation))
            }
            XCTAssertNotEqual(manager.shieldedLocalBalanceGeneration.current(), localGeneration)
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
