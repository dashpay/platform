import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Verifies that the persisted wallet state survives an SPV stop/start
/// cycle
final class SpvRestartIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 { UInt64(fundingDash * 1e8) }
    private let amount: UInt64 = 100_000

    func testTxosSurviveSpvRestart() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "spv-restart-alice")
        let bob = try await env.makeTestWallet(name: "spv-restart-bob")

        let aliceAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: aliceAddress, dash: fundingDash)

        let bobAddress = try bob.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: bobAddress, dash: fundingDash)

        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)
        try await bob.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        try await sendAndConfirm(from: alice, to: bob)

        // Restart the SPV client with explicit stop + start directly on
        // the wallet manager — the same API a real app drives — then
        // wait for it to catch back up before the next send.
        try await env.walletManager.stopSpv()
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.waitUntilUpToDate(
            height: try await env.coreRPC.getBlockCount(),
            timeout: 30
        )

        try await sendAndConfirm(from: alice, to: bob)

        let aliceWalletId = alice.getPlatformWallet().walletId
        let bobWalletId = bob.getPlatformWallet().walletId
        let container = env.modelContainer

        try await MainActor.run {
            let context = ModelContext(container)

            let aliceTxoCount = try context.fetchCount(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate<PersistentTxo> { $0.walletId == aliceWalletId }
            ))
            let bobTxoCount = try context.fetchCount(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate<PersistentTxo> { $0.walletId == bobWalletId }
            ))

            XCTAssertEqual(aliceTxoCount, 3)
            XCTAssertEqual(bobTxoCount, 3)
        }
    }

    private func sendAndConfirm(
        from sender: TestWalletWrapper,
        to receiver: TestWalletWrapper
    ) async throws {
        let receiverBalanceBefore = try receiver.getPlatformWallet().balance().spendable
        let recipientAddress = try receiver.getCoreWallet().nextReceiveAddress()

        let beforeTxids = try await readTxids()
        // #3970 replaced the one-shot ManagedCoreWallet.sendToAddresses with
        // the split CoreTransactionBuilder; drive it exactly as SendViewModel's
        // `.coreToCore` flow does (build → fund → sign → broadcast). Coin
        // selection, funding, and signing are all Rust-side.
        let senderWallet = sender.getPlatformWallet()
        let builder = try CoreTransactionBuilder(network: .regtest)
        try builder.addOutput(address: recipientAddress, amountDuffs: amount)
        try builder.setFunding(wallet: senderWallet, accountType: .bip44, accountIndex: 0)
        let signedTx = try builder.buildSigned(wallet: senderWallet, accountType: .bip44, accountIndex: 0)
        _ = try senderWallet.coreWallet().broadcastTransaction(signedTx)
        guard let sendTxid = try await waitForNewTxid(notIn: beforeTxids) else {
            XCTFail("send PersistentTransaction row never appeared")
            return
        }
        _ = try await env.mine(1, including: sendTxid)

        try await Wait.until(
            "receiver balance advances by \(amount)",
            timeout: 60,
            pollInterval: 0.01
        ) {
            try receiver.getPlatformWallet().balance().spendable
                == receiverBalanceBefore + amount
        }
    }
}
