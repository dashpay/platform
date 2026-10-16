import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Regression coverage for broadcast-probe callbacks that are queued for the
/// main actor while `deleteWallet` runs on it.
@MainActor
final class BroadcastProbeWalletGenerationTests: XCTestCase {

    private nonisolated static func noOpCalls() -> PlatformWalletNativeTeardownCalls {
        let success: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(
                code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS,
                message: nil
            )
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: success,
            platformAddressSyncStop: success,
            shieldedSyncStop: success,
            dashPaySyncStop: success,
            dpnsSyncStop: success,
            destroy: success
        )
    }

    private func makeManager() -> PlatformWalletManager {
        PlatformWalletManager.makeForTesting(handle: 72, calls: Self.noOpCalls())
    }

    private func id(_ byte: UInt8) -> Data { Data(repeating: byte, count: 32) }

    /// Deliver a callback the way Rust does, through the C trampoline. It
    /// reads the wallet's generation now and queues the rest for the main
    /// queue, which cannot run before this actor turn yields.
    private func deliver(
        _ handler: PlatformWalletEventHandler,
        walletId: Data,
        txid: Data,
        verdictCode: Int
    ) {
        walletId.withUnsafeBytes { wallet in
            txid.withUnsafeBytes { tx in
                outgoingTransactionProbedCallback(
                    context: Unmanaged.passUnretained(handler).toOpaque(),
                    walletIdPtr: wallet.bindMemory(to: UInt8.self).baseAddress,
                    txidPtr: tx.bindMemory(to: UInt8.self).baseAddress,
                    verdictCode: UInt8(verdictCode),
                    reasonPtr: nil
                )
            }
        }
    }

    /// Returns once everything queued on the main queue so far has run.
    private func drainMainQueue() async {
        await withCheckedContinuation { continuation in
            DispatchQueue.main.async { continuation.resume() }
        }
    }

    func testShouldPublishAQueuedVerdictAndApplyItsLaterClear() async {
        let manager = makeManager()
        let handler = PlatformWalletEventHandler(manager: manager)
        let key = OutgoingTransactionKey(walletId: id(1), txidWire: id(9))

        deliver(handler, walletId: id(1), txid: id(9), verdictCode: Int(OUTGOING_PROBE_VERDICT_ACCEPTED))
        await drainMainQueue()
        XCTAssertEqual(manager.outgoingTransactionVerdicts[key]?.verdict, .accepted)
        XCTAssertEqual(manager.lastOutgoingTransactionProbe?.key, key)

        deliver(handler, walletId: id(1), txid: id(9), verdictCode: Int(OUTGOING_PROBE_VERDICT_CLEARED))
        await drainMainQueue()
        XCTAssertNil(manager.outgoingTransactionVerdicts[key])
        XCTAssertNil(manager.lastOutgoingTransactionProbe)
        await manager.shutdown()
    }

    func testShouldDropAQueuedVerdictWhenItsWalletIsDeletedBeforeDelivery() async {
        let manager = makeManager()
        let handler = PlatformWalletEventHandler(manager: manager)

        deliver(handler, walletId: id(1), txid: id(9), verdictCode: Int(OUTGOING_PROBE_VERDICT_ACCEPTED))
        manager.forgetOutgoingTransactionProbes(walletId: id(1))
        await drainMainQueue()

        XCTAssertTrue(manager.outgoingTransactionVerdicts.isEmpty)
        XCTAssertNil(manager.lastOutgoingTransactionProbe)
        await manager.shutdown()
    }

    /// A wallet restored from the same mnemonic has the same id, so wallet
    /// membership cannot tell its verdicts from the deleted wallet's.
    func testShouldNotPublishADeletedWalletsQueuedVerdictForItsRestoredSuccessor() async {
        let manager = makeManager()
        let handler = PlatformWalletEventHandler(manager: manager)

        deliver(handler, walletId: id(1), txid: id(9), verdictCode: Int(OUTGOING_PROBE_VERDICT_ACCEPTED))
        manager.forgetOutgoingTransactionProbes(walletId: id(1))
        // The successor's own verdict, for another send.
        deliver(handler, walletId: id(1), txid: id(8), verdictCode: Int(OUTGOING_PROBE_VERDICT_UNRESOLVED))
        await drainMainQueue()

        let successorKey = OutgoingTransactionKey(walletId: id(1), txidWire: id(8))
        XCTAssertEqual(Array(manager.outgoingTransactionVerdicts.keys), [successorKey])
        XCTAssertEqual(manager.lastOutgoingTransactionProbe?.key, successorKey)
        await manager.shutdown()
    }

    func testShouldNotLetADeletedWalletsClearRemoveItsSuccessorsVerdict() async {
        let manager = makeManager()
        let key = OutgoingTransactionKey(walletId: id(1), txidWire: id(9))
        let deletedWalletsGeneration = manager.outgoingProbeWalletGeneration.current(id(1))
        manager.forgetOutgoingTransactionProbes(walletId: id(1))
        let event = OutgoingTransactionProbeEvent(walletId: id(1), txidWire: id(9), verdict: .accepted)
        manager.handleOutgoingTransactionProbed(
            event, generation: manager.outgoingProbeWalletGeneration.current(id(1)))

        manager.handleOutgoingTransactionCleared(key, generation: deletedWalletsGeneration)

        XCTAssertEqual(manager.outgoingTransactionVerdicts[key], event)
        XCTAssertEqual(manager.lastOutgoingTransactionProbe, event)
        await manager.shutdown()
    }

    func testShouldKeepAnotherWalletsQueuedVerdictWhenAWalletIsDeleted() async {
        let manager = makeManager()
        let handler = PlatformWalletEventHandler(manager: manager)
        let otherKey = OutgoingTransactionKey(walletId: id(2), txidWire: id(9))

        deliver(handler, walletId: id(2), txid: id(9), verdictCode: Int(OUTGOING_PROBE_VERDICT_ACCEPTED))
        manager.forgetOutgoingTransactionProbes(walletId: id(1))
        await drainMainQueue()

        XCTAssertEqual(manager.outgoingTransactionVerdicts[otherKey]?.verdict, .accepted)
        await manager.shutdown()
    }
}
