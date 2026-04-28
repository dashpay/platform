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

// MARK: - Import

/// Drives `CreateWalletView` with the import toggle on. The import path
/// skips the seed-backup screen and goes straight to wallet creation.
@MainActor
func importWallet(
    named walletName: String,
    mnemonic: String,
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
            "Expected toolbar add or empty-state create button.",
            file: file, line: line
        )
        emptyCreateButton.tap()
    }

    XCTAssertTrue(
        app.navigationBars["Create Wallet"].waitForExistence(timeout: 10),
        "Expected Create Wallet sheet.",
        file: file, line: line
    )

    let nameField = textField(Identifier.walletNameField, in: app)
    XCTAssertTrue(nameField.waitForExistence(timeout: 5), file: file, line: line)
    nameField.tap()
    nameField.typeText(walletName)

    let pinFieldEl = secureTextField(Identifier.pinField, in: app)
    XCTAssertTrue(pinFieldEl.waitForExistence(timeout: 5), file: file, line: line)
    pinFieldEl.tap()
    pinFieldEl.typeText(pin)

    let confirmPinFieldEl = secureTextField(Identifier.confirmPinField, in: app)
    XCTAssertTrue(confirmPinFieldEl.waitForExistence(timeout: 5), file: file, line: line)
    confirmPinFieldEl.tap()
    confirmPinFieldEl.typeText(pin)

    let importToggle = switchControl(Identifier.importToggle, in: app)
    XCTAssertTrue(
        importToggle.waitForExistence(timeout: 5),
        "Expected Import Existing Wallet toggle.",
        file: file, line: line
    )
    scrollUntilHittable(importToggle, in: app)

    // SwiftUI Toggle in a Form is flaky to tap reliably — the
    // accessibility frame spans the whole row but only the switch
    // handle on the right toggles state. Try a couple of strategies in
    // sequence: right-edge coordinate (handle area), then a plain
    // .tap() (center of element). Bail as soon as the switch flips on.
    let tapStrategies: [() -> Void] = [
        {
            importToggle
                .coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5))
                .tap()
        },
        { importToggle.tap() },
        {
            importToggle
                .coordinate(withNormalizedOffset: CGVector(dx: 0.95, dy: 0.5))
                .tap()
        },
    ]
    var toggled = isSwitchOn(importToggle)
    for strategy in tapStrategies where !toggled {
        strategy()
        toggled = waitForSwitchToTurnOn(importToggle, timeout: 3)
    }
    XCTAssertTrue(
        toggled,
        "Expected Import toggle to turn on after \(tapStrategies.count) attempts.",
        file: file, line: line
    )

    let mnemonicField = textField(Identifier.mnemonicField, in: app)
    XCTAssertTrue(
        mnemonicField.waitForExistence(timeout: 5),
        "Expected mnemonic field after toggling Import.",
        file: file, line: line
    )
    mnemonicField.tap()
    mnemonicField.typeText(mnemonic)
    // Don't swipe down to dismiss the keyboard — the Create button lives
    // in the navigation bar, not under the keyboard, so the swipe is
    // unnecessary AND a sheet-on-app swipeDown will dismiss the sheet.

    let createButton = button(Identifier.createWalletButton, in: app)
    XCTAssertTrue(
        waitForElementToBeEnabled(createButton, timeout: 5),
        "Expected Create button to enable after filling import form.",
        file: file, line: line
    )
    createButton.tap()
}

// MARK: - Tab navigation (additional)

@MainActor
func openIdentitiesTab(
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    openTabByIdentifierOrLabel(
        idIdentifier: Identifier.identitiesTab,
        labelFallback: "Identities",
        boundByIndexFallback: 2,
        in: app,
        file: file, line: line
    )
}

@MainActor
func openSettingsTab(
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    openTabByIdentifierOrLabel(
        idIdentifier: Identifier.settingsTab,
        labelFallback: "Settings",
        boundByIndexFallback: 4,
        in: app,
        file: file, line: line
    )
}

