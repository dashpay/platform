import XCTest

/// DashPay tab smoke tests.
///
/// Network-free: these assert only the identity-picker states the
/// DashPay tab renders from local state — no wallet, no funded
/// identity, no testnet round-trips. They are the launch-and-render
/// gate for the tab, keyed on the `dashpay.*` accessibility ids.
///
/// TODO (gated on a funded testnet wallet): the full
/// add → approve → pay XCUITest — AddContact by DPNS →
/// request appears in Outgoing → (peer accepts) → appears in Contacts
/// → open contact → Send Dash → confirm txid — needs two funded
/// testnet identities (one driven out-of-band to accept), so it is
/// deliberately NOT implemented here.
final class DashPayTabUITests: XCTestCase {

    private enum Identifier {
        /// On the tab *content* view (same pattern as
        /// `rootTab.wallets`); the tab-bar button itself may only be
        /// reachable by its "DashPay" label.
        static let dashpayTab = "dashpay.tab"
        static let openWalletsButton = "dashpay.openWallets"
        static let openIdentitiesButton = "dashpay.openIdentities"
        static let identityPicker = "dashpay.identityPicker"
        static let segment = "dashpay.segment"
        static let addContactButton = "dashpay.addContact"
        static let refreshButton = "dashpay.refresh"
        static let openIgnoredButton = "dashpay.openIgnored"
        static let openHiddenLink = "dashpay.openHidden"
    }

    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    /// Launch → open the DashPay tab → the tab must render exactly one
    /// of the picker states:
    ///   1. no wallet            → "Open Wallets" CTA
    ///   2. wallet, no identity  → "Open Identities" CTA
    ///   3. ≥1 eligible identity → segmented [Contacts | Requests]
    /// On a fresh simulator state 1 is what we expect, but the test
    /// accepts any of the three so it stays valid on a machine with
    /// leftover local wallets — the invariant is "the tab renders a
    /// recognized state", not "the simulator is fresh".
    @MainActor
    func testDashPayTabRendersAPickerState() throws {
        let app = XCUIApplication()
        app.launch()

        openDashPayTab(in: app)

        let openWallets = app.buttons
            .matching(identifier: Identifier.openWalletsButton).firstMatch
        let openIdentities = app.buttons
            .matching(identifier: Identifier.openIdentitiesButton).firstMatch
        let segment = app.descendants(matching: .any)
            .matching(identifier: Identifier.segment).firstMatch

        let landed = waitForAny(
            [openWallets, openIdentities, segment],
            timeout: 30
        )
        XCTAssertTrue(
            landed,
            "DashPay tab must render one of the §6.4 states: no-wallet CTA, "
                + "no-identity CTA, or the Contacts/Requests segment."
        )

        // The toolbar AddContact entry point exists in every state
        // (disabled until an identity is active) — its presence is the
        // contract the add→approve→pay flow will key on.
        let addContact = app.buttons
            .matching(identifier: Identifier.addContactButton).firstMatch
        XCTAssertTrue(
            addContact.waitForExistence(timeout: 10),
            "dashpay.addContact toolbar button must exist on the DashPay tab."
        )

        if openWallets.exists || openIdentities.exists {
            // States 1–2: no active identity ⇒ AddContact is disabled.
            XCTAssertFalse(
                addContact.isEnabled,
                "AddContact must be disabled while no identity is active."
            )
        } else {
            // State 3: an identity is active — the Contacts segment is
            // reachable and AddContact is live.
            XCTAssertTrue(
                addContact.isEnabled,
                "AddContact must be enabled once an identity is active."
            )
            let refresh = app.buttons
                .matching(identifier: Identifier.refreshButton).firstMatch
            XCTAssertTrue(
                refresh.waitForExistence(timeout: 5),
                "dashpay.refresh toolbar button must exist alongside the segment."
            )
        }
    }

