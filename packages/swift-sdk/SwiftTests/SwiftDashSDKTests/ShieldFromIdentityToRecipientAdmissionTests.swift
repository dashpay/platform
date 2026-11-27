import XCTest
import Combine
import DashSDKFFI
@testable import SwiftDashSDK

/// `shieldedShieldFromIdentityToRecipient` stays admitted as a native
/// operation from its FFI call through result delivery: while the spend runs,
/// the synchronous `deleteWallet` is rejected and `shutdown()` does not
/// destroy the native manager.
@MainActor
final class ShieldFromIdentityToRecipientAdmissionTests: XCTestCase {
    private final class Flag: @unchecked Sendable {
        private let lock = NSLock()
        private var raised = false
        func raise() { lock.withLock { raised = true } }
        var isRaised: Bool { lock.withLock { raised } }
    }

    func testShouldHoldAdmissionWhileSpendIsInFlight() async throws {
        let destroyed = expectation(description: "native destroy ran")
        let destroyedEarly = expectation(description: "native destroy ran during the spend")
        destroyedEarly.isInverted = true
        let spendFinished = Flag()
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok,
            destroy: { _ in
                if !spendFinished.isRaised { destroyedEarly.fulfill() }
                destroyed.fulfill()
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            })
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: calls)

        let entered = expectation(description: "spend entered the native call")
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        manager.nativeShieldFromIdentityToRecipientCall = { handle, wid, acct, ident, rcp, amt, memo, _, newBalance in
            XCTAssertEqual(handle, 77)
            XCTAssertEqual(Data(bytes: wid, count: 32), Data(repeating: 1, count: 32))
            XCTAssertEqual(acct, 5)
            XCTAssertEqual(Data(bytes: ident, count: 32), Data(repeating: 2, count: 32))
            XCTAssertEqual(Data(bytes: rcp, count: 43), Data(repeating: 3, count: 43))
            XCTAssertEqual(amt, 123_456_789)
            XCTAssertNil(memo, "no memo argument must reach the native call as NULL")
            entered.fulfill()
            XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            newBalance.pointee = 7
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        let spend = Task {
            try await manager.shieldedShieldFromIdentityToRecipient(
                walletId: Data(repeating: 1, count: 32), shieldedAccount: 5,
                identityId: Data(repeating: 2, count: 32),
                recipientRaw43: Data(repeating: 3, count: 43), amount: 123_456_789,
                identitySigner: signer)
        }
        await fulfillment(of: [entered], timeout: 5)

        // deleteWallet must be refused by the in-flight gate, not by a later
        // precondition (the test manager has no persistence handler either).
        do {
            try manager.deleteWallet(walletId: Data(repeating: 1, count: 32))
            XCTFail("deleteWallet must be rejected while the spend is in flight")
        } catch PlatformWalletError.invalidHandle(let message) {
            XCTAssertTrue(message.contains("in flight"), "deleteWallet rejected for another reason: \(message)")
        }

        let revoked = expectation(description: "shutdown revoked the public handle")
        let subscription = manager.$isConfigured.dropFirst().sink { if !$0 { revoked.fulfill() } }
        defer { subscription.cancel() }
        let shutdown = Task { await manager.shutdown() }
        await fulfillment(of: [revoked], timeout: 5)
        await fulfillment(of: [destroyedEarly], timeout: 1)

        spendFinished.raise()
        gate.signal()
        let balance = try await spend.value
        XCTAssertEqual(balance, 7)
        _ = await shutdown.value
        await fulfillment(of: [destroyed], timeout: 5)
    }

    /// Swift does not pre-empt a Clear while a shield is in flight: the call
    /// reaches the native Clear, and Rust's refusal (code 84) surfaces as
    /// `shieldedOperationInProgress` with the sync generation and progress
    /// mirrors untouched. Once the shield finished, a Clear Rust accepts goes
    /// through and bumps and resets them.
    func testShouldLetRustDecideClearShieldedWhileShieldIsInFlight() async throws {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok)
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: calls)

        let entered = expectation(description: "shield entered the native call")
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        manager.nativeShieldFromIdentityToRecipientCall = { _, _, _, _, _, _, _, _, newBalance in
            entered.fulfill()
            XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            newBalance.pointee = 7
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        // The message is left nil: `check()` frees it through Rust's
        // allocator, so a fake cannot hand one over.
        let clearCalls = ClearLog()
        let clearCode = ClearCode(PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHIELDED_OPERATION_IN_PROGRESS)
        manager.nativeShieldedClearCall = { handle in
            XCTAssertEqual(handle, 77)
            clearCalls.record()
            return PlatformWalletFFIResult(code: clearCode.value, message: nil)
        }
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        let spend = Task {
            try await manager.shieldedShieldFromIdentityToRecipient(
                walletId: Data(repeating: 1, count: 32), identityId: Data(repeating: 2, count: 32),
                recipientRaw43: Data(repeating: 3, count: 43), amount: 1, identitySigner: signer)
        }
        await fulfillment(of: [entered], timeout: 5)

        manager.currentShieldedSyncScanned = 11
        manager.currentShieldedSyncBlockHeight = 12
        manager.currentShieldedTreeCommitted = 13
        manager.currentShieldedTreeTotal = 14
        let generation = manager.shieldedSyncGeneration.current()

        XCTAssertThrowsError(try manager.clearShielded()) { error in
            guard case .shieldedOperationInProgress = error as? PlatformWalletError else {
                return XCTFail("Expected Rust's refusal, got \(error)")
            }
        }
        XCTAssertEqual(clearCalls.count, 1, "Swift must not pre-empt the native Clear")
        XCTAssertEqual(manager.shieldedSyncGeneration.current(), generation)
        XCTAssertEqual(manager.currentShieldedSyncScanned, 11)
        XCTAssertEqual(manager.currentShieldedSyncBlockHeight, 12)
        XCTAssertEqual(manager.currentShieldedTreeCommitted, 13)
        XCTAssertEqual(manager.currentShieldedTreeTotal, 14)

        gate.signal()
        let balance = try await spend.value
        XCTAssertEqual(balance, 7)

        clearCode.value = PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS
        try manager.clearShielded()
        XCTAssertEqual(clearCalls.count, 2)
        XCTAssertNotEqual(manager.shieldedSyncGeneration.current(), generation)
        XCTAssertNil(manager.currentShieldedSyncScanned)
        XCTAssertNil(manager.currentShieldedSyncBlockHeight)
        XCTAssertNil(manager.currentShieldedTreeCommitted)
        XCTAssertNil(manager.currentShieldedTreeTotal)

        // A Clear that fails after the native reset (a host wipe that
        // failed) drops the callbacks queued before it as well.
        manager.currentShieldedSyncScanned = 21
        manager.currentShieldedTreeCommitted = 23
        let beforeFailure = manager.shieldedSyncGeneration.current()
        clearCode.value = PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_UNKNOWN
        XCTAssertThrowsError(try manager.clearShielded())
        XCTAssertEqual(clearCalls.count, 3)
        XCTAssertNotEqual(manager.shieldedSyncGeneration.current(), beforeFailure)
        XCTAssertNil(manager.currentShieldedSyncScanned)
        XCTAssertNil(manager.currentShieldedTreeCommitted)
    }

    /// The `memo` argument reaches the native call: `nil` and an empty string
    /// as a null pointer, text as the C string.
    func testShouldForwardTheMemoToTheNativeCall() async throws {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok)
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: calls)
        let seen = MemoLog()
        manager.nativeShieldFromIdentityToRecipientCall = { _, _, _, _, _, _, memo, _, _ in
            seen.append(memo.map { String(cString: $0) })
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        for memo in [nil, "", "hi"] as [String?] {
            try await manager.shieldedShieldFromIdentityToRecipient(
                walletId: Data(repeating: 1, count: 32), identityId: Data(repeating: 2, count: 32),
                recipientRaw43: Data(repeating: 3, count: 43), amount: 1, memo: memo,
                identitySigner: signer)
        }
        XCTAssertEqual(seen.values, [nil, nil, "hi"])
    }

    private final class ClearLog: @unchecked Sendable {
        private let lock = NSLock()
        private var calls = 0
        func record() { lock.withLock { calls += 1 } }
        var count: Int { lock.withLock { calls } }
    }

    private final class ClearCode: @unchecked Sendable {
        private let lock = NSLock()
        private var code: PlatformWalletFFIResultCode
        init(_ code: PlatformWalletFFIResultCode) { self.code = code }
        var value: PlatformWalletFFIResultCode {
            get { lock.withLock { code } }
            set { lock.withLock { code = newValue } }
        }
    }

    private final class MemoLog: @unchecked Sendable {
        private let lock = NSLock()
        private var items: [String?] = []
        func append(_ value: String?) { lock.withLock { items.append(value) } }
        var values: [String?] { lock.withLock { items } }
    }
}
