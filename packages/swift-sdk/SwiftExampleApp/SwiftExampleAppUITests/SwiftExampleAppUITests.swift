//
//  SwiftExampleAppUITests.swift
//  SwiftExampleAppUITests
//
//  Created by Sam Westrich on 8/6/25.
//

import XCTest

final class SwiftExampleAppUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testCreateGeneratedWalletFlow() throws {
        let app = XCUIApplication()
        app.launch()

        failIfRecoveryPromptVisible(in: app, timeout: 2)
        openWalletsTab(in: app)

        let walletName = "UITest Wallet \(UUID().uuidString.prefix(8))"
        createGeneratedWallet(named: walletName, in: app)

        assertWalletRowVisible(named: walletName, in: app, exists: true, timeout: 20)

        deleteWallet(named: walletName, in: app)
    }
}
