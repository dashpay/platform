import XCTest
import SwiftData
import CryptoKit
@testable import SwiftDashSDK

/// This test ensures that the classification of a self-send transaction
/// as Internal / -fee survives a restart of the persister
final class PersisterRestartClassificationIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 { UInt64(fundingDash * 1e8) }
    private let sendAmount: UInt64 = 10_000

    func testSelfSendClassificationSurvivesRestart() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "restart-class-alice")

        let aliceFundingAddr = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: aliceFundingAddr, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let aliceSecondAddr = try alice.getCoreWallet().nextReceiveAddress()
        let sendTxData = try alice.getCoreWallet().sendToAddresses(
            recipients: [(address: aliceSecondAddr, amountDuffs: sendAmount)]
        )
        let sendTxid = Self.txid(ofRawTx: sendTxData)
        try await waitForTxRow(sendTxid)

        // First sighting must already classify as Internal / -fee.
        try await assertSelfSendRow(txid: sendTxid, phase: "first sighting")

        try await env.restartWalletManager()
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.waitUntilUpToDate(height: try await env.coreRPC.getBlockCount())

        // Regression: classification must survive the restart.
        try await assertSelfSendRow(txid: sendTxid, phase: "post-restart catch-up")
    }

    // MARK: - Helpers

    /// `sha256d(tx_bytes)` in internal byte order — the identity
    /// `PersistentTransaction.txid` stores (the persister copies Rust's
    /// `Txid` verbatim).
    private static func txid(ofRawTx data: Data) -> Data {
        let first = Data(SHA256.hash(data: data))
        return Data(SHA256.hash(data: first))
    }

    private func waitForTxRow(_ txid: Data, timeout: TimeInterval = 60) async throws {
        try await Wait.until(
            "PersistentTransaction row for \(txid.toHexString())",
            timeout: timeout
        ) {
            try await self.fetchTransaction(txid) != nil
        }
    }

    private struct TxSnapshot: Sendable {
        let direction: UInt32
        let netAmount: Int64
        let context: UInt32
    }

    private func fetchTransaction(_ txid: Data) async throws -> TxSnapshot? {
        let container = env.modelContainer
        return try await MainActor.run {
            let ctx = ModelContext(container)
            guard let row = try ctx.fetch(FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txid }
            )).first else { return nil }
            return TxSnapshot(
                direction: row.direction,
                netAmount: row.netAmount,
                context: row.context
            )
        }
    }

    private func assertSelfSendRow(txid: Data, phase: String) async throws {
        guard let row = try await fetchTransaction(txid) else {
            XCTFail("\(phase): row missing for txid \(txid.toHexString())")
            return
        }

        XCTAssertEqual(
            row.direction, 2,
            "\(phase): direction=\(row.direction) (expected 2=Internal). " +
            "context=\(row.context), netAmount=\(row.netAmount). " +
            "Positive netAmount with direction=0 is the phantom-incoming signature."
        )

        XCTAssertLessThan(
            row.netAmount, 0,
            "\(phase): netAmount=\(row.netAmount) (expected <0; self-send only leaves the fee)."
        )
    }
}
