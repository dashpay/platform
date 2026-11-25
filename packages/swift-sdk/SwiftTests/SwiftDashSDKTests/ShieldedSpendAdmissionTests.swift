import XCTest
import Combine
import DashSDKFFI
@testable import SwiftDashSDK

/// The shielded spend wrappers on `PlatformWalletManager` (pool spends,
/// shields and the identity-debit abandonment) stay admitted as native
/// operations from before their FFI call until their result is delivered.
/// While one is paused inside its native call, `deleteWallet` is refused and
/// `shutdown()` does not destroy the native manager; a spend started once
/// shutdown has begun never reaches native code.
///
/// Every wrapper runs through its real public body. Only the native exports
/// are replaced, through `nativeShieldedSpendCalls`, so the fake manager
/// handle never reaches Rust.
///
/// Every wait is bounded: a wrapper that leaked its admission would leave
/// `shutdown()` draining forever, and the test must then fail naming that
/// wrapper instead of hanging until the job times out.
@MainActor
final class ShieldedSpendAdmissionTests: XCTestCase {

    private static let walletId = Data(repeating: 0x5A, count: 32)

    /// How long a released spend, or the shutdown queued behind it, may take
    /// to finish before the test fails. Both finish in milliseconds.
    private static let finishTimeout: TimeInterval = 5

    /// Stands in for the shielded spend exports: records which export was
    /// entered with which handle and wallet id, and holds the call until
    /// `release()` when `pauses` is set.
    private final class NativeSpendRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate = DispatchSemaphore(value: 0)
        private let pauses: Bool
        private let onEnter: @Sendable (String) -> Void
        private var released = false
        private var entries: [(name: String, handle: Handle, walletId: Data?)] = []

        init(pauses: Bool = true, onEnter: @escaping @Sendable (String) -> Void = { _ in }) {
            self.pauses = pauses
            self.onEnter = onEnter
        }

        func enter(_ name: String, handle: Handle, walletId: UnsafePointer<UInt8>?) -> PlatformWalletFFIResult {
            let id = walletId.map { Data(bytes: $0, count: 32) }
            lock.withLock { entries.append((name, handle, id)) }
            onEnter(name)
            if pauses {
                XCTAssertEqual(gate.wait(timeout: .now() + 10), .success, "\(name) was never released")
            }
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }

        /// Lets every paused call return. Idempotent, so tests can also call
        /// it from `defer` to unblock a native call after a failed assertion.
        func release() {
            let first = lock.withLock { () -> Bool in
                defer { released = true }
                return !released
            }
            // A generous count: each paused call consumes one signal.
            if first { for _ in 0..<32 { gate.signal() } }
        }

