//
//  SwiftExampleAppUITests.swift
//  SwiftExampleAppUITests
//
//  Created by Sam Westrich on 8/6/25.
//

import XCTest

final class SwiftExampleAppUITests: XCTestCase {
    private enum Identifier {
        static let walletsTab = "rootTab.wallets"
        static let walletsScreen = "wallets.screen"
        static let addWalletButton = "wallets.addWalletButton"
        static let emptyCreateWalletButton = "wallets.empty.createWalletButton"
        static let walletNameField = "createWallet.walletNameField"
        static let pinField = "createWallet.pinField"
        static let confirmPinField = "createWallet.confirmPinField"
        static let createWalletButton = "createWallet.createButton"
        static let wroteItDownToggle = "seedBackup.wroteItDownToggle"
        static let confirmSeedCreateWalletButton = "seedBackup.createWalletButton"
        static let walletInfoButton = "walletDetail.infoButton"
        static let deleteWalletButton = "walletInfo.deleteWalletButton"
    }

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

        let createdWalletRow = scrollToWalletRow(named: walletName, in: app)
        XCTAssertTrue(
            createdWalletRow.waitForExistence(timeout: 20),
            "Expected created wallet row named \(walletName) to appear."
        )

        deleteWallet(named: walletName, in: app)
    }

    @MainActor
    private func openWalletsTab(in app: XCUIApplication) {
        let walletsScreen = element(Identifier.walletsScreen, in: app)
        if walletsScreen.exists {
            return
        }

        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(
            tabBar.waitForExistence(timeout: 60),
            "Expected root tab bar to appear after app initialization."
        )
        failIfRecoveryPromptVisible(in: app, timeout: 0)

        let walletsTab = app.tabBars.buttons
            .matching(identifier: Identifier.walletsTab)
            .firstMatch
        if walletsTab.waitForExistence(timeout: 2) {
            walletsTab.tap()
        } else {
            let labeledWalletsTab = app.tabBars.buttons["Wallets"]
            if labeledWalletsTab.waitForExistence(timeout: 2) {
                labeledWalletsTab.tap()
            } else {
                let indexedWalletsTab = app.tabBars.buttons.element(boundBy: 1)
                XCTAssertTrue(
                    indexedWalletsTab.waitForExistence(timeout: 5),
                    "Expected Wallets tab button to exist."
                )
                indexedWalletsTab.tap()
            }
        }

        XCTAssertTrue(
            walletsScreen.waitForExistence(timeout: 10)
                || app.navigationBars["Wallets"].waitForExistence(timeout: 1),
            "Expected Wallets screen after selecting Wallets tab."
        )
    }

    @MainActor
    private func createGeneratedWallet(named walletName: String, in app: XCUIApplication) {
        let addWalletButton = button(Identifier.addWalletButton, in: app)
        if addWalletButton.waitForExistence(timeout: 5) {
            addWalletButton.tap()
        } else {
            let emptyCreateButton = button(Identifier.emptyCreateWalletButton, in: app)
            XCTAssertTrue(
                emptyCreateButton.waitForExistence(timeout: 5),
                "Expected either the toolbar add button or empty-state create wallet button."
            )
            emptyCreateButton.tap()
        }

        XCTAssertTrue(
            app.navigationBars["Create Wallet"].waitForExistence(timeout: 10),
            "Expected Create Wallet sheet to open."
        )

        let walletNameField = textField(Identifier.walletNameField, in: app)
        XCTAssertTrue(walletNameField.waitForExistence(timeout: 5))
        walletNameField.tap()
        walletNameField.typeText(walletName)

        let pinField = secureTextField(Identifier.pinField, in: app)
        XCTAssertTrue(pinField.waitForExistence(timeout: 5))
        pinField.tap()
        pinField.typeText("1234")

        let confirmPinField = secureTextField(Identifier.confirmPinField, in: app)
        XCTAssertTrue(confirmPinField.waitForExistence(timeout: 5))
        confirmPinField.tap()
        confirmPinField.typeText("1234")

        let createButton = button(Identifier.createWalletButton, in: app)
        XCTAssertTrue(
            waitForElementToBeEnabled(createButton, timeout: 5),
            "Expected Create button to become enabled after valid wallet form input."
        )
        createButton.tap()

        XCTAssertTrue(
            app.navigationBars["Backup Seed"].waitForExistence(timeout: 10),
            "Expected Backup Seed screen after creating a generated recovery phrase."
        )

        let wroteItDownToggle = switchControl(Identifier.wroteItDownToggle, in: app)
        XCTAssertTrue(wroteItDownToggle.waitForExistence(timeout: 5))
        scrollUntilHittable(wroteItDownToggle, in: app)
        XCTAssertTrue(
            wroteItDownToggle.isHittable,
            "Expected seed backup confirmation switch to be hittable."
        )
        if !isSwitchOn(wroteItDownToggle) {
            wroteItDownToggle
                .coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5))
                .tap()
        }
        XCTAssertTrue(
            waitForSwitchToTurnOn(wroteItDownToggle, timeout: 5),
            "Expected seed backup confirmation switch to turn on."
        )

        let confirmCreateButton = button(Identifier.confirmSeedCreateWalletButton, in: app)
        XCTAssertTrue(
            waitForElementToBeEnabled(confirmCreateButton, timeout: 5),
            "Expected final Create Wallet button to enable after confirming seed backup."
        )
        confirmCreateButton.tap()
    }

    @MainActor
    private func deleteWallet(named walletName: String, in app: XCUIApplication) {
        let walletRow = scrollToWalletRow(named: walletName, in: app)
        XCTAssertTrue(
            walletRow.waitForExistence(timeout: 10),
            "Expected wallet row named \(walletName) before cleanup."
        )
        walletRow.tap()

        let infoButton = button(Identifier.walletInfoButton, in: app)
        XCTAssertTrue(infoButton.waitForExistence(timeout: 10))
        infoButton.tap()

        let deleteButton = button(Identifier.deleteWalletButton, in: app)
        scrollUntilHittable(deleteButton, in: app)
        XCTAssertTrue(
            deleteButton.exists && deleteButton.isHittable,
            "Expected Delete Wallet button to be reachable in Wallet Info."
        )
        deleteButton.tap()

        let deleteAlert = app.alerts["Delete Wallet"]
        XCTAssertTrue(deleteAlert.waitForExistence(timeout: 5))
        deleteAlert.buttons["Delete"].tap()

        XCTAssertTrue(
            waitForNonExistence(walletRow, timeout: 10),
            "Expected created wallet row named \(walletName) to disappear after cleanup."
        )
    }

    @MainActor
    private func failIfRecoveryPromptVisible(in app: XCUIApplication, timeout: TimeInterval) {
        // Title is plural for N>1 orphans, singular for one — match
        // either label in a single predicate-based query so the
        // total wait stays bounded by `timeout` and both variants
        // are detected symmetrically.
        let recoveryAlert = app.alerts.matching(
            NSPredicate(
                format: "label == %@ OR label == %@",
                "Recover Wallets?",
                "Recover Wallet?"
            )
        ).firstMatch
        if recoveryAlert.waitForExistence(timeout: timeout) {
            XCTFail(
                "Pre-existing orphan-mnemonic recovery alert is blocking the UI test. "
                + "Clean simulator state or resolve the alert manually before running this flow."
            )
        }
    }

    @MainActor
    private func scrollToWalletRow(named walletName: String, in app: XCUIApplication) -> XCUIElement {
        let row = app.buttons
            .matching(NSPredicate(format: "label == %@", walletName))
            .firstMatch
        for _ in 0..<8 where !row.exists {
            app.swipeUp()
        }
        if row.exists {
            return row
        }

        let label = app.staticTexts
            .matching(NSPredicate(format: "label == %@", walletName))
            .firstMatch
        for _ in 0..<8 where !label.exists {
            app.swipeUp()
        }
        return label
    }

    @MainActor
    private func scrollUntilHittable(_ element: XCUIElement, in app: XCUIApplication) {
        for _ in 0..<6 where !(element.exists && element.isHittable) {
            app.swipeUp()
        }
    }

    @MainActor
    private func element(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)
            .matching(identifier: identifier)
            .firstMatch
    }

    @MainActor
    private func button(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.buttons
            .matching(identifier: identifier)
            .firstMatch
    }

    @MainActor
    private func textField(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.textFields
            .matching(identifier: identifier)
            .firstMatch
    }

    @MainActor
    private func secureTextField(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.secureTextFields
            .matching(identifier: identifier)
            .firstMatch
    }

    @MainActor
    private func switchControl(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.switches
            .matching(identifier: identifier)
            .firstMatch
    }

    @MainActor
    private func waitForElementToBeEnabled(
        _ element: XCUIElement,
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate { object, _ in
            guard let element = object as? XCUIElement else { return false }
            return element.exists && element.isEnabled
        }
        let expectation = XCTNSPredicateExpectation(predicate: predicate, object: element)
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }

    @MainActor
    private func waitForNonExistence(
        _ element: XCUIElement,
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate { object, _ in
            guard let element = object as? XCUIElement else { return false }
            return !element.exists
        }
        let expectation = XCTNSPredicateExpectation(predicate: predicate, object: element)
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }

    @MainActor
    private func waitForSwitchToTurnOn(
        _ element: XCUIElement,
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate { object, _ in
            guard let element = object as? XCUIElement else { return false }
            guard let value = element.value as? String else { return false }
            return value == "1" || value.lowercased() == "true"
        }
        let expectation = XCTNSPredicateExpectation(predicate: predicate, object: element)
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }

    @MainActor
    private func isSwitchOn(_ element: XCUIElement) -> Bool {
        guard let value = element.value as? String else {
            return false
        }
        return value == "1" || value.lowercased() == "true"
    }

}
