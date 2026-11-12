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
        manager.nativeShieldFromIdentityToRecipientCall = { handle, _, _, _, _, _, _, newBalance in
            XCTAssertEqual(handle, 77)
            entered.fulfill()
            XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            newBalance.pointee = 7
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        let spend = Task {
            try await manager.shieldedShieldFromIdentityToRecipient(
                walletId: Data(repeating: 1, count: 32), identityId: Data(repeating: 2, count: 32),
                recipientRaw43: Data(repeating: 3, count: 43), amount: 1, identitySigner: signer)
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

    /// `clearShielded` would wipe the recovery state the admitted spend is
    /// about to write or has just armed, so it is refused while the spend is
    /// in flight and is no longer refused by the gate once the spend finished.
    func testShouldRejectClearShieldedWhileSpendIsInFlight() async throws {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok)
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: calls)

        let entered = expectation(description: "spend entered the native call")
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        manager.nativeShieldFromIdentityToRecipientCall = { _, _, _, _, _, _, _, newBalance in
            entered.fulfill()
            XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            newBalance.pointee = 7
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let signer = KeychainSigner(modelContainer: try DashModelContainer.createInMemory())
        let spend = Task {
            try await manager.shieldedShieldFromIdentityToRecipient(
                walletId: Data(repeating: 1, count: 32), identityId: Data(repeating: 2, count: 32),
                recipientRaw43: Data(repeating: 3, count: 43), amount: 1, identitySigner: signer)
        }
        await fulfillment(of: [entered], timeout: 5)

        do {
            try manager.clearShielded()
            XCTFail("clearShielded must be rejected while the spend is in flight")
        } catch PlatformWalletError.invalidHandle(let message) {
            XCTAssertTrue(message.contains("in flight"), "clearShielded rejected for another reason: \(message)")
        }

        gate.signal()
        _ = try await spend.value

        // The test handle is not a live native manager, so the FFI clear may
        // fail; it must no longer be the admission gate that refuses it.
        do {
            try manager.clearShielded()
        } catch PlatformWalletError.invalidHandle(let message) {
            XCTAssertFalse(message.contains("in flight"), "clearShielded still gated after the spend: \(message)")
        } catch {}
    }
}