@MainActor
private func openTabByIdentifierOrLabel(
    idIdentifier: String,
    labelFallback: String,
    boundByIndexFallback: Int,
    in app: XCUIApplication,
    file: StaticString,
    line: UInt
) {
    let tabBar = app.tabBars.firstMatch
    XCTAssertTrue(
        tabBar.waitForExistence(timeout: 60),
        "Expected root tab bar.",
        file: file, line: line
    )

    let byId = app.tabBars.buttons.matching(identifier: idIdentifier).firstMatch
    if byId.waitForExistence(timeout: 2) {
        byId.tap()
        return
    }
    let byLabel = app.tabBars.buttons[labelFallback]
    if byLabel.waitForExistence(timeout: 2) {
        byLabel.tap()
        return
    }
    let byIndex = app.tabBars.buttons.element(boundBy: boundByIndexFallback)
    XCTAssertTrue(
        byIndex.waitForExistence(timeout: 5),
        "Expected \(labelFallback) tab button.",
        file: file, line: line
    )
    byIndex.tap()
}

// MARK: - Network

/// Drives Settings → Network segmented picker to "Testnet". Idempotent.
/// Waits for `options.networkStatusLabel` to read "Connected" before
/// returning. Fails the test if the label reads "Disconnected" within
/// the timeout window.
@MainActor
func switchAppNetworkToTestnet(
    in app: XCUIApplication,
    timeout: TimeInterval = 30,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    openSettingsTab(in: app, file: file, line: line)

    // Try the identifier-scoped picker first, fall back to a generic
    // segmented control. SwiftUI exposes the segmented options as
    // children (buttons or NSSegmentedControl-equivalent), and the
    // outer identifier may not propagate to them on every OS version.
    let testnetSegmentInPicker = app.descendants(matching: .any)
        .matching(identifier: Identifier.Options.networkPicker)
        .firstMatch
        .buttons["Testnet"]
    let segmentedTestnet = app.segmentedControls.buttons["Testnet"]
    let testnetButton: XCUIElement = testnetSegmentInPicker.waitForExistence(timeout: 5)
        ? testnetSegmentInPicker
        : segmentedTestnet

    XCTAssertTrue(
        testnetButton.waitForExistence(timeout: 10),
        "Expected Testnet segment in network picker.",
        file: file, line: line
    )

    if testnetButton.isSelected {
        // Already on Testnet; status should already be Connected.
    } else {
        testnetButton.tap()
    }

    let statusLabel = app.descendants(matching: .any)
        .matching(identifier: Identifier.Options.networkStatusLabel)
        .firstMatch
    XCTAssertTrue(
        statusLabel.waitForExistence(timeout: 10),
        "Expected network status label.",
        file: file, line: line
    )
    let connectedPredicate = NSPredicate { object, _ in
        guard let element = object as? XCUIElement, element.exists else { return false }
        return element.label.contains("Connected")
    }
    let result = XCTWaiter.wait(
        for: [XCTNSPredicateExpectation(predicate: connectedPredicate, object: statusLabel)],
        timeout: timeout
    )
    XCTAssertEqual(
        result,
        .completed,
        "Network status did not reach 'Connected' within \(Int(timeout))s. Last label: \(statusLabel.label).",
        file: file, line: line
    )
    XCTAssertFalse(
        statusLabel.label.contains("Disconnected"),
        "Network status reported Disconnected after switching to Testnet.",
        file: file, line: line
    )
}

// MARK: - Identity discovery