    /// Hidden-contact recovery is gated on there being a hidden
    /// contact: the "Hidden contacts" link (`dashpay.openHidden`) must
    /// NOT appear when the active identity has none, while the Ignored
    /// entry point (`dashpay.openIgnored`) is always reachable. This is
    /// the network-free half of F16 — it proves the affordance is wired
    /// and correctly gated; the full hide → recover round-trip needs two
    /// funded, established testnet contacts and is covered manually (see
    /// the funded-wallet TODO above).
    @MainActor
    func testHiddenRecoveryAffordanceIsGated() throws {
        let app = XCUIApplication()
        app.launch()

        openDashPayTab(in: app)

        let segment = app.descendants(matching: .any)
            .matching(identifier: Identifier.segment).firstMatch
        guard segment.waitForExistence(timeout: 30) else {
            throw XCTSkip(
                "No eligible identity on this simulator — the Contacts segment "
                    + "is unreachable, so the Hidden affordance can't be asserted."
            )
        }

        // The Ignored entry point is always present once an identity is
        // active (toolbar), mirroring where Hidden recovery belongs.
        let openIgnored = app.descendants(matching: .any)
            .matching(identifier: Identifier.openIgnoredButton).firstMatch
        XCTAssertTrue(
            openIgnored.waitForExistence(timeout: 5),
            "dashpay.openIgnored must be reachable whenever an identity is active."
        )

        // With no hidden contact in local state, the recovery link is
        // gated off — it appears only once a contact is hidden.
        let openHidden = app.descendants(matching: .any)
            .matching(identifier: Identifier.openHiddenLink).firstMatch
        XCTAssertFalse(
            openHidden.exists,
            "dashpay.openHidden must be hidden until the identity has a hidden contact."
        )
    }

    /// State-1 deep link: with no wallet loaded, the "Open Wallets"
    /// CTA must switch the root tab to Wallets. Skipped (not failed)
    /// when local wallets exist, because then state 1 is unreachable.
    @MainActor
    func testNoWalletStateDeepLinksToWalletsTab() throws {
        let app = XCUIApplication()
        app.launch()

        openDashPayTab(in: app)

        let openWallets = app.buttons
            .matching(identifier: Identifier.openWalletsButton).firstMatch
        guard openWallets.waitForExistence(timeout: 30) else {
            throw XCTSkip(
                "Simulator has local wallets — the §6.4 no-wallet state is "
                    + "unreachable; covered manually / on fresh simulators."
            )
        }
        openWallets.tap()

        let walletsScreen = app.descendants(matching: .any)
            .matching(identifier: "wallets.screen").firstMatch
        XCTAssertTrue(
            walletsScreen.waitForExistence(timeout: 10)
                || app.navigationBars["Wallets"].waitForExistence(timeout: 2),
            "Open Wallets CTA must land on the Wallets tab."
        )
    }

    // MARK: - Helpers

    @MainActor
    private func openDashPayTab(in app: XCUIApplication) {
        let tabBar = app.tabBars.firstMatch
        XCTAssertTrue(
            tabBar.waitForExistence(timeout: 60),
            "Expected root tab bar to appear after app initialization."
        )

        let identifiedTab = app.tabBars.buttons
            .matching(identifier: Identifier.dashpayTab).firstMatch
        if identifiedTab.waitForExistence(timeout: 2) {
            identifiedTab.tap()
            return
        }
        let labeledTab = app.tabBars.buttons["DashPay"]
        XCTAssertTrue(
            labeledTab.waitForExistence(timeout: 5),
            "Expected DashPay tab button to exist."
        )
        labeledTab.tap()
    }

    /// Wait until any of `elements` exists, polling as one predicate
    /// expectation so the total wait stays bounded by `timeout`.
    @MainActor
    private func waitForAny(
        _ elements: [XCUIElement],
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate { object, _ in
            guard let elements = object as? [XCUIElement] else { return false }
            return elements.contains { $0.exists }
        }
        let expectation = XCTNSPredicateExpectation(
            predicate: predicate,
            object: elements
        )
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }
}
