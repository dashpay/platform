//
//  CreditTransferTest.swift
//  SwiftExampleAppUITests
//
//  Imports a wallet from a known testnet mnemonic that already has a
//  registered identity, runs identity discovery, and asserts that the
//  expected identity surfaces with a non-zero balance. The credit-
//  transfer assertion is deferred to a follow-up.
//
//  Skipped automatically when the env var is unset, so the rest of the
//  suite can run locally without test-network credentials.
//
//  Env var:
//    * UI_TEST_TESTNET_MNEMONIC — sender wallet's 12-word phrase
//
//  Note on env var forwarding: a previous run on this branch showed that
//  `xcodebuild test ENV=...` did not propagate env vars to the XCUITest
//  runner — only the prefix form `TEST_RUNNER_<NAME>` reached the test
//  process (Xcode strips the prefix). Try the unprefixed form first; if
//  the env var doesn't reach the test, use the prefixed form.
//

import XCTest

final class CreditTransferTest: XCTestCase {
    /// The pre-registered identity behind the sender mnemonic. Discovery
    /// must surface this exact ID — that's the regression check on the
    /// discovery path.
    private let expectedSenderIdentityIdBase58 = "3ou98WEERy6ExmmHWYWsFtyhgW8rmr1giceZYTFqdAAA"

    /// walletId derived from the test mnemonic (deterministic). Lets us
    /// detect an already-imported wallet from a prior run and reuse it
    /// instead of failing on `Wallet operation: Wallet already exists`.
    private let expectedSenderWalletIdHex = "2450ec6b6dc2b1b0476875305a6870dee743d47474e3838642b655b68a600793"

    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testImportWalletAndDiscoverIdentity() throws {
        guard let mnemonic = ProcessInfo.processInfo.environment["UI_TEST_TESTNET_MNEMONIC"],
              !mnemonic.isEmpty
        else {
            throw XCTSkip("Set UI_TEST_TESTNET_MNEMONIC to run this test.")
        }
        XCTAssertEqual(
            mnemonic.split(separator: " ").count,
            12,
            "UI_TEST_TESTNET_MNEMONIC must be a 12-word phrase."
        )

        let app = XCUIApplication()
        app.launch()
        failIfRecoveryPromptVisible(in: app, timeout: 2)

        // Force testnet — the simulator may have been left on a non-testnet
        // network by previous runs. Idempotent if already on Testnet.
        switchAppNetworkToTestnet(in: app)

        openWalletsTab(in: app)

        // Wallets restored from the persister on cold launch come back
        // watch-only — the SwiftExampleApp comment in SwiftExampleAppApp.swift
        // notes that biometric unlock to rehydrate signing keys is "future
        // work". Identity discovery needs private keys, so a leftover
        // wallet from a prior run is unusable. Delete it (if present) and
        // re-import to get a hot, signing-capable wallet.
        let existingRow = app.descendants(matching: .any)
            .matching(identifier: "wallets.walletRow.\(expectedSenderWalletIdHex)")
            .firstMatch
        if existingRow.waitForExistence(timeout: 5) {
            // accessibilityLabel == wallet.label, so .label gives us the name.
            let staleName = existingRow.label
            bestEffortDeleteWallet(named: staleName, in: app)
        }

        let walletName = "ImportTransfer-\(UUID().uuidString.prefix(6))"
        addTeardownBlock {
            MainActor.assumeIsolated {
                let cleanupApp = XCUIApplication()
                cleanupApp.launch()
                bestEffortDeleteWallet(named: walletName, in: cleanupApp)
                cleanupApp.terminate()
            }
        }
        importWallet(named: walletName, mnemonic: mnemonic, in: app)
        assertWalletRowVisible(named: walletName, in: app, exists: true, timeout: 30)

        openIdentitiesTab(in: app)
        runIdentityDiscovery(forWalletNamed: walletName, in: app)

        let senderRow = waitForIdentityRow(idBase58: expectedSenderIdentityIdBase58, in: app)
        senderRow.tap()

        let balance = readIdentityBalanceCredits(in: app)
        XCTAssertGreaterThan(
            balance,
            0,
            "Discovered identity should have a non-zero balance."
        )
    }
}