@MainActor
func runIdentityDiscovery(
    forWalletNamed walletName: String,
    in app: XCUIApplication,
    timeout: TimeInterval = 60,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let addMenu = app.descendants(matching: .any)
        .matching(identifier: Identifier.Identities.addMenu)
        .firstMatch
    XCTAssertTrue(
        addMenu.waitForExistence(timeout: 10),
        "Expected Identities add menu.",
        file: file, line: line
    )

    // SwiftUI Menu popovers are flaky to drive — XCUITest sometimes
    // computes a `{-1, -1}` hit point on freshly-shown menu items, the
    // auto-retry then taps a stale element, and the sheet never opens.
    // Wrap "open menu, tap item, verify sheet" in a retry loop driven
    // by the actual signal (Search Wallets nav bar appears).
    let searchSheetNavBar = app.navigationBars["Search Wallets"]
    var sheetOpened = false
    for attempt in 1...3 where !sheetOpened {
        addMenu.tap()

        let searchMenuItem = app.descendants(matching: .any)
            .matching(identifier: Identifier.Identities.searchWalletsMenuItem)
            .firstMatch
        if searchMenuItem.waitForExistence(timeout: 3) {
            searchMenuItem.tap()
        } else {
            // Fallback: match the menu item by visible label.
            let labeled = app.buttons["Search Wallets for Identities"]
            if labeled.waitForExistence(timeout: 3) {
                labeled.tap()
            }
        }
        sheetOpened = searchSheetNavBar.waitForExistence(timeout: 5)
        _ = attempt
    }
    XCTAssertTrue(
        sheetOpened,
        "Expected Search Wallets sheet to open after tapping the Add menu item.",
        file: file, line: line
    )

    // Trust the picker's default-first auto-selection. The
    // CreditTransferTest deletes any leftover wallet and re-imports a
    // fresh one before this runs, so exactly one wallet is in the
    // picker — the one we want. Tapping the menu picker reliably to
    // pick a non-default option turns out to be flaky in XCUITest
    // (`pickerStyle(.menu)` keeps the dropdown overlay around long
    // enough to occlude the Search button below). Verify the picker
    // currently shows our wallet's label as a sanity check, then tap
    // Search.
    // Generous timeout — SearchWalletsForIdentitiesView gates the picker
    // on `hdWallets.isEmpty`, which is driven by an `@Query` over
    // PersistentWallet. After a fresh import the SwiftData write
    // → @Query update → view rerender takes a moment, and during that
    // window the view shows the "No wallets loaded" branch instead of
    // the picker. 20s comfortably covers the propagation lag.
    let walletPicker = app.descendants(matching: .any)
        .matching(identifier: Identifier.SearchWallets.walletPicker)
        .firstMatch
    XCTAssertTrue(
        walletPicker.waitForExistence(timeout: 20),
        "Expected the wallet picker. (Did SwiftData propagate the imported wallet to @Query?)",
        file: file, line: line
    )
    XCTAssertTrue(
        walletPicker.label.contains(walletName),
        "Picker shows \"\(walletPicker.label)\" but the test imported \(walletName). Was an unrelated wallet selected as default?",
        file: file, line: line
    )

    let searchButton = app.descendants(matching: .any)
        .matching(identifier: Identifier.SearchWallets.searchButton)
        .firstMatch
    XCTAssertTrue(
        waitForElementToBeEnabled(searchButton, timeout: 15),
        "Expected Search Wallet button to enable.",
        file: file, line: line
    )
    searchButton.tap()

    let foundCount = app.staticTexts
        .matching(identifier: Identifier.SearchWallets.foundCountLabel)
        .firstMatch
    let foundPredicate = NSPredicate { object, _ in
        guard let element = object as? XCUIElement, element.exists else { return false }
        let label = element.label
        return label.hasPrefix("+") && label != "+0"
    }
    XCTAssertEqual(
        XCTWaiter.wait(
            for: [XCTNSPredicateExpectation(predicate: foundPredicate, object: foundCount)],
            timeout: timeout
        ),
        .completed,
        "Expected discovery to find at least one identity within \(Int(timeout))s.",
        file: file, line: line
    )

    let doneButton = app.buttons["Done"]
    if doneButton.waitForExistence(timeout: 5) {
        doneButton.tap()
    }
}

@MainActor
func waitForIdentityRow(
    idBase58: String,
    in app: XCUIApplication,
    timeout: TimeInterval = 60,
    file: StaticString = #filePath,
    line: UInt = #line
) -> XCUIElement {
    let row = app.descendants(matching: .any)
        .matching(identifier: Identifier.Identities.row(idBase58))
        .firstMatch
    XCTAssertTrue(
        row.waitForExistence(timeout: timeout),
        "Expected identity row \(idBase58) within \(Int(timeout))s.",
        file: file, line: line
    )
    return row
}

// MARK: - Identity detail / balance

/// Reads the raw credit balance from `identityDetail.balanceLabel`'s
/// `accessibilityValue` (set to `"\(identity.balance)"` in IdentityDetailView).
/// Fails the test loudly if `.value` is empty or non-numeric — the rounded
/// display label hides sub-1000-credit deltas, and silent fallback there
/// would mask regressions.
@MainActor
func readIdentityBalanceCredits(
    in app: XCUIApplication,
    timeout: TimeInterval = 30,
    file: StaticString = #filePath,
    line: UInt = #line
) -> UInt64 {
    let label = app.descendants(matching: .any)
        .matching(identifier: Identifier.IdentityDetail.balanceLabel)
        .firstMatch
    XCTAssertTrue(
        label.waitForExistence(timeout: timeout),
        "Expected identityDetail.balanceLabel.",
        file: file, line: line
    )
    let displayLabel = label.label
    guard let raw = label.value as? String, !raw.isEmpty else {
        XCTFail(
            "identityDetail.balanceLabel has no accessibilityValue. Display label was \"\(displayLabel)\". "
            + "Did the .accessibilityValue modifier get dropped?",
            file: file, line: line
        )
        return 0
    }
    // iOS may apply locale-aware thousand separators to the accessibility
    // value string (e.g. "79 750 667 720" in French/German locales,
    // "79,750,667,720" in en-US). Strip non-digit characters before
    // parsing — we know the underlying value is a UInt64 credit count.
    let digits = raw.filter { $0.isASCII && $0.isNumber }
    guard !digits.isEmpty, let credits = UInt64(digits) else {
        XCTFail(
            "Could not parse \"\(raw)\" as UInt64 credits. Display label was \"\(displayLabel)\".",
            file: file, line: line
        )
        return 0
    }
    return credits
}

