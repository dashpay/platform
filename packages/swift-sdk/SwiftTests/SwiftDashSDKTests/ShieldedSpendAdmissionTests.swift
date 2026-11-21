import XCTest
import Combine
import DashSDKFFI
@testable import SwiftDashSDK

/// Every shielded payment entry point of `PlatformWalletManager` stays
/// admitted as a native operation from its detached FFI call through result
/// delivery: while it runs, the synchronous `deleteWallet` is rejected and
/// `shutdown()` does not destroy the native manager.
///
/// The plan and the four note spends are driven through
/// `nativeShieldedSpendCalls`, with the native call parked on a semaphore.
/// The other entry points have no seam for their native call; for them the
/// test records `admitNativeOp` through `nativeOpAdmissionObserver` and
/// checks the admission is released when the call returns (the test handle
/// is not a live native manager, so the FFI call itself fails fast).
@MainActor
final class ShieldedSpendAdmissionTests: XCTestCase {
    private static let walletId = Data(repeating: 7, count: 32)
    private nonisolated static var success: PlatformWalletFFIResult {
        PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
    }

    private final class Flag: @unchecked Sendable {
        private let lock = NSLock()
        private var raised = false
        func raise() { lock.withLock { raised = true } }
        var isRaised: Bool { lock.withLock { raised } }
    }

    /// Seam whose every call fails the test; each test replaces the one call
    /// its entry point must reach.
    private nonisolated static func failingSpendCalls() -> PlatformWalletNativeShieldedSpendCalls {
        @Sendable func unexpected(_ name: String) -> PlatformWalletFFIResult {
            XCTFail("unexpected native call \(name)")
            return success
        }
        return PlatformWalletNativeShieldedSpendCalls(
            planSpend: { _, _, _, _, _, _, _, _ in unexpected("planSpend") },
            transfer: { _, _, _, _, _, _, _ in unexpected("transfer") },
            transferWithPlan: { _, _, _, _, _, _, _, _ in unexpected("transferWithPlan") },
            unshield: { _, _, _, _, _, _ in unexpected("unshield") },
            unshieldWithPlan: { _, _, _, _, _, _, _ in unexpected("unshieldWithPlan") },
            topUp: { _, _, _, _, _, _ in unexpected("topUp") },
            topUpWithPlan: { _, _, _, _, _, _, _ in unexpected("topUpWithPlan") },
            withdraw: { _, _, _, _, _, _, _ in unexpected("withdraw") },
            withdrawWithPlan: { _, _, _, _, _, _, _, _ in unexpected("withdrawWithPlan") })
    }

    private func plan(_ kind: ShieldedSpendKind) -> ShieldedSpendPlan {
        ShieldedSpendPlan(
            kind: kind, coreFeePerByte: kind == .withdrawal ? 1 : 0, amount: 1, recipientAmount: 1,
            fee: 2, totalInput: 3, totalDebited: 3, noteCount: 1,
            digest: Data(repeating: 9, count: 32))
    }

