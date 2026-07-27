//
//  KeyDisableGateTests.swift
//  SwiftExampleAppTests
//
//  Unit coverage for `KeyDisableGate.evaluate` — the pure, single
//  source of truth for whether a Disable Key submit is attempted
//  (used in KeyDetailView, KeysListView, and the pre-submit re-check).
//

import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class KeyDisableGateTests: XCTestCase {

    // MARK: - Fixtures

    /// Build an `IdentityPublicKey` test value with the fields the gate
    /// actually reads (`id`, `purpose`, `securityLevel`, `disabledAt`).
    /// `keyType`/`data`/`readOnly` are irrelevant to the gate, so they
    /// get trivial defaults.
    private func makeKey(
        id: KeyID,
        purpose: KeyPurpose,
        securityLevel: SecurityLevel = .high,
        disabledAt: TimestampMillis? = nil
    ) -> IdentityPublicKey {
        IdentityPublicKey(
            id: id,
            purpose: purpose,
            securityLevel: securityLevel,
            contractBounds: nil,
            keyType: .ecdsaSecp256k1,
            readOnly: false,
            data: BinaryData(),
            disabledAt: disabledAt
        )
    }

    /// A fixed, non-nil disabled timestamp for "already disabled" keys.
    private let disabledTimestamp: TimestampMillis = 1_700_000_000_000

    // MARK: - 1. Already disabled

    func testAlreadyDisabledTarget_returnsAlreadyDisabled() {
        let target = makeKey(id: 5, purpose: .authentication, disabledAt: disabledTimestamp)
        // Include a second enabled auth key so the result can only be
        // `.alreadyDisabled` (and not, e.g., a last-auth refusal).
        let other = makeKey(id: 6, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target, other])

        XCTAssertEqual(result, .alreadyDisabled)
    }

    // MARK: - 2. Master key

    func testMasterKey_isForbidden() {
        // A master key is also an authentication key in practice; the
        // master check runs before the auth check, so this must be
        // `.forbidden` regardless of how many other auth keys exist.
        let target = makeKey(id: 0, purpose: .authentication, securityLevel: .master)
        let other = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target, other])

        guard case .forbidden = result else {
            return XCTFail("expected .forbidden for a master key, got \(result)")
        }
    }

    // MARK: - 3. Last authentication key

    func testLastEnabledAuthenticationKey_isForbidden() {
        let target = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target])

        guard case .forbidden = result else {
            return XCTFail("expected .forbidden for the only auth key, got \(result)")
        }
    }

    func testAuthenticationKey_withSecondEnabledAuth_isAllowed() {
        let target = makeKey(id: 1, purpose: .authentication)
        let second = makeKey(id: 2, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target, second])

        XCTAssertEqual(result, .allowed)
    }

    // MARK: - 4. Last transfer key

    func testLastEnabledTransferKey_isForbidden() {
        // Pair with an auth key so the auth rail can't trip first and
        // the transfer rail is the one under test.
        let target = makeKey(id: 3, purpose: .transfer)
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target, auth])

        guard case .forbidden = result else {
            return XCTFail("expected .forbidden for the only transfer key, got \(result)")
        }
    }

    func testTransferKey_withSecondEnabledTransfer_isAllowed() {
        let target = makeKey(id: 3, purpose: .transfer)
        let secondTransfer = makeKey(id: 4, purpose: .transfer)
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, secondTransfer, auth]
        )

        XCTAssertEqual(result, .allowed)
    }

    // MARK: - 5. Normal non-last key

    func testNormalNonLastKey_isAllowed() {
        // A non-auth / non-transfer purpose with no special rails.
        let target = makeKey(id: 7, purpose: .encryption)
        // Keep an auth key around so the identity stays signable; this
        // shouldn't matter for an encryption-key disable, but it makes
        // the scenario realistic.
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(target: target, allKeys: [target, auth])

        XCTAssertEqual(result, .allowed)
    }

    // MARK: - 6. enabledCount edge cases (exercised through evaluate)

    /// Keys of a *different* purpose must not count toward the auth
    /// limit: disabling the only auth key is forbidden even when other
    /// purposes have many enabled keys.
    func testOtherPurposeKeysDoNotCountTowardAuthLimit() {
        let target = makeKey(id: 1, purpose: .authentication)
        let encryption = makeKey(id: 2, purpose: .encryption)
        let transferA = makeKey(id: 3, purpose: .transfer)
        let transferB = makeKey(id: 4, purpose: .transfer)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, encryption, transferA, transferB]
        )

        guard case .forbidden = result else {
            return XCTFail(
                "expected .forbidden — other-purpose keys must not satisfy the auth requirement, got \(result)"
            )
        }
    }

    /// Already-disabled keys of the same purpose are excluded from the
    /// count: two auth keys where one is already disabled still forbids
    /// disabling the remaining enabled one.
    func testAlreadyDisabledSamePurposeKeyExcludedFromCount() {
        let target = makeKey(id: 1, purpose: .authentication)
        let disabledAuth = makeKey(
            id: 2,
            purpose: .authentication,
            disabledAt: disabledTimestamp
        )

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, disabledAuth]
        )

        guard case .forbidden = result else {
            return XCTFail(
                "expected .forbidden — a disabled sibling auth key is not an enabled fallback, got \(result)"
            )
        }
    }

    /// Symmetric transfer-side check: a disabled sibling transfer key
    /// doesn't keep the last enabled transfer key disableable.
    func testAlreadyDisabledSiblingTransferKeyExcludedFromCount() {
        let target = makeKey(id: 3, purpose: .transfer)
        let disabledTransfer = makeKey(
            id: 4,
            purpose: .transfer,
            disabledAt: disabledTimestamp
        )
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, disabledTransfer, auth]
        )

        guard case .forbidden = result else {
            return XCTFail(
                "expected .forbidden — a disabled sibling transfer key is not an enabled fallback, got \(result)"
            )
        }
    }
}
