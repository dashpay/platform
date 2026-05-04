//
//  CreditTransferTest.swift
//  SwiftExampleAppUITests
//
//  Imports a wallet from a known testnet mnemonic that already has a
//  registered identity, runs identity discovery, and asserts that the
//  expected identity surfaces with a non-zero balance. The fixture
//  identity is intentionally pre-funded; if it ever drains, top it up
//  rather than weaken the assertion (otherwise a regression that breaks
//  balance discovery would render `0` and silently pass —
//  IdentityDetailView.onAppear doesn't refresh balance). The credit-
//  transfer assertion itself is deferred to a follow-up.
//
//  Skipped automatically when the env var is unset, so the rest of the
//  suite can run locally without test-network credentials.
//
//  Env var:
//    * UI_TEST_MNEMONIC — sender wallet's 12-word phrase
//
//  Note on env var forwarding: a previous run on this branch showed that
//  `xcodebuild test ENV=...` did not propagate env vars to the XCUITest
//  runner — only the prefix form `TEST_RUNNER_<NAME>` reached the test
//  process (Xcode strips the prefix). Try the unprefixed form first; if
//  the env var doesn't reach the test, use the prefixed form.
//

import XCTest

final class CreditTransferTest: XCTestCase {
    // The two constants below are deterministic functions of the
    // mnemonic stored in the `UI_TEST_MNEMONIC` GitHub Actions secret.
    // **They MUST be regenerated whenever the secret is rotated** — a
    // mismatched fixture will surface as an opaque
    // `waitForIdentityRow` timeout ("identity row not found within
    // 60s") with no breadcrumb pointing here.
    //
    // Regeneration steps (manual, one-time per rotation):
    //   1. `simctl erase` a fresh simulator.
    //   2. Run `testImportWalletAndDiscoverIdentity` locally with the
    //      new mnemonic in `TEST_RUNNER_UI_TEST_MNEMONIC`.
    //   3. Watch the Wallets tab for the `wallets.walletRow.<hex>`
    //      identifier on the imported wallet — that hex is
    //      `expectedSenderWalletIdHex`.
    //   4. After discovery, watch the Identities tab for the
    //      `identities.row.<base58>` identifier on the discovered
    //      identity — that base58 is `expectedSenderIdentityIdBase58`.
    //   5. Paste both here and update the PR.

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
        guard let mnemonic = ProcessInfo.processInfo.environment["UI_TEST_MNEMONIC"],
              !mnemonic.isEmpty
        else {
            throw XCTSkip("Set UI_TEST_MNEMONIC to run this test.")
        }
        XCTAssertEqual(
            mnemonic.split(separator: " ").count,
            12,
            "UI_TEST_MNEMONIC must be a 12-word phrase."
        )

        let app = XCUIApplication()
        app.launch()
        failIfRecoveryPromptVisible(in: app, timeout: 2)

        // Force testnet — the simulator may have been left on a non-testnet
        // network by previous runs. Idempotent if already on Testnet.
        switchAppNetworkToTestnet(in: app)
        // Re-run the recovery-prompt guard. A leftover testnet wallet on a
        // simulator booted to a different default network only triggers
        // the orphan-mnemonic prompt after the network switch rebinds
        // wallet-scoped services, so the pre-switch guard above misses it.
        failIfRecoveryPromptVisible(in: app, timeout: 2)

        openWalletsTab(in: app)

        // Sweep wallets from prior failed runs of this test. Each run uses
        // a random `ImportTransfer-<UUIDsuffix>` name, so the deterministic
        // walletId-based check below misses them — they accumulate on the
        // simulator and eventually clash with `importWallet` (`Wallet
        // already exists`).
        cleanupWalletsByPrefix("ImportTransfer-", in: app)

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

        // The fixture identity is pre-funded; assert > 0 so a regression
        // that breaks balance discovery (returns 0) actually fails this
        // test. `IdentityDetailView.onAppear` only refreshes
        // DPNS/DashPay/tokens — it doesn't refresh balance — so without
        // this floor the assertion would only check parseability and a
        // 0-credit render would silently pass.
        let credits = readIdentityBalanceCredits(in: app)
        XCTAssertGreaterThan(
            credits,
            0,
            "Expected discovered identity \(expectedSenderIdentityIdBase58) to surface a non-zero balance. If the testnet fixture identity has drained, top it up rather than weaken this assertion."
        )
    }
}
