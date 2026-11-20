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
            kind: kind, coreFeePerByte: kind == .withdrawal ? 5 : 0, amount: 1, recipientAmount: 1,
            fee: 2, totalInput: 3, totalDebited: 3, noteCount: 1,
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
        ffi.kind = 2
        ffi.core_fee_per_byte = .max
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
                kind: .withdrawal, coreFeePerByte: .max, amount: 100, recipientAmount: 60, fee: .max,
                totalInput: 300, totalDebited: 200, noteCount: .max,
                digest: Data((0..<32).map(UInt8.init))))
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
            ("withdraw", {
                let plan = self.plan(kind: .withdrawal, digestCount: digestCount)
                try await manager.shieldedWithdraw(
                    walletId: Self.walletId, resolver: resolver, toCoreAddress: "yX", amount: 1,
                    coreFeePerByte: plan.coreFeePerByte, plan: plan)
            }),
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

    /// Records which native spend export ran, the digest it was handed and,
    /// for a withdrawal, the Core fee rate.
    private final class SpendLog: @unchecked Sendable {
        private let lock = NSLock()
        private var items: [(String, Data?)] = []
        private var rates: [UInt32] = []
        func record(_ name: String, _ digest: UnsafePointer<UInt8>?, rate: UInt32? = nil) {
            lock.withLock {
                items.append((name, digest.map { Data(bytes: $0, count: 32) }))
                if let rate { rates.append(rate) }
            }
        }
        var entries: [(String, Data?)] { lock.withLock { items } }
        var coreFeeRates: [UInt32] { lock.withLock { rates } }
    }

    private nonisolated static func recordingCalls(_ log: SpendLog) -> PlatformWalletNativeShieldedSpendCalls {
        @Sendable func ok() -> PlatformWalletFFIResult {
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeShieldedSpendCalls(
            planSpend: { _, _, _, _, _, _, _, _ in log.record("planSpend", nil); return ok() },
            transfer: { _, _, _, _, _, _, _ in log.record("transfer", nil); return ok() },
            transferWithPlan: { _, _, _, _, _, _, _, d in log.record("transferWithPlan", d); return ok() },
            unshield: { _, _, _, _, _, _ in log.record("unshield", nil); return ok() },
            unshieldWithPlan: { _, _, _, _, _, _, d in log.record("unshieldWithPlan", d); return ok() },
            topUp: { _, _, _, _, _, _ in log.record("topUp", nil); return ok() },
            topUpWithPlan: { _, _, _, _, _, _, d in log.record("topUpWithPlan", d); return ok() },
            withdraw: { _, _, _, _, _, _, rate in log.record("withdraw", nil, rate: rate); return ok() },
            withdrawWithPlan: { _, _, _, _, _, _, rate, d in
                log.record("withdrawWithPlan", d, rate: rate); return ok()
            })
    }

    /// Each `plan:` overload calls its `_with_plan` export exactly once with
    /// the plan's exact digest bytes (and a withdrawal the rate the caller
    /// passed, the plan's own) and never the plain export; each plain
    /// overload calls only the plain export.
    func testShouldRouteEverySpendToTheExportMatchingItsPlan() async throws {
        let digest = Data((0..<32).map { UInt8($0 * 7 + 1) })
        let resolver = MnemonicResolver()
        func plan(_ kind: ShieldedSpendKind) -> ShieldedSpendPlan {
            ShieldedSpendPlan(
                kind: kind, coreFeePerByte: kind == .withdrawal ? 5 : 0, amount: 1,
                recipientAmount: 1, fee: 2, totalInput: 3, totalDebited: 3, noteCount: 1,
                digest: digest)
        }
        let cases: [(String, (PlatformWalletManager) async throws -> Void)] = [
            ("transferWithPlan", { try await $0.shieldedTransfer(
                walletId: Self.walletId, resolver: resolver,
                recipientRaw43: Data(repeating: 1, count: 43), amount: 1, plan: plan(.transfer)) }),
            ("transfer", { try await $0.shieldedTransfer(
                walletId: Self.walletId, resolver: resolver,
                recipientRaw43: Data(repeating: 1, count: 43), amount: 1) }),
            ("unshieldWithPlan", { try await $0.shieldedUnshield(
                walletId: Self.walletId, resolver: resolver, toPlatformAddress: "tdash1x",
                amount: 1, plan: plan(.unshield)) }),
            ("unshield", { try await $0.shieldedUnshield(
                walletId: Self.walletId, resolver: resolver, toPlatformAddress: "tdash1x",
                amount: 1) }),
            ("withdrawWithPlan", { try await $0.shieldedWithdraw(
                walletId: Self.walletId, resolver: resolver, toCoreAddress: "yX", amount: 1,
                coreFeePerByte: plan(.withdrawal).coreFeePerByte, plan: plan(.withdrawal)) }),
            ("withdraw", { try await $0.shieldedWithdraw(
                walletId: Self.walletId, resolver: resolver, toCoreAddress: "yX", amount: 1) }),
            ("topUpWithPlan", { try await $0.shieldedIdentityTopUpFromPool(
                walletId: Self.walletId, resolver: resolver,
                identityId: Data(repeating: 2, count: 32), amount: 1, plan: plan(.identityTopUp)) }),
            ("topUp", { try await $0.shieldedIdentityTopUpFromPool(
                walletId: Self.walletId, resolver: resolver,
                identityId: Data(repeating: 2, count: 32), amount: 1) })
        ]
        for (expected, spend) in cases {
            let manager = makeManager()
            let log = SpendLog()
            manager.nativeShieldedSpendCalls = Self.recordingCalls(log)
            try await spend(manager)
            let entries = log.entries
            XCTAssertEqual(entries.map(\.0), [expected], "\(expected): wrong export(s) called")
            if expected.hasSuffix("WithPlan") {
                XCTAssertEqual(entries.first?.1, digest, "\(expected): digest bytes differ")
            }
            if expected == "withdrawWithPlan" {
                XCTAssertEqual(log.coreFeeRates, [5], "the plan's Core fee rate reaches the FFI")
            }
            await manager.shutdown()
        }
    }
}
