import XCTest
@testable import SwiftDashSDK

/// A wallet imported into an ALREADY-RUNNING, already-synced SPV client must
/// backfill its historical transactions.
///
/// The realistic flow this exercises: the client is started empty and synced to
/// the chain tip, then the user imports an existing, already-funded mnemonic
/// with `birthHeight: 0` — per the `createWallet` contract that means "scan from
/// genesis and see funds received before this device knew the wallet".
///
/// Regression: the SPV rescan that fires when the wallet is added backfills its
/// historical UTXOs, and `register_wallet` re-seeds the lock-free balance atomic
/// from the wallet's inner balance so those backfilled funds are visible even
/// though the `BlockProcessed` events can land before the wallet enters the
/// manager's `wallets` map. Before the fix the imported wallet's balance stayed 0.
final class SpvLateWalletBackfillIntegrationTests: IntegrationTestCase {
    private let fundingDash: Double = 0.5
    private var fundingDuffs: UInt64 { UInt64(fundingDash * 1e8) }

    func testWalletAddedToRunningClientBackfillsHistory() async throws {
        let sdk = env.sdk

        // 1. Derive the wallet's receive address WITHOUT registering it in the
        //    SPV-driving manager, so the client can start genuinely empty. A
        //    throwaway manager (which never starts SPV, on its own in-memory
        //    store) just hands us the mnemonic + funding address.
        let mnemonic = try Mnemonic.generate(wordCount: 24)
        let fundingAddress = try await MainActor.run { () -> String in
            let throwaway = try PlatformWalletManager(
                sdk: sdk,
                modelContainer: try DashModelContainer.createInMemory()
            )
            let w = try throwaway.createWallet(
                mnemonic: mnemonic,
                network: .regtest,
                createDefaultAccounts: true
            )
            return try w.coreWallet().nextReceiveAddress()
        }

        // 2. Fund the address on-chain and confirm it (`fund` mines 1 block).
        _ = try await env.fund(address: fundingAddress, dash: fundingDash)

        // 3. Start the SPV client EMPTY (env.walletManager holds no wallets yet)
        //    and sync past the funding block. The client matches nothing — there
        //    is no wallet to match against.
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.waitUntilUpToDate(
            height: try await env.coreRPC.getBlockCount(),
            timeout: 60
        )

        // 4. Import the funded wallet into the RUNNING client, birthHeight 0 so
        //    it is contractually expected to scan history from genesis.
        let imported = try await env.walletManager.createWallet(
            mnemonic: mnemonic,
            network: .regtest,
            name: "late-import",
            createDefaultAccounts: true,
            birthHeight: 0
        )

        // 5. Give the running client generous time to backfill the new wallet's
        //    history. Don't fail on timeout here — let step 6 report the clean
        //    balance assertion.
        let expected = fundingDuffs
        _ = try? await Wait.until(
            "imported wallet backfills its historical funding (\(expected) duffs)",
            timeout: 60,
            pollInterval: 0.25
        ) {
            try imported.balance().spendable == expected
        }

        // 6. Verify all funds were found.
        let bal = try imported.balance()
        XCTAssertEqual(
            bal.spendable,
            expected,
            "backfill balance breakdown → spendable=\(bal.spendable) unconfirmed=\(bal.unconfirmed) immature=\(bal.immature) locked=\(bal.locked) total=\(bal.total)"
        )
    }
}
