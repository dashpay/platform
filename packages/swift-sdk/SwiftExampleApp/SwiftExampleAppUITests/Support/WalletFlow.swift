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

    let tappedToSwitch = !testnetButton.isSelected
    if tappedToSwitch {
        testnetButton.tap()
    }

    // Belt-and-braces: the AppState change makes the status label
    // honest about the rebind-in-progress window, but if the segmented-
    // control tap itself never landed (animation interrupted, picker
    // disabled mid-frame), `appState.currentNetwork` never changes,
    // `isSwitchingNetwork` stays false, and the label keeps reading
    // "Connected" against the *previous* network's SDK. Wait for the
    // segment to latch before trusting the connected predicate below.
    let selectedResult = XCTWaiter.wait(
        for: [XCTNSPredicateExpectation(
            predicate: NSPredicate(format: "isSelected == true"),
            object: testnetButton
        )],
        timeout: 10
    )
    XCTAssertEqual(
        selectedResult,
        .completed,
        "Testnet segment did not latch as selected within 10s. Did the segmented-control tap miss?",
        file: file, line: line
    )

    let statusLabel = app.descendants(matching: .any)
        .matching(identifier: Identifier.Options.networkStatusLabel)
        .firstMatch
    XCTAssertTrue(
        statusLabel.waitForExistence(timeout: 10),
        "Expected network status label.",
        file: file, line: line
    )

    // When we actually tapped to switch, observe the "Switching..." state
    // before trusting "Connected". The status label isn't network-aware
    // (it cycles between "Connected", "Switching...", "Disconnected"), so
    // a stale "Connected" from the *previous* network can satisfy the
    // predicate before the AppState chain (`currentNetwork.didSet` →
    // `beginNetworkSwitch` → `isSwitchingNetwork = true` → SwiftUI
    // rerender) has flipped the label. Observing "Switching..." first
    // proves that chain ran. Idempotent path (already on testnet) skips
    // this — there's no transition to wait for.
    if tappedToSwitch {
        let switchingPredicate = NSPredicate { object, _ in
            guard let element = object as? XCUIElement, element.exists else { return false }
            return element.label.contains("Switching")
        }
        let switchingResult = XCTWaiter.wait(
            for: [XCTNSPredicateExpectation(predicate: switchingPredicate, object: statusLabel)],
            timeout: 10
        )
        XCTAssertEqual(
            switchingResult,
            .completed,
            "Status label never showed 'Switching...' after the testnet tap. Either the AppState chain didn't fire or the switch completed faster than the XCUITest poll cadence. Last label: \(statusLabel.label).",
            file: file, line: line
        )
    }

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
    // by the actual signal (Re-scan for Identities nav bar appears).
    //
    // Each retry forces a fresh accessibility-tree snapshot for the
    // menu: if the previous iteration's item-tap missed and left the
    // menu open, close it first before re-opening. A naive
    // unconditional re-tap would toggle an already-open menu shut and
    // halve the effective retries.
    let searchSheetNavBar = app.navigationBars["Re-scan for Identities"]
    var sheetOpened = false
    for _ in 0..<3 where !sheetOpened {
        let searchMenuItem = app.descendants(matching: .any)
            .matching(identifier: Identifier.Identities.searchWalletsMenuItem)
            .firstMatch
        // If the menu is still open from a prior failed item-tap,
        // close it first so the open below forces a fresh
        // accessibility-tree snapshot. Tapping `addMenu` unconditionally
        // would toggle an already-open menu shut, halving the effective
        // retries.
        if searchMenuItem.exists {
            addMenu.tap()  // close stale menu
        }
        addMenu.tap()      // open fresh menu

        if searchMenuItem.waitForExistence(timeout: 3) {
            searchMenuItem.tap()
        } else {
            // Fallback: match the menu item by visible label.
            let labeled = app.buttons["Re-scan for Identities"]
            if labeled.waitForExistence(timeout: 3) {
                labeled.tap()
            }
        }
        sheetOpened = searchSheetNavBar.waitForExistence(timeout: 5)
    }
    XCTAssertTrue(
        sheetOpened,
        "Expected Re-scan for Identities sheet to open after tapping the Add menu item.",
        file: file, line: line
    )

    // Drive the picker explicitly — we can't trust the default-first
    // auto-selection. SearchWalletsForIdentitiesView's `@Query` over
    // PersistentWallet is unfiltered and sorted by createdAt, so any
    // older wallet on the simulator (e.g. one a developer created
    // outside this test) wins the default selection.
    //
    // Generous timeout: the SwiftData write → @Query update → view
    // rerender takes a moment after a fresh import. During that window
    // the view shows the "No wallets loaded" branch instead of the
    // picker. 20s comfortably covers the propagation lag.
    let walletPicker = app.descendants(matching: .any)
        .matching(identifier: Identifier.SearchWallets.walletPicker)
        .firstMatch
    XCTAssertTrue(
        walletPicker.waitForExistence(timeout: 20),
        "Expected the wallet picker. (Did SwiftData propagate the imported wallet to @Query?)",
        file: file, line: line
    )
    if !walletPicker.label.contains(walletName) {
        // Open the .menu popover and tap the row whose accessibility
        // label starts with our wallet name. `walletPickerRow` renders
        // `HStack { Text(label), Text(fingerprint) }`, which combines
        // into `"<walletName> <fingerprint>"` on the row's button. Match
        // with a trailing space so a longer wallet name that shares a
        // prefix can't accidentally win firstMatch.
        walletPicker.tap()
        let walletOption = app.buttons
            .matching(NSPredicate(format: "label BEGINSWITH %@", "\(walletName) "))
            .firstMatch
        XCTAssertTrue(
            walletOption.waitForExistence(timeout: 5),
            "Expected wallet menu option for \(walletName).",
            file: file, line: line
        )
        walletOption.tap()
    }
    // SwiftUI takes a frame to update the picker's collapsed label after
    // the menu option is tapped — a bare `.label.contains` check races
    // on slower simulators. Wait for the propagation explicitly.
    let selectedPredicate = NSPredicate { object, _ in
        guard let element = object as? XCUIElement, element.exists else { return false }
        return element.label.contains(walletName)
    }
    let selectedResult = XCTWaiter.wait(
        for: [XCTNSPredicateExpectation(predicate: selectedPredicate, object: walletPicker)],
        timeout: 5
    )
    XCTAssertEqual(
        selectedResult,
        .completed,
        "Picker shows \"\(walletPicker.label)\" but the test imported \(walletName).",
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
        // SearchWalletsForIdentitiesView renders `"+\(foundCount)"`, so
        // require literally "+<digits>" (excluding "+0") rather than
        // accepting any "+"-prefixed string. Defends against future
        // label format drift without coupling to a specific count.
        return label.hasPrefix("+")
            && label.dropFirst().allSatisfy(\.isNumber)
            && label != "+0"
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

// MARK: - Pre-import cleanup

/// Deletes any wallet whose label starts with the given prefix. Used at
/// the start of the credit-transfer test to remove leftovers from prior
/// failed runs — re-importing the same mnemonic otherwise hits
/// `Wallet operation: Wallet already exists` because walletId is
/// deterministic from the mnemonic.
///
/// Bails as soon as no matching row is visible: an earlier full-sweep
/// implementation issued blind `swipeUp` calls on an empty wallets list
/// and routinely tripped XCUITest's 60s event-synthesis timeout (per
/// swipe), blowing the test runtime out by ~20+ minutes. If a developer
/// has accumulated more `ImportTransfer-*` wallets than the viewport
/// can hold, `simctl erase` is the right recovery (documented in
/// SwiftExampleAppUITests/README.md).
@MainActor
func cleanupWalletsByPrefix(_ prefix: String, in app: XCUIApplication) {
    let walletsScreen = element(Identifier.walletsScreen, in: app)
    guard walletsScreen.waitForExistence(timeout: 10) else { return }

    let predicate = NSPredicate(format: "label BEGINSWITH %@", prefix)
    for _ in 0..<10 {
        let row = app.buttons.matching(predicate).firstMatch
        if !row.waitForExistence(timeout: 1) {
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
    // If a previous failure left Keychain mnemonics behind without
    // matching SwiftData rows, the cold-launch shows the orphan-mnemonic
    // recovery prompt before any UI we care about. Dismiss it
    // best-effort so the rest of the helper isn't silently no-oped by
    // a modal blocking the wallets tab. The prompt's "Cancel" button
    // declines the recovery offer (we then proceed with the deletion
    // we came here to do).
    // Match both the singular ("Recover Wallet?") and plural
    // ("Recover Wallets?", N>1 orphans) titles so the teardown
    // dismisses either variant.
    let recoverAlert = app.alerts.matching(
        NSPredicate(
            format: "label == %@ OR label == %@",
            "Recover Wallets?",
            "Recover Wallet?"
        )
    ).firstMatch
    if recoverAlert.waitForExistence(timeout: 1) {
        if recoverAlert.buttons["Cancel"].exists {
            recoverAlert.buttons["Cancel"].tap()
        } else if recoverAlert.buttons["Don't Recover"].exists {
            recoverAlert.buttons["Don't Recover"].tap()
        } else {
            // Last-ditch: tap whatever the dismissive button is by index.
            recoverAlert.buttons.element(boundBy: 0).tap()
        }
    }

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
/// `scrollToWalletRow`. For `exists: false` it scrolls back to the top
/// and sweeps down — SwiftUI Lists are lazy, and a still-persisted row
/// off-screen would otherwise let the absence predicate evaluate true
/// even though deletion or relaunch cleanup actually failed.
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

    // Reset to the top of the list, then sweep down. If the row appears
    // at any scroll position, fail loudly — its presence anywhere in the
    // list means deletion didn't actually happen.
    for _ in 0..<6 { app.swipeDown() }

    let buttonRow = app.buttons
        .matching(NSPredicate(format: "label == %@", walletName))
        .firstMatch
    let textRow = app.staticTexts
        .matching(NSPredicate(format: "label == %@", walletName))
        .firstMatch

    for _ in 0..<10 {
        if buttonRow.exists || textRow.exists {
            XCTFail(
                "Expected wallet row \(walletName) to be absent, but found during sweep.",
                file: file,
                line: line
            )
            return
        }
        app.swipeUp()
    }

    let absencePredicate = NSPredicate { _, _ in
        !buttonRow.exists && !textRow.exists
    }
    let expectation = XCTNSPredicateExpectation(predicate: absencePredicate, object: app)
    let result = XCTWaiter.wait(for: [expectation], timeout: timeout)
    XCTAssertEqual(
        result,
        .completed,
        "Expected wallet row \(walletName) to be absent after sweep.",
        file: file,
        line: line
    )
}
