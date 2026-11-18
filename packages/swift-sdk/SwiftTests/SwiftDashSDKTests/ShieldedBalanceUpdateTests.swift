import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// The pushed shielded balance: decoding the native update, the
/// latest-revision rule, the two balance accessors, and what the manager
/// publishes. The native values are built in Swift; no Rust code runs.
@MainActor
final class ShieldedBalanceUpdateTests: XCTestCase {
    private static let walletId = Data(repeating: 7, count: 32)

    private func row(
        _ account: UInt32,
        credits: UInt64 = 0,
        source: ShieldedBalanceSourceFFI = SHIELDED_BALANCE_SOURCE_FFI_RESTORED
    ) -> ShieldedLocalAccountBalanceFFI {
        ShieldedLocalAccountBalanceFFI(
            account_index: account, spendable_credits: credits,
            last_scanned_index: 5, has_last_scanned_index: true, source: source)
    }

    /// Builds a native update over a Swift-owned account array and passes
    /// it to `body` while the array is alive, as Rust does for a callback.
    private func withNativeUpdate<T>(
        revision: UInt64 = 1,
        status: ShieldedLocalBalanceStatusFFI = SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY,
        ordinary: UInt64 = 0,
        established: Bool = true,
        rows: [ShieldedLocalAccountBalanceFFI] = [],
        countOverride: UInt? = nil,
        _ body: (UnsafePointer<ShieldedBalanceUpdateFFI>) throws -> T
    ) rethrows -> T {
        try rows.withUnsafeBufferPointer { buffer in
            var update = ShieldedBalanceUpdateFFI()
            withUnsafeMutableBytes(of: &update.wallet_id) { bytes in
                Self.walletId.copyBytes(to: bytes)
            }
            update.revision = revision
            update.status = status
            update.ordinary_spendable_credits = ordinary
            update.ordinary_balance_established = established
            update.accounts = rows.isEmpty ? nil : buffer.baseAddress
            update.accounts_count = countOverride ?? UInt(rows.count)
            return try withUnsafePointer(to: &update) { try body($0) }
        }
    }

    private func update(revision: UInt64, _ state: ShieldedBalanceState) -> ShieldedBalanceUpdate {
        ShieldedBalanceUpdate(walletId: Self.walletId, revision: revision, state: state)
    }

    private func ready(
        ordinary: UInt64,
        established: Bool,
        accounts: [UInt32: ShieldedLocalAccountBalance] = [:]
    ) -> ShieldedBalanceState {
        .ready(ShieldedReadyBalance(
            ordinarySpendableCredits: ordinary,
            ordinaryBalanceEstablished: established,
            accounts: accounts))
    }

