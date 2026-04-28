//
//  XCUIElement+Helpers.swift
//  SwiftExampleAppUITests
//
//  Shared XCUITest helpers — element lookup, predicate-driven waits, and
//  the orphan-mnemonic recovery-prompt guard. Relocated from
//  SwiftExampleAppUITests.swift so multiple test classes can share them.
//  Behavior matches the previous private implementations exactly.
//

import XCTest

// MARK: - Element lookup

@MainActor
func element(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
    app.descendants(matching: .any)
        .matching(identifier: identifier)
        .firstMatch
}

@MainActor
func button(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
    app.buttons
        .matching(identifier: identifier)
        .firstMatch
}

@MainActor
func textField(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
    app.textFields
        .matching(identifier: identifier)
        .firstMatch
}

@MainActor
func secureTextField(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
    app.secureTextFields
        .matching(identifier: identifier)
        .firstMatch
}

@MainActor
func switchControl(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
    app.switches
        .matching(identifier: identifier)
        .firstMatch
}

// MARK: - Waits

@MainActor
func waitForElementToBeEnabled(
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
func waitForNonExistence(
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
func waitForSwitchToTurnOn(
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
func isSwitchOn(_ element: XCUIElement) -> Bool {
    guard let value = element.value as? String else {
        return false
    }
    return value == "1" || value.lowercased() == "true"
}

@MainActor
func scrollUntilHittable(_ element: XCUIElement, in app: XCUIApplication) {
    for _ in 0..<6 where !(element.exists && element.isHittable) {
        app.swipeUp()
    }
}

// MARK: - Recovery-prompt guard

/// Fails the running test if the orphan-mnemonic "Recover Wallet?" alert is
/// already on screen. Used at the start of any test that depends on a clean
/// wallet state — pre-existing residue from an aborted run would otherwise
/// silently change the flow under test.
@MainActor
func failIfRecoveryPromptVisible(
    in app: XCUIApplication,
    timeout: TimeInterval,
    file: StaticString = #filePath,
    line: UInt = #line
) {
    let recoverWalletAlert = app.alerts["Recover Wallet?"]
    if recoverWalletAlert.waitForExistence(timeout: timeout) {
        XCTFail(
            "Pre-existing orphan-mnemonic recovery alert is blocking the UI test. "
            + "Clean simulator state or resolve the alert manually before running this flow.",
            file: file,
            line: line
        )
    }
}
