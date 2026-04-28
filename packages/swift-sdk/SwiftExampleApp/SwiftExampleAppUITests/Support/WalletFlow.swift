//
//  WalletFlow.swift
//  SwiftExampleAppUITests
//
//  Wallet-specific UI flows shared across test classes. The create/delete
//  flows are exactly what `testCreateGeneratedWalletFlow` ran inline before
//  the extraction; `assertWalletRowVisible` mirrors `scrollToWalletRow`'s
//  buttons-first / staticTexts-fallback strategy so it works against a
//  `NavigationLink` row whose accessibility label is the wallet name.
//

import XCTest

// MARK: - Tab navigation

@MainActor
func openWalletsTab(
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let walletsScreen = element(Identifier.walletsScreen, in: app)
    if walletsScreen.exists {
        return
    }

    let tabBar = app.tabBars.firstMatch
    XCTAssertTrue(
        tabBar.waitForExistence(timeout: 60),
        "Expected root tab bar to appear after app initialization.",
        file: file,
        line: line
    )
    failIfRecoveryPromptVisible(in: app, timeout: 0, file: file, line: line)

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
                "Expected Wallets tab button to exist.",
                file: file,
                line: line
            )
            indexedWalletsTab.tap()
        }
    }

    XCTAssertTrue(
        walletsScreen.waitForExistence(timeout: 10)
            || app.navigationBars["Wallets"].waitForExistence(timeout: 1),
        "Expected Wallets screen after selecting Wallets tab.",
        file: file,
        line: line
    )
}

// MARK: - Create / delete flows

@MainActor
func createGeneratedWallet(
    named walletName: String,
    pin: String = "1234",
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let addWalletButton = button(Identifier.addWalletButton, in: app)
    if addWalletButton.waitForExistence(timeout: 5) {
        addWalletButton.tap()
    } else {
        let emptyCreateButton = button(Identifier.emptyCreateWalletButton, in: app)
        XCTAssertTrue(
            emptyCreateButton.waitForExistence(timeout: 5),
            "Expected either the toolbar add button or empty-state create wallet button.",
            file: file,
            line: line
        )
        emptyCreateButton.tap()
    }

    XCTAssertTrue(
        app.navigationBars["Create Wallet"].waitForExistence(timeout: 10),
        "Expected Create Wallet sheet to open.",
        file: file,
        line: line
    )

    let walletNameField = textField(Identifier.walletNameField, in: app)
    XCTAssertTrue(walletNameField.waitForExistence(timeout: 5), file: file, line: line)
    walletNameField.tap()
    walletNameField.typeText(walletName)

    let pinField = secureTextField(Identifier.pinField, in: app)
    XCTAssertTrue(pinField.waitForExistence(timeout: 5), file: file, line: line)
    pinField.tap()
    pinField.typeText(pin)

    let confirmPinField = secureTextField(Identifier.confirmPinField, in: app)
    XCTAssertTrue(confirmPinField.waitForExistence(timeout: 5), file: file, line: line)
    confirmPinField.tap()
    confirmPinField.typeText(pin)

    let createButton = button(Identifier.createWalletButton, in: app)
    XCTAssertTrue(
        waitForElementToBeEnabled(createButton, timeout: 5),
        "Expected Create button to become enabled after valid wallet form input.",
        file: file,
        line: line
    )
    createButton.tap()

    XCTAssertTrue(
        app.navigationBars["Backup Seed"].waitForExistence(timeout: 10),
        "Expected Backup Seed screen after creating a generated recovery phrase.",
        file: file,
        line: line
    )

    let wroteItDownToggle = switchControl(Identifier.wroteItDownToggle, in: app)
    XCTAssertTrue(wroteItDownToggle.waitForExistence(timeout: 5), file: file, line: line)
    scrollUntilHittable(wroteItDownToggle, in: app)
    XCTAssertTrue(
        wroteItDownToggle.isHittable,
        "Expected seed backup confirmation switch to be hittable.",
        file: file,
        line: line
    )
    if !isSwitchOn(wroteItDownToggle) {
        wroteItDownToggle
            .coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5))
            .tap()
    }
    XCTAssertTrue(
        waitForSwitchToTurnOn(wroteItDownToggle, timeout: 5),
        "Expected seed backup confirmation switch to turn on.",
        file: file,
        line: line
    )

    let confirmCreateButton = button(Identifier.confirmSeedCreateWalletButton, in: app)
    XCTAssertTrue(
        waitForElementToBeEnabled(confirmCreateButton, timeout: 5),
        "Expected final Create Wallet button to enable after confirming seed backup.",
        file: file,
        line: line
    )
    confirmCreateButton.tap()
}