    private func makeManager() -> PlatformWalletManager {
        let step: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletManager.makeForTesting(
            handle: Handle.max,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: step, platformAddressSyncStop: step, shieldedSyncStop: step,
                dashPaySyncStop: step, dpnsSyncStop: step, destroy: step))
    }

    // MARK: - Decoding

    func testShouldDecodeAReadyUpdateKeepingIdentityAccounts() throws {
        let decoded = try withNativeUpdate(
            revision: 4, ordinary: 150, established: true,
            rows: [
                row(0, credits: 150),
                row(0x4000_0000, credits: 900, source: SHIELDED_BALANCE_SOURCE_FFI_SCANNED_THIS_SESSION),
            ]
        ) { try ShieldedBalanceUpdate.decode($0.pointee) }
        XCTAssertEqual(decoded, update(revision: 4, ready(ordinary: 150, established: true, accounts: [
            0: ShieldedLocalAccountBalance(spendableCredits: 150, lastScannedIndex: 5, source: .restored),
            0x4000_0000: ShieldedLocalAccountBalance(
                spendableCredits: 900, lastScannedIndex: 5, source: .scannedThisSession),
        ])))
        XCTAssertEqual(decoded.ordinaryBalance, 150)
    }

    func testShouldDecodeUnboundAndIncompleteUpdates() throws {
        let unbound = try withNativeUpdate(status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND) {
            try ShieldedBalanceUpdate.decode($0.pointee)
        }
        XCTAssertEqual(unbound.state, .unbound)
        let incomplete = try withNativeUpdate(status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE) {
            try ShieldedBalanceUpdate.decode($0.pointee)
        }
        XCTAssertEqual(incomplete.state, .restoreIncomplete)
    }

    func testShouldRejectMalformedUpdates() {
        let cases: [(String, () throws -> ShieldedBalanceUpdate)] = [
            ("unknown status", {
                try self.withNativeUpdate(status: ShieldedLocalBalanceStatusFFI(rawValue: 99)) {
                    try ShieldedBalanceUpdate.decode($0.pointee)
                }
            }),
            ("unbound with accounts", {
                try self.withNativeUpdate(status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND, rows: [self.row(0)]) {
                    try ShieldedBalanceUpdate.decode($0.pointee)
                }
            }),
            ("incomplete with accounts", {
                try self.withNativeUpdate(
                    status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_RESTORE_INCOMPLETE, rows: [self.row(0)]
                ) { try ShieldedBalanceUpdate.decode($0.pointee) }
            }),
            ("duplicate account", {
                try self.withNativeUpdate(rows: [self.row(1), self.row(1)]) {
                    try ShieldedBalanceUpdate.decode($0.pointee)
                }
            }),
            ("unknown source", {
                try self.withNativeUpdate(rows: [self.row(0, source: ShieldedBalanceSourceFFI(rawValue: 99))]) {
                    try ShieldedBalanceUpdate.decode($0.pointee)
                }
            }),
            ("count without storage", {
                try self.withNativeUpdate(countOverride: 1) { try ShieldedBalanceUpdate.decode($0.pointee) }
            }),
        ]
        for (name, decode) in cases {
            XCTAssertThrowsError(try decode(), name) { error in
                guard case PlatformWalletError.deserialization = error else {
                    return XCTFail("\(name): unexpected error \(error)")
                }
            }
        }
    }

    // MARK: - Accessors

    func testShouldReportAnOrdinaryBalanceOnlyWhenEstablished() {
        XCTAssertEqual(update(revision: 1, ready(ordinary: 150, established: true)).ordinaryBalance, 150)
        XCTAssertEqual(update(revision: 1, ready(ordinary: 0, established: true)).ordinaryBalance, 0)
        XCTAssertNil(update(revision: 1, ready(ordinary: 0, established: false)).ordinaryBalance)
        XCTAssertNil(update(revision: 1, .unbound).ordinaryBalance)
        XCTAssertNil(update(revision: 1, .restoreIncomplete).ordinaryBalance)
    }

    func testShouldReportAnAccountBalanceOnlyForABoundAccountWithHistory() {
        let state = ready(ordinary: 100, established: false, accounts: [
            0: ShieldedLocalAccountBalance(spendableCredits: 100, lastScannedIndex: 3, source: .scannedThisSession),
            1: ShieldedLocalAccountBalance(spendableCredits: 0, lastScannedIndex: nil, source: .noHistory),
            2: ShieldedLocalAccountBalance(spendableCredits: 0, lastScannedIndex: 0, source: .restored),
        ])
        let held = update(revision: 1, state)
        XCTAssertEqual(held.accountBalance(0), 100)
        XCTAssertNil(held.accountBalance(1), "a no-history zero is not a known zero")
        XCTAssertEqual(held.accountBalance(2), 0)
        XCTAssertNil(held.accountBalance(7), "an account that is not bound")
        XCTAssertNil(update(revision: 1, .unbound).accountBalance(0))
        XCTAssertNil(update(revision: 1, .restoreIncomplete).accountBalance(0))
    }

    // MARK: - Latest revision

    func testShouldReplaceTheHeldUpdateOnlyWithAHigherRevision() {
        let held = update(revision: 5, .unbound)
        XCTAssertTrue(ShieldedBalanceUpdate.shouldReplace(held: nil, with: update(revision: 1, .unbound)))
        XCTAssertTrue(ShieldedBalanceUpdate.shouldReplace(held: held, with: update(revision: 6, .restoreIncomplete)))
        XCTAssertFalse(ShieldedBalanceUpdate.shouldReplace(held: held, with: update(revision: 5, .restoreIncomplete)))
        XCTAssertFalse(ShieldedBalanceUpdate.shouldReplace(held: held, with: update(revision: 4, .restoreIncomplete)))
    }

    // MARK: - Publication

    func testShouldPublishTheLatestRevisionPerWallet() async {
        let manager = makeManager()
        manager.handleShieldedBalanceChanged(update(revision: 2, ready(ordinary: 10, established: true)))
        manager.handleShieldedBalanceChanged(update(revision: 1, .unbound))
        XCTAssertEqual(manager.shieldedBalances[Self.walletId]?.revision, 2)
        manager.handleShieldedBalanceChanged(update(revision: 3, .unbound))
        XCTAssertEqual(manager.shieldedBalances[Self.walletId], update(revision: 3, .unbound))
        await manager.shutdown()
    }

    func testShouldEmptyTheBalancesAtShutdownAndDropLaterUpdates() async {
        let manager = makeManager()
        manager.handleShieldedBalanceChanged(update(revision: 1, ready(ordinary: 10, established: true)))
        await manager.shutdown()
        XCTAssertTrue(manager.shieldedBalances.isEmpty)
        manager.handleShieldedBalanceChanged(update(revision: 2, ready(ordinary: 20, established: true)))
        XCTAssertTrue(manager.shieldedBalances.isEmpty)
    }

    func testShouldKeepBalancesWhenShieldedSyncGenerationChanges() async {
        // Stop and Clear bump the sync generation; the balance state stays
        // Rust's and keeps arriving.
        let manager = makeManager()
        manager.handleShieldedBalanceChanged(update(revision: 1, ready(ordinary: 10, established: true)))
        manager.shieldedSyncGeneration.bump()
        manager.handleShieldedBalanceChanged(update(revision: 2, .unbound))
        XCTAssertEqual(manager.shieldedBalances[Self.walletId], update(revision: 2, .unbound))
        await manager.shutdown()
    }

    /// The trampoline copies the borrowed account array before its
    /// main-actor hop: the array is gone when the update is published.
    func testShouldCopyTheUpdateInsideTheCallback() async throws {
        let manager = makeManager()
        let handler = PlatformWalletEventHandler(manager: manager)
        let context = Unmanaged.passUnretained(handler).toOpaque()
        var rows: [ShieldedLocalAccountBalanceFFI]? = [row(0, credits: 42)]
        rows!.withUnsafeBufferPointer { buffer in
            var native = ShieldedBalanceUpdateFFI()
            withUnsafeMutableBytes(of: &native.wallet_id) { Self.walletId.copyBytes(to: $0) }
            native.revision = 1
            native.status = SHIELDED_LOCAL_BALANCE_STATUS_FFI_READY
            native.ordinary_spendable_credits = 42
            native.ordinary_balance_established = true
            native.accounts = buffer.baseAddress
            native.accounts_count = 1
            withUnsafePointer(to: &native) { shieldedBalanceChangedCallback(context: context, update: $0) }
        }
        rows = nil
        for _ in 0..<200 where manager.shieldedBalances[Self.walletId] == nil {
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTAssertEqual(manager.shieldedBalances[Self.walletId]?.ordinaryBalance, 42)
        XCTAssertEqual(manager.shieldedBalances[Self.walletId]?.accountBalance(0), 42)
        withExtendedLifetime(handler) {}
        await manager.shutdown()
    }
}