// MARK: - Credit transfer

/// Settings → State Transitions → Identity → Transfer Credits.
@MainActor
func navigateToIdentityCreditTransferForm(
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    openSettingsTab(in: app, file: file, line: line)

    // OptionsView's Form is lazy — cells below the fold (including the
    // Platform section's "State Transitions" cell) aren't in the
    // accessibility tree until we scroll them in.
    let stateTransitionsCell = app.buttons["State Transitions"]
    for _ in 0..<8 where !stateTransitionsCell.exists {
        app.swipeUp()
    }
    XCTAssertTrue(
        stateTransitionsCell.waitForExistence(timeout: 10),
        "Expected State Transitions cell in Settings.",
        file: file, line: line
    )
    stateTransitionsCell.tap()

    // The category rows in StateTransitionsView render an HStack with
    // icon + headline + description, so the button's accessibility label
    // is the composed text — `app.buttons["Identity"]` (exact label
    // match) fails. Match the description text, which is unique per
    // category, via CONTAINS.
    let identityCategory = app.buttons
        .matching(NSPredicate(format: "label CONTAINS[c] %@", "manage identities"))
        .firstMatch
    XCTAssertTrue(
        identityCategory.waitForExistence(timeout: 10),
        "Expected Identity category cell.",
        file: file, line: line
    )
    identityCategory.tap()

    // Same shape inside TransitionCategoryView — match by the unique
    // description "Transfer credits between identities".
    let transferCredits = app.buttons
        .matching(NSPredicate(format: "label CONTAINS[c] %@", "Transfer credits between identities"))
        .firstMatch
    XCTAssertTrue(
        transferCredits.waitForExistence(timeout: 10),
        "Expected Transfer Credits cell.",
        file: file, line: line
    )
    transferCredits.tap()

    XCTAssertTrue(
        app.navigationBars["Transfer Credits"].waitForExistence(timeout: 10),
        "Expected Transfer Credits form.",
        file: file, line: line
    )
}

