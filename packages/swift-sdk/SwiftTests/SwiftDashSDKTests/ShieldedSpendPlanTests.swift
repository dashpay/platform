import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// The Swift side of `platform_wallet_manager_shielded_plan_spend` and the
/// `_with_plan` spend exports: the plan decoded from its FFI struct, the
/// guards that run before any FFI call, and what reaches the FFI.
///
/// The manager here holds a handle no native manager owns, so a call that
/// reaches the FFI comes back `invalidHandle`; a guard or a marshalling the
/// FFI refuses comes back `invalidParameter` instead.
@MainActor
final class ShieldedSpendPlanTests: XCTestCase {
    private static let walletId = Data(repeating: 7, count: 32)

    private nonisolated static func noOpCalls() -> PlatformWalletNativeTeardownCalls {
        let success: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: success, platformAddressSyncStop: success, shieldedSyncStop: success,
            dashPaySyncStop: success, dpnsSyncStop: success, destroy: success)
    }

    private func makeManager() -> PlatformWalletManager {
        PlatformWalletManager.makeForTesting(handle: 71, calls: Self.noOpCalls())
    }

    private func plan(kind: ShieldedSpendKind, digestCount: Int = 32) -> ShieldedSpendPlan {
        ShieldedSpendPlan(
            kind: kind, amount: 1, recipientAmount: 1, fee: 2, totalInput: 3, totalDebited: 3, noteCount: 1,
            digest: Data(repeating: 9, count: digestCount))
    }

    private func assertThrows(
        _ name: String,
        _ expected: (PlatformWalletError) -> Bool,
        _ operation: () async throws -> Void,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        do {
            try await operation()
            XCTFail("\(name) did not throw", file: file, line: line)
        } catch let error as PlatformWalletError {
            XCTAssertTrue(expected(error), "\(name): unexpected \(error)", file: file, line: line)
        } catch {
            XCTFail("\(name): unexpected \(error)", file: file, line: line)
        }
    }

    private static func isInvalidHandle(_ error: PlatformWalletError) -> Bool {
        if case .invalidHandle = error { return true }
        return false
    }

    private static func isInvalidParameter(_ error: PlatformWalletError) -> Bool {
        if case .invalidParameter = error { return true }
        return false
    }

    // MARK: - The plan value

    func testShouldNumberSpendKindsAsTheFFIDoes() {
        XCTAssertEqual(ShieldedSpendKind.transfer.rawValue, 0)
        XCTAssertEqual(ShieldedSpendKind.unshield.rawValue, 1)
        XCTAssertEqual(ShieldedSpendKind.withdrawal.rawValue, 2)
        XCTAssertEqual(ShieldedSpendKind.identityTopUp.rawValue, 4)
    }

    func testShouldDecodeEveryFieldOfThePlan() throws {
        var ffi = ShieldedSpendPlanFFI()
        ffi.kind = 4
        ffi.amount = 100
        ffi.recipient_amount = 60
        ffi.fee = .max
        ffi.total_input = 300
        ffi.total_debited = 200
        ffi.note_count = .max
        withUnsafeMutableBytes(of: &ffi.digest) { raw in
            for index in raw.indices { raw[index] = UInt8(index) }
        }
        XCTAssertEqual(
            try ShieldedSpendPlan(ffi: ffi),
            ShieldedSpendPlan(
                kind: .identityTopUp, amount: 100, recipientAmount: 60, fee: .max, totalInput: 300,
                totalDebited: 200, noteCount: .max, digest: Data((0..<32).map(UInt8.init))))
    }

    /// A withdrawal plan without its Core fee rate is refused before the FFI.
    func testShouldRequireTheCoreFeeRateOfAWithdrawalPlan() async {
        let manager = makeManager()
        await assertThrows("plan", Self.isInvalidParameter) {
            _ = try await manager.shieldedPlanSpend(
                walletId: Self.walletId, kind: .withdrawal, amount: .exact(5))
        }
        await assertThrows("plan", Self.isInvalidHandle) {
            _ = try await manager.shieldedPlanSpend(
                walletId: Self.walletId, kind: .withdrawal, amount: .exact(5),
                coreFeePerByte: 1)
        }
        await manager.shutdown()
    }

    func testShouldRejectAPlanKindTheFFINeverWrites() {
        var ffi = ShieldedSpendPlanFFI()
        ffi.kind = 3
        XCTAssertThrowsError(try ShieldedSpendPlan(ffi: ffi))
    }

    // MARK: - Planning

    func testShouldRejectAnUnconfiguredManagerBeforePlanning() async {
        let manager = PlatformWalletManager()
        await assertThrows("plan", Self.isInvalidHandle) {
            _ = try await manager.shieldedPlanSpend(
                walletId: Self.walletId, kind: .transfer, amount: .exact(1))
        }
    }

    func testShouldRejectAShortWalletIdBeforePlanning() async {
        let manager = makeManager()
        await assertThrows("plan", Self.isInvalidParameter) {
            _ = try await manager.shieldedPlanSpend(
                walletId: Data(repeating: 7, count: 31), kind: .transfer, amount: .exact(1))
        }
        await manager.shutdown()
    }

    /// The FFI refuses an unknown kind byte, and a sweep with an amount, with
    /// `invalidParameter` before it looks the manager up: reaching the handle
    /// lookup shows every kind and both amounts were marshalled as it reads them.
    func testShouldMarshalEveryKindAndBothAmountsToTheFFI() async {
        let manager = makeManager()
        let kinds: [ShieldedSpendKind] = [.transfer, .unshield, .withdrawal, .identityTopUp]
        let amounts: [ShieldedSpendAmount] = [.exact(5), .sweepAll]
        for kind in kinds {
            for amount in amounts {
                await assertThrows("plan \(kind) \(amount)", Self.isInvalidHandle) {
                    _ = try await manager.shieldedPlanSpend(
                        walletId: Self.walletId, account: 3, kind: kind, amount: amount,
                        coreFeePerByte: 1)
                }
            }
        }
        await manager.shutdown()
    }

    // MARK: - Source compatibility

    /// The spends keep their signatures from before plans, so method references to them still
    /// resolve; spending against a plan is a separate overload taking an explicit plan.
    func testShouldKeepTheOriginalSpendSignaturesBesidePlanOverloads() async {
        let manager = makeManager()
        let transfer: (Data, MnemonicResolver, UInt32, Data, UInt64, String?) async throws -> Void =
            manager.shieldedTransfer(walletId:resolver:account:recipientRaw43:amount:memo:)
        let unshield: (Data, MnemonicResolver, UInt32, String, UInt64) async throws -> Void =
            manager.shieldedUnshield(walletId:resolver:account:toPlatformAddress:amount:)
        let topUp: (Data, MnemonicResolver, UInt32, Data, UInt64) async throws -> Void =
            manager.shieldedIdentityTopUpFromPool(walletId:resolver:account:identityId:amount:)
        let withdraw: (Data, MnemonicResolver, UInt32, String, UInt64, UInt32) async throws -> Void =
            manager.shieldedWithdraw(walletId:resolver:account:toCoreAddress:amount:coreFeePerByte:)
        let planned: [Any] = [
            manager.shieldedTransfer(walletId:resolver:account:recipientRaw43:amount:memo:plan:)
                as (Data, MnemonicResolver, UInt32, Data, UInt64, String?, ShieldedSpendPlan)
                async throws -> Void,
            manager.shieldedUnshield(walletId:resolver:account:toPlatformAddress:amount:plan:)
                as (Data, MnemonicResolver, UInt32, String, UInt64, ShieldedSpendPlan)
                async throws -> Void,
            manager.shieldedIdentityTopUpFromPool(walletId:resolver:account:identityId:amount:plan:)
                as (Data, MnemonicResolver, UInt32, Data, UInt64, ShieldedSpendPlan)
                async throws -> Void,
            manager.shieldedWithdraw(
                walletId:resolver:account:toCoreAddress:amount:coreFeePerByte:plan:)
                as (Data, MnemonicResolver, UInt32, String, UInt64, UInt32, ShieldedSpendPlan)
                async throws -> Void
        ]
        XCTAssertEqual(planned.count, 4)

        // The originals still reach the FFI with no plan.
        let resolver = MnemonicResolver()
        await assertThrows("transfer", Self.isInvalidHandle) {
            try await transfer(Self.walletId, resolver, 0, Data(repeating: 1, count: 43), 1, nil)
        }
        await assertThrows("unshield", Self.isInvalidHandle) {
            try await unshield(Self.walletId, resolver, 0, "tdash1x", 1)
        }
        await assertThrows("top up", Self.isInvalidHandle) {
            try await topUp(Self.walletId, resolver, 0, Data(repeating: 2, count: 32), 1)
        }
        await assertThrows("withdraw", Self.isInvalidHandle) {
            try await withdraw(Self.walletId, resolver, 0, "yX", 1, 1)
        }
        await manager.shutdown()
    }

    // MARK: - Spending against a plan

    private func spends(
        _ manager: PlatformWalletManager,
        resolver: MnemonicResolver,
        digestCount: Int
    ) -> [(String, () async throws -> Void)] {
        [
            ("transfer", { try await manager.shieldedTransfer(
                walletId: Self.walletId, resolver: resolver,
                recipientRaw43: Data(repeating: 1, count: 43), amount: 1,
                plan: self.plan(kind: .transfer, digestCount: digestCount)) }),
            ("unshield", { try await manager.shieldedUnshield(
                walletId: Self.walletId, resolver: resolver, toPlatformAddress: "tdash1x",
                amount: 1, plan: self.plan(kind: .unshield, digestCount: digestCount)) }),
            ("withdraw", { try await manager.shieldedWithdraw(
                walletId: Self.walletId, resolver: resolver, toCoreAddress: "yX", amount: 1,
                plan: self.plan(kind: .withdrawal, digestCount: digestCount)) }),
            ("top up", { try await manager.shieldedIdentityTopUpFromPool(
                walletId: Self.walletId, resolver: resolver,
                identityId: Data(repeating: 2, count: 32), amount: 1,
                plan: self.plan(kind: .identityTopUp, digestCount: digestCount)) })
        ]
    }

    func testShouldRejectAPlanDigestOfTheWrongLengthBeforeAnySpend() async {
        let manager = makeManager()
        let resolver = MnemonicResolver()
        for (name, spend) in spends(manager, resolver: resolver, digestCount: 31) {
            await assertThrows(name, Self.isInvalidParameter, spend)
        }
        await manager.shutdown()
    }

    func testShouldPassAPlanToEverySpendsFFI() async {
        let manager = makeManager()
        let resolver = MnemonicResolver()
        for (name, spend) in spends(manager, resolver: resolver, digestCount: 32) {
            await assertThrows(name, Self.isInvalidHandle, spend)
        }
        await manager.shutdown()
    }
}