    /// Runs `operation` with its native call parked (`park` installs the
    /// parked call, which calls the given closure), then checks the
    /// in-flight window: `deleteWallet` refused as in flight, `shutdown()`
    /// revoking the handle but not destroying it until the call returned.
    private func assertAdmittedWhileParked(
        install: (PlatformWalletManager, @escaping @Sendable () -> Void) -> Void,
        operation: @escaping @MainActor (PlatformWalletManager) async throws -> Void,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async throws {
        let destroyed = expectation(description: "native destroy ran")
        let destroyedEarly = expectation(description: "native destroy ran during the spend")
        destroyedEarly.isInverted = true
        let spendFinished = Flag()
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in Self.success }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok,
            destroy: { _ in
                if !spendFinished.isRaised { destroyedEarly.fulfill() }
                destroyed.fulfill()
                return Self.success
            })
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: calls)
        manager.nativeShieldedSpendCalls = Self.failingSpendCalls()

        let entered = expectation(description: "the entry point reached its native call")
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        install(manager) {
            entered.fulfill()
            XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
        }
        let spend = Task { try await operation(manager) }
        await fulfillment(of: [entered], timeout: 5)

        do {
            try manager.deleteWallet(walletId: Self.walletId)
            XCTFail("deleteWallet must be rejected while the call is in flight", file: file, line: line)
        } catch PlatformWalletError.invalidHandle(let message) {
            XCTAssertTrue(
                message.contains("in flight"), "deleteWallet rejected for another reason: \(message)",
                file: file, line: line)
        }

        let revoked = expectation(description: "shutdown revoked the public handle")
        let subscription = manager.$isConfigured.dropFirst().sink { if !$0 { revoked.fulfill() } }
        defer { subscription.cancel() }
        let shutdown = Task { await manager.shutdown() }
        await fulfillment(of: [revoked], timeout: 5)
        await fulfillment(of: [destroyedEarly], timeout: 1)

        spendFinished.raise()
        gate.signal()
        try await spend.value
        _ = await shutdown.value
        await fulfillment(of: [destroyed], timeout: 5)
    }

    // MARK: - Plan and spends (parked native call)

    func testShouldHoldAdmissionWhilePlanningASpend() async throws {
        try await assertAdmittedWhileParked(
            install: { manager, park in
                manager.nativeShieldedSpendCalls.planSpend = { _, _, _, kind, _, _, _, out in
                    park()
                    out.pointee.kind = kind
                    return Self.success
                }
            },
            operation: { manager in
                _ = try await manager.shieldedPlanSpend(
                    walletId: Self.walletId, kind: .transfer, amount: .exact(1))
            })
    }

    func testShouldHoldAdmissionWhileATransferIsInFlight() async throws {
        try await assertAdmittedWhileParked(
            install: { manager, park in
                manager.nativeShieldedSpendCalls.transferWithPlan = { _, _, _, _, _, _, _, _ in
                    park()
                    return Self.success
                }
            },
            operation: { manager in
                try await manager.shieldedTransfer(
                    walletId: Self.walletId, resolver: MnemonicResolver(),
                    recipientRaw43: Data(repeating: 1, count: 43), amount: 1,
                    plan: self.plan(.transfer))
            })
    }

    func testShouldHoldAdmissionWhileAnUnshieldIsInFlight() async throws {
        try await assertAdmittedWhileParked(
            install: { manager, park in
                manager.nativeShieldedSpendCalls.unshieldWithPlan = { _, _, _, _, _, _, _ in
                    park()
                    return Self.success
                }
            },
            operation: { manager in
                try await manager.shieldedUnshield(
                    walletId: Self.walletId, resolver: MnemonicResolver(),
                    toPlatformAddress: "tdash1x", amount: 1, plan: self.plan(.unshield))
            })
    }

    func testShouldHoldAdmissionWhileAnIdentityTopUpIsInFlight() async throws {
        try await assertAdmittedWhileParked(
            install: { manager, park in
                manager.nativeShieldedSpendCalls.topUpWithPlan = { _, _, _, _, _, _, _ in
                    park()
                    return Self.success
                }
            },
            operation: { manager in
                try await manager.shieldedIdentityTopUpFromPool(
                    walletId: Self.walletId, resolver: MnemonicResolver(),
                    identityId: Data(repeating: 2, count: 32), amount: 1,
                    plan: self.plan(.identityTopUp))
            })
    }

    func testShouldHoldAdmissionWhileAWithdrawalIsInFlight() async throws {
        try await assertAdmittedWhileParked(
            install: { manager, park in
                manager.nativeShieldedSpendCalls.withdrawWithPlan = { _, _, _, _, _, _, _, _ in
                    park()
                    return Self.success
                }
            },
            operation: { manager in
                let plan = self.plan(.withdrawal)
                try await manager.shieldedWithdraw(
                    walletId: Self.walletId, resolver: MnemonicResolver(),
                    toCoreAddress: "yX", amount: 1, coreFeePerByte: plan.coreFeePerByte, plan: plan)
            })
    }

    // MARK: - Entry points without a native-call seam

    /// `operation` must be admitted exactly once under `name`, and the
    /// admission released once it returned, whatever the FFI answered.
    private func assertAdmitted(
        _ name: String,
        file: StaticString = #filePath,
        line: UInt = #line,
        _ operation: (PlatformWalletManager) async throws -> Void
    ) async {
        let manager = PlatformWalletManager.makeForTesting(
            handle: 71,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: { _ in Self.success }, platformAddressSyncStop: { _ in Self.success },
                shieldedSyncStop: { _ in Self.success }, dashPaySyncStop: { _ in Self.success },
                dpnsSyncStop: { _ in Self.success }, destroy: { _ in Self.success }))
        var admitted: [String] = []
        manager.nativeOpAdmissionObserver = { admitted.append($0) }
        _ = try? await operation(manager)
        XCTAssertEqual(admitted, [name], "\(name) was not admitted", file: file, line: line)
        XCTAssertNoThrow(
            try manager.ensureSyncNativeOpAllowed("probe"),
            "\(name) left its admission held", file: file, line: line)
        await manager.shutdown()
    }

    private var signer: KeychainSigner {
        get throws { KeychainSigner(modelContainer: try DashModelContainer.createInMemory()) }
    }

    func testShouldAdmitTheShieldPreflight() async {
        await assertAdmitted("shieldedShieldPreflight") { manager in
            _ = try await manager.shieldedShieldPreflight(walletId: Self.walletId)
        }
    }

    func testShouldAdmitAShield() async throws {
        let signer = try signer
        await assertAdmitted("shieldedShield") { manager in
            try await manager.shieldedShield(walletId: Self.walletId, amount: 1, addressSigner: signer)
        }
    }

    func testShouldAdmitAShieldFromIdentity() async throws {
        let signer = try signer
        await assertAdmitted("shieldedShieldFromIdentity") { manager in
            _ = try await manager.shieldedShieldFromIdentity(
                walletId: Self.walletId, identityId: Data(repeating: 2, count: 32), amount: 1,
                identitySigner: signer)
        }
    }

    func testShouldAdmitAShieldToRecipient() async throws {
        let signer = try signer
        await assertAdmitted("shieldedShieldToRecipient") { manager in
            try await manager.shieldedShieldToRecipient(
                walletId: Self.walletId, recipientRaw43: Data(repeating: 3, count: 43), amount: 1,
                addressSigner: signer)
        }
    }

    func testShouldAdmitAnIdentityCreateFromPool() async throws {
        let signer = try signer
        let key = ManagedPlatformWallet.IdentityPubkey(
            keyId: 0, keyType: .ecdsaSecp256k1, purpose: .authentication,
            securityLevel: .master, pubkeyBytes: Data(repeating: 2, count: 33))
        await assertAdmitted("shieldedIdentityCreateFromPool") { manager in
            _ = try await manager.shieldedIdentityCreateFromPool(
                walletId: Self.walletId, resolver: MnemonicResolver(), identityIndex: 0,
                identityPubkeys: [key], denomination: 1,
                sendToAddressOnCreationFailure: Data(repeating: 0, count: 21),
                identitySigner: signer)
        }
    }

    private static let recipients = [
        ShieldedFundFromAssetLockRecipient(recipientRaw43: Data(repeating: 3, count: 43))
    ]

    func testShouldAdmitAFundFromAssetLock() async {
        await assertAdmitted("shieldedFundFromAssetLock") { manager in
            try await manager.shieldedFundFromAssetLock(
                walletId: Self.walletId, fundingAccountIndex: 0, amountDuffs: 1,
                recipients: Self.recipients)
        }
    }

    func testShouldAdmitAFundFromCoinJoinDrain() async {
        await assertAdmitted("shieldedFundFromCoinJoinDrain") { manager in
            try await manager.shieldedFundFromCoinJoinDrain(
                walletId: Self.walletId, recipients: Self.recipients)
        }
    }

    func testShouldAdmitAResumedFundFromAssetLock() async {
        await assertAdmitted("shieldedResumeFundFromAssetLock") { manager in
            try await manager.shieldedResumeFundFromAssetLock(
                walletId: Self.walletId, outPointTxid: Data(repeating: 4, count: 32),
                outPointVout: 0, recipients: Self.recipients)
        }
    }

    /// The pool-note seed is a long devnet/testnet QA loop that does not
    /// watch for shutdown, so it is deliberately left unadmitted: an admitted
    /// seed would hold `shutdown()` (and reject every synchronous call) for
    /// the whole run.
    func testShouldNotAdmitAPoolNoteSeedSoShutdownDoesNotWaitForIt() async {
        let manager = PlatformWalletManager.makeForTesting(
            handle: 71,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: { _ in Self.success }, platformAddressSyncStop: { _ in Self.success },
                shieldedSyncStop: { _ in Self.success }, dashPaySyncStop: { _ in Self.success },
                dpnsSyncStop: { _ in Self.success }, destroy: { _ in Self.success }))
        var admitted: [String] = []
        manager.nativeOpAdmissionObserver = { admitted.append($0) }
        _ = try? await manager.seedShieldedPoolNotes(walletId: Self.walletId)
        XCTAssertEqual(admitted, [], "seedShieldedPoolNotes must not be admitted")
        await manager.shutdown()
    }

    func testShouldAdmitAnIdentityDebitRecoveryRead() async {
        await assertAdmitted("shieldedIdentityDebitRecoveryRecords") { manager in
            _ = try await manager.shieldedIdentityDebitRecoveryRecords(walletId: Self.walletId)
        }
    }

    func testShouldAdmitAnIdentityDebitAbandonment() async {
        await assertAdmitted("abandonShieldedIdentityDebit") { manager in
            try await manager.abandonShieldedIdentityDebit(
                walletId: Self.walletId, accountIndex: 0, activityId: Data(repeating: 5, count: 32),
                acknowledgePossibleExecution: true)
        }
    }
}