@MainActor
func deleteWallet(
    named walletName: String,
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let walletRow = scrollToWalletRow(named: walletName, in: app)
    XCTAssertTrue(
        walletRow.waitForExistence(timeout: 10),
        "Expected wallet row named \(walletName) before cleanup.",
        file: file,
        line: line
    )
    walletRow.tap()

    let infoButton = button(Identifier.walletInfoButton, in: app)
    XCTAssertTrue(infoButton.waitForExistence(timeout: 10), file: file, line: line)
    infoButton.tap()

    let deleteButton = button(Identifier.deleteWalletButton, in: app)
    scrollUntilHittable(deleteButton, in: app)
    XCTAssertTrue(
        deleteButton.exists && deleteButton.isHittable,
        "Expected Delete Wallet button to be reachable in Wallet Info.",
        file: file,
        line: line
    )
    deleteButton.tap()

    let deleteAlert = app.alerts["Delete Wallet"]
    XCTAssertTrue(deleteAlert.waitForExistence(timeout: 5), file: file, line: line)
    deleteAlert.buttons["Delete"].tap()

    XCTAssertTrue(
        waitForNonExistence(walletRow, timeout: 10),
        "Expected created wallet row named \(walletName) to disappear after cleanup.",
        file: file,
        line: line
    )
}

// MARK: - Best-effort cleanup

/// Best-effort wallet deletion for teardown blocks — does not assert. Used
/// by relaunch tests so an aborted assertion mid-flow doesn't leave a real
/// wallet on the developer's simulator. Silent on every "not found" path,
/// because if any required element is missing there's nothing useful to
/// clean up. This mirrors `deleteWallet` step-for-step but with bailouts
/// instead of XCTAssert calls.
@MainActor
func bestEffortDeleteWallet(named walletName: String, in app: XCUIApplication) {
    let walletsScreen = element(Identifier.walletsScreen, in: app)
    if !walletsScreen.exists {
        let walletsTab = app.tabBars.buttons
            .matching(identifier: Identifier.walletsTab)
            .firstMatch
        if walletsTab.waitForExistence(timeout: 30) {
            walletsTab.tap()
        } else if app.tabBars.buttons["Wallets"].waitForExistence(timeout: 2) {
            app.tabBars.buttons["Wallets"].tap()
        }
        guard walletsScreen.waitForExistence(timeout: 10)
            || app.navigationBars["Wallets"].waitForExistence(timeout: 1)
        else { return }
    }

    let row = app.buttons
        .matching(NSPredicate(format: "label == %@", walletName))
        .firstMatch
    for _ in 0..<8 where !row.exists {
        app.swipeUp()
    }
    guard row.waitForExistence(timeout: 5) else { return }
    row.tap()

    let infoButton = button(Identifier.walletInfoButton, in: app)
    guard infoButton.waitForExistence(timeout: 5) else { return }
    infoButton.tap()

    let deleteButton = button(Identifier.deleteWalletButton, in: app)
    scrollUntilHittable(deleteButton, in: app)
    guard deleteButton.exists, deleteButton.isHittable else { return }
    deleteButton.tap()

    let deleteAlert = app.alerts["Delete Wallet"]
    if deleteAlert.waitForExistence(timeout: 5) {
        deleteAlert.buttons["Delete"].tap()
    }
}

// MARK: - Row lookup / assertion

/// Mirrors the original `scrollToWalletRow`: each wallet row is a
/// `NavigationLink` (a button in the accessibility tree) wrapping
/// `WalletRowView`, with `.accessibilityLabel(wallet.label)` set on the
/// link. Match by buttons first; fall back to staticTexts for surfaces
/// where the wallet name is rendered as plain text.
@MainActor
func scrollToWalletRow(named walletName: String, in app: XCUIApplication) -> XCUIElement {
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

/// Assert a wallet row's presence (or absence) by name. For `exists: true`
/// this scrolls up to ~16 swipes to find the row, mirroring
/// `scrollToWalletRow`. For `exists: false` it does not scroll — deleted
/// wallets disappear in place, so we wait for both the buttons and
/// staticTexts predicate matches to fail at the current scroll position.
@MainActor
func assertWalletRowVisible(
    named walletName: String,
    in app: XCUIApplication,
    exists: Bool,
    timeout: TimeInterval = 10,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    if exists {
        let row = scrollToWalletRow(named: walletName, in: app)
        XCTAssertTrue(
            row.waitForExistence(timeout: timeout),
            "Expected wallet row \(walletName) to be visible.",
            file: file,
            line: line
        )
        return
    }

    let buttonRow = app.buttons
        .matching(NSPredicate(format: "label == %@", walletName))
        .firstMatch
    let textRow = app.staticTexts
        .matching(NSPredicate(format: "label == %@", walletName))
        .firstMatch
    let absencePredicate = NSPredicate { _, _ in
        !buttonRow.exists && !textRow.exists
    }
    let expectation = XCTNSPredicateExpectation(predicate: absencePredicate, object: app)
    let result = XCTWaiter.wait(for: [expectation], timeout: timeout)
    XCTAssertEqual(
        result,
        .completed,
        "Expected wallet row \(walletName) to be absent.",
        file: file,
        line: line
    )
}
