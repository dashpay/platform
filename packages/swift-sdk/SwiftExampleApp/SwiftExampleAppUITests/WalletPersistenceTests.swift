//
//  WalletPersistenceTests.swift
//  SwiftExampleAppUITests
//
//  SDK-backed integration tests that exercise the real SwiftData persister
//  + Keychain bootstrap path across app relaunches. These tests deliberately
//  do NOT use `-UITestResetState` or any in-memory ModelContainer hook —
//  doing so would defeat the SDK signal they are designed to give. Aborted
//  local runs may leave a wallet or an orphan-mnemonic recovery prompt on
//  the simulator; this is an intentional tradeoff documented in the PR.
//

import XCTest

final class WalletPersistenceTests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        return app
    }

    // MARK: - B-1

    /// Validates `walletManager.loadFromPersistor()` after a cold restart:
    /// SwiftData rehydration + Keychain read + the `rebindWalletScopedServices`
    /// chain. A wallet created in run #1 must come back in run #2.
    @MainActor
    func testWalletPersistsAcrossRelaunch() throws {
        let walletName = "PersistTest-\(UUID().uuidString.prefix(6))"

        // Best-effort teardown: if any assertion below halts the test before
        // the explicit delete in step 11, this re-launches a fresh app and
        // attempts to remove the wallet by name. Silent on failure.
        addTeardownBlock {
            // Teardown for UI tests runs on the main thread; assume the
            // isolation so we can call MainActor-isolated helpers.
            MainActor.assumeIsolated {
                let cleanupApp = XCUIApplication()
                cleanupApp.launch()
                bestEffortDeleteWallet(named: walletName, in: cleanupApp)
                cleanupApp.terminate()
            }
        }

        let app = launchApp()
        failIfRecoveryPromptVisible(in: app, timeout: 2)
        openWalletsTab(in: app)

        createGeneratedWallet(named: walletName, in: app)
        assertWalletRowVisible(named: walletName, in: app, exists: true, timeout: 20)

        app.terminate()

        let app2 = launchApp()
        failIfRecoveryPromptVisible(in: app2, timeout: 10)
        openWalletsTab(in: app2)
        assertWalletRowVisible(named: walletName, in: app2, exists: true, timeout: 15)

        deleteWallet(named: walletName, in: app2)
        assertWalletRowVisible(named: walletName, in: app2, exists: false)
    }

    // MARK: - B-2

    /// Validates that `deleteWallet` clears SwiftData and Keychain
    /// atomically. If either side leaks, the orphan-mnemonic recovery
    /// prompt fires on relaunch and the test fails. This is the strongest
    /// SDK-integration assertion in the suite.
    @MainActor
    func testWalletDeletionCleanupSurvivesRelaunch() throws {
        let walletName = "DeleteTest-\(UUID().uuidString.prefix(6))"

        // Defensive teardown: ordinarily the test deletes the wallet itself
        // in step 6, but if we fail mid-flow before delete, this catches the
        // residue. After the delete-then-relaunch sequence runs cleanly,
        // there's nothing for cleanup to find — that's expected.
        addTeardownBlock {
            // Teardown for UI tests runs on the main thread; assume the
            // isolation so we can call MainActor-isolated helpers.
            MainActor.assumeIsolated {
                let cleanupApp = XCUIApplication()
                cleanupApp.launch()
                bestEffortDeleteWallet(named: walletName, in: cleanupApp)
                cleanupApp.terminate()
            }
        }

        let app = launchApp()
        failIfRecoveryPromptVisible(in: app, timeout: 2)
        openWalletsTab(in: app)

        createGeneratedWallet(named: walletName, in: app)
        assertWalletRowVisible(named: walletName, in: app, exists: true, timeout: 20)

        deleteWallet(named: walletName, in: app)
        assertWalletRowVisible(named: walletName, in: app, exists: false)

        app.terminate()

        let app2 = launchApp()
        failIfRecoveryPromptVisible(in: app2, timeout: 10)    // ← key SDK assertion
        openWalletsTab(in: app2)
        assertWalletRowVisible(named: walletName, in: app2, exists: false, timeout: 15)
    }
}
