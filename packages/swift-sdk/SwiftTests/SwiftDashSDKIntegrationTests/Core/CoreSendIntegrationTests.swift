import XCTest
import SwiftData
@testable import SwiftDashSDK

final class CoreSendIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 {
        UInt64(fundingDash * 1e8)
    }

    func testWalletToWalletViaSpv() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        let alice = try await env.makeTestWallet(name: "core-send-alice")
        let bob = try await env.makeTestWallet(name: "core-send-bob")

        let aliceAddress = try alice.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: aliceAddress, dash: fundingDash)
        let bobAddress = try bob.getCoreWallet().nextReceiveAddress()
        _ = try await env.fund(address: bobAddress, dash: fundingDash)
        try await alice.waitForSpendable(exactly: fundingDuffs, timeout: 90)
        try await bob.waitForSpendable(exactly: fundingDuffs, timeout: 90)

        let iterations = 5
        let amount: UInt64 = 100_000 // 0.001 DASH per hop

        for i in 0 ..< iterations {
            let aliceSends = (i % 2 == 0)
            let sender = aliceSends ? alice: bob
            let receiver = aliceSends ? bob: alice

            let receiverBalanceBefore = try receiver.getPlatformWallet().balance().spendable
            let recipientAddress = try receiver.getCoreWallet().nextReceiveAddress()

            let beforeTxids = try await readTxids()
            _ = try sender.send(to: recipientAddress, amountDuffs: amount)
            guard let sendTxid = try await waitForNewTxid(notIn: beforeTxids) else {
                XCTFail("send PersistentTransaction row never appeared on iteration \(i)")
                return
            }
            _ = try await env.mine(1, including: sendTxid)

            try await Wait.until(
                "receiver +\(amount) after iteration \(i)",
                timeout: 60,
                pollInterval: 0.01
            ) {
                try receiver.getPlatformWallet().balance().spendable
                    == receiverBalanceBefore + amount
            }
        }

        let aliceFinal = try alice.getPlatformWallet().balance().spendable
        let bobFinal = try bob.getPlatformWallet().balance().spendable
        XCTAssertLessThanOrEqual(aliceFinal + bobFinal, 2 * fundingDuffs)

        // Validate via the SwiftData
        let expectedTotalTxs = 2 + iterations
        let aliceWalletId = alice.getPlatformWallet().walletId
        let bobWalletId = bob.getPlatformWallet().walletId
        let container = env.modelContainer

        try await MainActor.run {
            let context = ModelContext(container)
            let allTxCount = try context.fetchCount(FetchDescriptor<PersistentTransaction>())
            XCTAssertEqual(allTxCount, expectedTotalTxs)

            let aliceTxoCount = try context.fetchCount(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate<PersistentTxo>{
                    $0.walletId == aliceWalletId
                }
            ))
            let bobTxoCount = try context.fetchCount(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate<PersistentTxo>{
                    $0.walletId == bobWalletId
                }
            ))

            XCTAssertEqual(aliceTxoCount, 1 + iterations)
            XCTAssertEqual(bobTxoCount, 1 + iterations)
        }
    }
}