/// Drive a credit-transfer state transition.
/// Sender selection: tap the senderIdentityPicker, tap the per-row option
/// matching the sender ID. Recipient handling covers both branches of
/// `recipientIdentityPicker` (the wallet's identity-only single-identity
/// case AND the multi-identity-on-simulator case).
@MainActor
func executeCreditTransfer(
    senderIdentityIdBase58: String,
    recipientIdentityIdBase58: String,
    amountCredits: UInt64,
    in app: XCUIApplication,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    // Sender selection.
    let senderPicker = app.descendants(matching: .any)
        .matching(identifier: Identifier.Transition.senderIdentityPicker)
        .firstMatch
    XCTAssertTrue(
        senderPicker.waitForExistence(timeout: 10),
        "Expected sender identity picker.",
        file: file, line: line
    )
    senderPicker.tap()
    let senderOptionId = Identifier.Transition.senderIdentityOption(senderIdentityIdBase58)
    let senderOption = app.descendants(matching: .any)
        .matching(identifier: senderOptionId)
        .firstMatch
    if senderOption.waitForExistence(timeout: 5) {
        senderOption.tap()
    } else {
        // Fallback: match by displayName prefix (first 12 chars + "...")
        let prefix = String(senderIdentityIdBase58.prefix(12))
        let labelPredicate = NSPredicate(format: "label BEGINSWITH %@", prefix)
        let senderByLabel = app.buttons.matching(labelPredicate).firstMatch
        XCTAssertTrue(
            senderByLabel.waitForExistence(timeout: 5),
            "Expected sender option \(senderIdentityIdBase58).",
            file: file, line: line
        )
        senderByLabel.tap()
    }

    // Wait for the picker's menu overlay to dismiss and the toIdentityId
    // form input wrapper to render. The menu overlay can occlude
    // descendants beneath it for a moment after a selection.
    let toIdentityWrapper = app.descendants(matching: .any)
        .matching(identifier: Identifier.Transition.input("toIdentityId"))
        .firstMatch
    XCTAssertTrue(
        toIdentityWrapper.waitForExistence(timeout: 15),
        "Expected toIdentityId input wrapper to render after sender selection. (Did the picker menu overlay get stuck open, or did selectedIdentityId not propagate?)",
        file: file, line: line
    )

    // Recipient: reach the manual-entry text field via either of the two
    // recipientIdentityPicker branches. Match descendants of the wrapper
    // to avoid picking up unrelated buttons elsewhere on screen.
    let manualButton = toIdentityWrapper.buttons
        .matching(identifier: Identifier.Transition.manualEntryButton("toIdentityId"))
        .firstMatch
    if manualButton.waitForExistence(timeout: 5) && manualButton.isHittable {
        manualButton.tap()
    } else {
        let recipientPicker = toIdentityWrapper.descendants(matching: .any)
            .matching(identifier: Identifier.Transition.recipientPicker("toIdentityId"))
            .firstMatch
        XCTAssertTrue(
            recipientPicker.waitForExistence(timeout: 10),
            "Expected either manual-entry button or recipient picker for toIdentityId.",
            file: file, line: line
        )
        recipientPicker.tap()
        let manualOption = app.buttons["💳 Manually Enter Recipient"]
        XCTAssertTrue(
            manualOption.waitForExistence(timeout: 5),
            "Expected 'Manually Enter Recipient' option in recipient picker menu.",
            file: file, line: line
        )
        manualOption.tap()
    }

    let recipientField = app.textFields
        .matching(identifier: Identifier.Transition.manualEntryField("toIdentityId"))
        .firstMatch
    XCTAssertTrue(
        recipientField.waitForExistence(timeout: 5),
        "Expected manual-entry recipient field.",
        file: file, line: line
    )
    recipientField.tap()
    recipientField.typeText(recipientIdentityIdBase58)

    // Amount.
    let amountWrapper = app.descendants(matching: .any)
        .matching(identifier: Identifier.Transition.input("amount"))
        .firstMatch
    XCTAssertTrue(
        amountWrapper.waitForExistence(timeout: 5),
        "Expected amount input wrapper.",
        file: file, line: line
    )
    let amountField = amountWrapper.textFields.firstMatch
    XCTAssertTrue(
        amountField.waitForExistence(timeout: 5),
        "Expected amount TextField.",
        file: file, line: line
    )
    amountField.tap()
    amountField.typeText(String(amountCredits))
    app.swipeDown()

    let executeButton = app.buttons
        .matching(identifier: Identifier.Transition.executeButton)
        .firstMatch
    XCTAssertTrue(
        waitForElementToBeEnabled(executeButton, timeout: 10),
        "Expected Execute Transition button to enable.",
        file: file, line: line
    )
    executeButton.tap()
}

@MainActor
func waitForCreditTransferSuccess(
    in app: XCUIApplication,
    timeout: TimeInterval = 30,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let resultStatus = app.staticTexts
        .matching(identifier: Identifier.Transition.resultStatusLabel)
        .firstMatch
    XCTAssertTrue(
        resultStatus.waitForExistence(timeout: timeout),
        "Expected transition result status label.",
        file: file, line: line
    )
    XCTAssertEqual(
        resultStatus.label,
        "Success",
        "Transition reported Error rather than Success.",
        file: file, line: line
    )
}

// MARK: - Pre-import cleanup

/// Deletes any wallet whose label starts with the given prefix. Used at
/// the start of the credit-transfer test to remove leftovers from prior
/// failed runs — re-importing the same mnemonic otherwise hits
/// `Wallet operation: Wallet already exists` because walletId is
/// deterministic from the mnemonic.
@MainActor
func cleanupWalletsByPrefix(_ prefix: String, in app: XCUIApplication) {
    let walletsScreen = element(Identifier.walletsScreen, in: app)
    guard walletsScreen.waitForExistence(timeout: 10) else { return }

    let predicate = NSPredicate(format: "label BEGINSWITH %@", prefix)
    var iteration = 0
    while iteration < 8 {
        iteration += 1
        let row = app.buttons.matching(predicate).firstMatch
        if !row.waitForExistence(timeout: 2) {
            return
        }
        let name = row.label
        bestEffortDeleteWallet(named: name, in: app)
    }
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