        var isReleased: Bool { lock.withLock { released } }
        var names: [String] { lock.withLock { entries.map(\.name) } }
        var handles: [Handle] { lock.withLock { entries.map(\.handle) } }
        var walletIds: [Data?] { lock.withLock { entries.map(\.walletId) } }
    }

    private final class DestroyCounter: @unchecked Sendable {
        private let lock = NSLock()
        private var count = 0
        private let onDestroy: @Sendable () -> Void
        init(onDestroy: @escaping @Sendable () -> Void = {}) { self.onDestroy = onDestroy }
        func destroy() {
            lock.withLock { count += 1 }
            onDestroy()
        }
        var value: Int { lock.withLock { count } }
    }

    /// Holds the outcome a bounded wait hands back from its watcher task.
    private final class OutcomeBox<T: Sendable>: @unchecked Sendable {
        private let lock = NSLock()
        private var stored: T?
        func set(_ value: T) { lock.withLock { stored = value } }
        var value: T? { lock.withLock { stored } }
    }

    /// Polls `box` until it holds a value or `timeout` passes. A deadline
    /// poll rather than an XCTest expectation: a watcher that finishes after
    /// an expectation's wait timed out would fulfil it late, which XCTest
    /// reports as an API violation.
    private func waitForOutcome<T: Sendable>(
        _ box: OutcomeBox<T>, timeout: TimeInterval
    ) async -> T? {
        let deadline = Date().addingTimeInterval(timeout)
        while box.value == nil, Date() < deadline {
            try? await Task.sleep(for: .milliseconds(10))
        }
        return box.value
    }

    /// Waits at most `timeout` for a throwing `task`. On a timeout the wait
    /// records a failure naming `what` and returns `nil` without awaiting the
    /// unfinished task again.
    private func boundedResult<T: Sendable>(
        of task: Task<T, Error>, _ what: String,
        timeout: TimeInterval = finishTimeout,
        file: StaticString = #filePath, line: UInt = #line
    ) async -> Result<T, Error>? {
        let box = OutcomeBox<Result<T, Error>>()
        Task { box.set(await task.result) }
        guard let outcome = await waitForOutcome(box, timeout: timeout) else {
            XCTFail("\(what) did not finish within \(timeout) s", file: file, line: line)
            return nil
        }
        return outcome
    }

    /// `boundedResult` for a spend expected to succeed: fails naming `what`
    /// when it throws or does not finish. True when it succeeded.
    @discardableResult
    private func expectSpendSucceeds(
        _ task: Task<Void, Error>, _ what: String,
        file: StaticString = #filePath, line: UInt = #line
    ) async -> Bool {
        switch await boundedResult(of: task, what, file: file, line: line) {
        case .success?:
            return true
        case .failure(let error)?:
            XCTFail("\(what) threw \(error)", file: file, line: line)
            return false
        case nil:
            return false
        }
    }

    /// Waits at most `timeout` for a shutdown task, failing with `what` when
    /// it does not finish. Returns its metrics, or `nil` on a timeout.
    private func boundedShutdown(
        _ task: Task<PlatformWalletShutdownMetrics, Never>, _ what: String,
        timeout: TimeInterval = finishTimeout,
        file: StaticString = #filePath, line: UInt = #line
    ) async -> PlatformWalletShutdownMetrics? {
        let box = OutcomeBox<PlatformWalletShutdownMetrics>()
        Task { box.set(await task.value) }
        guard let metrics = await waitForOutcome(box, timeout: timeout) else {
            XCTFail("\(what): shutdown did not finish within \(timeout) s", file: file, line: line)
            return nil
        }
        return metrics
    }

    private nonisolated static func makeTeardownCalls(
        _ counter: DestroyCounter
    ) -> PlatformWalletNativeTeardownCalls {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok,
            destroy: { _ in
                counter.destroy()
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            })
    }

    /// Every export answers through `recorder`. The out-params carry fixed
    /// values so the wrappers' result conversion is checked as well.
    private nonisolated static func makeSpendCalls(
        _ r: NativeSpendRecorder
    ) -> PlatformWalletNativeShieldedSpendCalls {
        PlatformWalletNativeShieldedSpendCalls(
            transfer: { h, w, _, _, _, _, _ in r.enter("transfer", handle: h, walletId: w) },
            shield: { h, w, _, _, _, _ in r.enter("shield", handle: h, walletId: w) },
            shieldFromIdentity: { h, w, _, _, _, _, newBalance in
                let result = r.enter("shieldFromIdentity", handle: h, walletId: w)
                newBalance?.pointee = 7
                return result
            },
            shieldToRecipient: { h, w, _, _, _, _, _, _ in
                r.enter("shieldToRecipient", handle: h, walletId: w)
            },
            unshield: { h, w, _, _, _, _ in r.enter("unshield", handle: h, walletId: w) },
            identityTopUpFromPool: { h, w, _, _, _, _ in
                r.enter("identityTopUpFromPool", handle: h, walletId: w)
            },
            tokenPurchaseFromPool: { h, w, _, _, _, _, _, _ in
                r.enter("tokenPurchaseFromPool", handle: h, walletId: w)
            },
            withdraw: { h, w, _, _, _, _, _ in r.enter("withdraw", handle: h, walletId: w) },
            identityCreateFromPool: { h, w, _, _, _, _, _, _, _, _, outIdentityId in
                let result = r.enter("identityCreateFromPool", handle: h, walletId: w)
                if let outIdentityId {
                    UnsafeMutableRawBufferPointer(start: outIdentityId, count: 32)
                        .copyBytes(from: Array(repeating: UInt8(9), count: 32))
                }
                return result
            },
            abandonIdentityDebit: { h, w, _, _, _ in
                r.enter("abandonIdentityDebit", handle: h, walletId: w)
            }
        )
    }

    private func makeManager(
        handle: Handle,
        recorder: NativeSpendRecorder,
        destroys: DestroyCounter
    ) -> PlatformWalletManager {
        let manager = PlatformWalletManager.makeForTesting(
            handle: handle, calls: Self.makeTeardownCalls(destroys))
        manager.nativeShieldedSpendCalls = Self.makeSpendCalls(recorder)
        return manager
    }

    /// One public wrapper, called with valid arguments, and the native
    /// export it must reach.
    private struct Spend {
        let wrapper: String
        let native: String
        let run: @MainActor @Sendable (PlatformWalletManager) async throws -> Void
    }

    private func allSpends() throws -> [Spend] {
        let resolver = MnemonicResolver()
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        let wid = Self.walletId
        let id32 = Data(repeating: 2, count: 32)
        let raw43 = Data(repeating: 3, count: 43)
        let pubkey = ManagedPlatformWallet.IdentityPubkey(
            keyId: 0, keyType: .ecdsaSecp256k1, purpose: .authentication,
            securityLevel: .master, pubkeyBytes: Data(repeating: 2, count: 33))
        return [
            Spend(wrapper: "shieldedTransfer", native: "transfer") { m in
                try await m.shieldedTransfer(
                    walletId: wid, resolver: resolver, recipientRaw43: raw43, amount: 1, memo: "hi")
            },
            Spend(wrapper: "shieldedShield", native: "shield") { m in
                try await m.shieldedShield(walletId: wid, amount: 1, addressSigner: signer)
            },
            Spend(wrapper: "shieldedShieldFromIdentity", native: "shieldFromIdentity") { m in
                let balance = try await m.shieldedShieldFromIdentity(
                    walletId: wid, identityId: id32, amount: 1, identitySigner: signer)
                XCTAssertEqual(balance, 7, "the proven balance must be returned")
            },
            Spend(wrapper: "shieldedShieldToRecipient", native: "shieldToRecipient") { m in
                try await m.shieldedShieldToRecipient(
                    walletId: wid, recipientRaw43: raw43, amount: 1, addressSigner: signer)
            },
            Spend(wrapper: "shieldedUnshield", native: "unshield") { m in
                try await m.shieldedUnshield(
                    walletId: wid, resolver: resolver, toPlatformAddress: "tdash1test", amount: 1)
            },
            Spend(wrapper: "shieldedIdentityTopUpFromPool", native: "identityTopUpFromPool") { m in
                try await m.shieldedIdentityTopUpFromPool(
                    walletId: wid, resolver: resolver, identityId: id32, amount: 1)
            },
            Spend(wrapper: "shieldedTokenPurchaseFromPool", native: "tokenPurchaseFromPool") { m in
                try await m.shieldedTokenPurchaseFromPool(
                    walletId: wid, resolver: resolver, contractId: id32, tokenPosition: 0,
                    tokenCount: 1, totalAgreedPrice: 1)
            },
            Spend(wrapper: "shieldedWithdraw", native: "withdraw") { m in
                try await m.shieldedWithdraw(
                    walletId: wid, resolver: resolver, toCoreAddress: "yTestAddress", amount: 1)
            },
            Spend(wrapper: "shieldedIdentityCreateFromPool", native: "identityCreateFromPool") { m in
                let identityId = try await m.shieldedIdentityCreateFromPool(
                    walletId: wid, resolver: resolver, identityIndex: 0, identityPubkeys: [pubkey],
                    denomination: 1, sendToAddressOnCreationFailure: Data(repeating: 4, count: 21),
                    identitySigner: signer)
                XCTAssertEqual(identityId, Data(repeating: 9, count: 32), "the new identity id must be returned")
            },
            Spend(wrapper: "abandonShieldedIdentityDebit", native: "abandonIdentityDebit") { m in
                try await m.abandonShieldedIdentityDebit(
                    walletId: wid, accountIndex: 0, activityId: id32,
                    acknowledgePossibleExecution: true)
            },
        ]
    }

    private func assertDeleteRefusedAsInFlight(
        _ manager: PlatformWalletManager, _ what: String,
        file: StaticString = #filePath, line: UInt = #line
    ) {
        XCTAssertThrowsError(try manager.deleteWallet(walletId: Self.walletId), file: file, line: line) { error in
            guard case .invalidHandle(let message, _) = error as? PlatformWalletError else {
                return XCTFail("\(what): expected an in-flight refusal, got \(error)", file: file, line: line)
            }
            XCTAssertTrue(
                message.contains("async native operation is in flight"),
                "\(what): deleteWallet refused for another reason: \(message)", file: file, line: line)
        }
    }

    /// After admission is released, `deleteWallet` passes the in-flight gate
    /// and stops at the persistence handler the test manager lacks.
    private func assertDeleteNoLongerGated(
        _ manager: PlatformWalletManager, _ what: String,
        file: StaticString = #filePath, line: UInt = #line
    ) {
        XCTAssertThrowsError(try manager.deleteWallet(walletId: Self.walletId), file: file, line: line) { error in
            guard case .invalidHandle(let message, _) = error as? PlatformWalletError else {
                return XCTFail("\(what): expected the persistence-handler refusal, got \(error)", file: file, line: line)
            }
            XCTAssertTrue(
                message.contains("persistence handler"),
                "\(what): deleteWallet still gated: \(message)", file: file, line: line)
        }
    }

    // MARK: - Admission held while paused

    /// `shieldedTransfer` paused in its native call keeps the admitted count
    /// above zero, so the synchronous `deleteWallet` is refused before any
    /// native call; once the transfer returns the gate opens again.
    func testShouldRefuseDeleteWalletWhileShieldedTransferIsInFlight() async throws {
        let entered = expectation(description: "transfer entered its native call")
        let recorder = NativeSpendRecorder(onEnter: { _ in entered.fulfill() })
        defer { recorder.release() }
        let manager = makeManager(handle: 0x5A01, recorder: recorder, destroys: DestroyCounter())
        let resolver = MnemonicResolver()

        let spend = Task {
            try await manager.shieldedTransfer(
                walletId: Self.walletId, resolver: resolver,
                recipientRaw43: Data(repeating: 3, count: 43), amount: 5)
        }
        await fulfillment(of: [entered], timeout: 5)
        assertDeleteRefusedAsInFlight(manager, "shieldedTransfer")

        recorder.release()
        guard await expectSpendSucceeds(spend, "shieldedTransfer") else { return }
        XCTAssertEqual(recorder.names, ["transfer"])
        XCTAssertEqual(recorder.handles, [0x5A01])
        XCTAssertEqual(recorder.walletIds, [Self.walletId])
        assertDeleteNoLongerGated(manager, "shieldedTransfer")
    }

    /// For each spend wrapper, paused in its native call: `deleteWallet` is
    /// refused, `shutdown()` revokes the public handle but neither completes
    /// nor runs the native destroy until the spend is released, and after
    /// release the spend returns its result and shutdown completes. A wrapper
    /// that leaked its admission fails here by name within the timeout.
    func testShouldHoldShutdownUntilEveryPausedSpendReturns() async throws {
        for (index, spend) in try allSpends().enumerated() {
            let handle = Handle(0x5B00 + index)
            let entered = expectation(description: "\(spend.wrapper) entered its native call")
            let recorder = NativeSpendRecorder(onEnter: { _ in entered.fulfill() })
            defer { recorder.release() }
            let destroyedEarly = expectation(description: "\(spend.wrapper): destroy ran under the spend")
            destroyedEarly.isInverted = true
            let destroys = DestroyCounter(onDestroy: {
                if !recorder.isReleased { destroyedEarly.fulfill() }
            })
            let manager = makeManager(handle: handle, recorder: recorder, destroys: destroys)

            let run = spend.run
            let call = Task { try await run(manager) }
            await fulfillment(of: [entered], timeout: 5)
            assertDeleteRefusedAsInFlight(manager, spend.wrapper)

            let revoked = expectation(description: "\(spend.wrapper): shutdown revoked the public handle")
            let subscription = manager.$isConfigured.dropFirst().sink { if !$0 { revoked.fulfill() } }
            defer { subscription.cancel() }
            let shutdown = Task { await manager.shutdown() }
            await fulfillment(of: [revoked], timeout: 5)
            await fulfillment(of: [destroyedEarly], timeout: 0.2)
            XCTAssertEqual(destroys.value, 0, "\(spend.wrapper): destroy ran while the spend was admitted")

            recorder.release()
            guard await expectSpendSucceeds(call, spend.wrapper) else { continue }
            guard let metrics = await boundedShutdown(shutdown, spend.wrapper) else { continue }
            XCTAssertEqual(destroys.value, 1, "\(spend.wrapper): shutdown must destroy once after the spend")
            XCTAssertEqual(metrics.steps.last?.name, "destroy")
            XCTAssertEqual(recorder.names, [spend.native], "\(spend.wrapper) reached the wrong export")
            XCTAssertEqual(recorder.handles, [handle])
            XCTAssertEqual(recorder.walletIds, [Self.walletId])
        }
    }

    // MARK: - Rejected once shutdown has begun

    /// With shutdown draining behind a paused transfer, every spend wrapper
    /// throws `invalidHandle` and none reaches its native export.
    func testShouldRejectSpendsStartedAfterShutdownBeganWithoutNativeWork() async throws {
        let entered = expectation(description: "first transfer entered its native call")
        let recorder = NativeSpendRecorder(onEnter: { _ in entered.fulfill() })
        defer { recorder.release() }
        let destroys = DestroyCounter()
        let manager = makeManager(handle: 0x5A02, recorder: recorder, destroys: destroys)
        let spends = try allSpends()

        let first = spends[0].run
        let inFlight = Task { try await first(manager) }
        await fulfillment(of: [entered], timeout: 5)

        let revoked = expectation(description: "shutdown revoked the public handle")
        let subscription = manager.$isConfigured.dropFirst().sink { if !$0 { revoked.fulfill() } }
        defer { subscription.cancel() }
        let shutdown = Task { await manager.shutdown() }
        await fulfillment(of: [revoked], timeout: 5)
        XCTAssertTrue(manager.shutdownRequested)

        for spend in spends {
            let rejected = Task { try await spend.run(manager) }
            switch await boundedResult(of: rejected, "late \(spend.wrapper)") {
            case .success?:
                XCTFail("\(spend.wrapper) must be rejected once shutdown has begun")
            case .failure(let error)?:
                guard case .invalidHandle = error as? PlatformWalletError else {
                    XCTFail("\(spend.wrapper): expected invalidHandle, got \(error)")
                    break
                }
            case nil:
                break
            }
        }
        XCTAssertEqual(recorder.names, ["transfer"], "a rejected spend reached native code")

        recorder.release()
        guard await expectSpendSucceeds(inFlight, "in-flight shieldedTransfer") else { return }
        guard await boundedShutdown(shutdown, "in-flight shieldedTransfer") != nil else { return }
        XCTAssertEqual(destroys.value, 1)
    }

    // MARK: - Cancellation

    /// Cancelling the task awaiting a paused spend does not release its
    /// admission: `deleteWallet` stays refused and shutdown still waits until
    /// the native call returns. The detached native call is not cancelled, so
    /// the spend still completes.
    func testShouldHoldAdmissionAfterTheAwaitingTaskIsCancelled() async throws {
        let entered = expectation(description: "transfer entered its native call")
        let recorder = NativeSpendRecorder(onEnter: { _ in entered.fulfill() })
        defer { recorder.release() }
        let destroyedEarly = expectation(description: "destroy ran under the cancelled spend")
        destroyedEarly.isInverted = true
        let destroys = DestroyCounter(onDestroy: {
            if !recorder.isReleased { destroyedEarly.fulfill() }
        })
        let manager = makeManager(handle: 0x5A03, recorder: recorder, destroys: destroys)
        let resolver = MnemonicResolver()

        let spend = Task {
            try await manager.shieldedTransfer(
                walletId: Self.walletId, resolver: resolver,
                recipientRaw43: Data(repeating: 3, count: 43), amount: 5)
        }
        await fulfillment(of: [entered], timeout: 5)
        spend.cancel()
        for _ in 0..<10 { await Task.yield() }
        assertDeleteRefusedAsInFlight(manager, "cancelled shieldedTransfer")

        let revoked = expectation(description: "shutdown revoked the public handle")
        let subscription = manager.$isConfigured.dropFirst().sink { if !$0 { revoked.fulfill() } }
        defer { subscription.cancel() }
        let shutdown = Task { await manager.shutdown() }
        await fulfillment(of: [revoked], timeout: 5)
        await fulfillment(of: [destroyedEarly], timeout: 0.2)
        XCTAssertEqual(destroys.value, 0)

        recorder.release()
        guard await expectSpendSucceeds(spend, "cancelled shieldedTransfer") else { return }
        guard await boundedShutdown(shutdown, "cancelled shieldedTransfer") != nil else { return }
        XCTAssertEqual(destroys.value, 1)
    }
}
