//
//  KeyDisableGateTests.swift
//  SwiftExampleAppTests
//
//  Unit coverage for `KeyDisableGate.evaluate` — the single place that
//  decides whether a Disable Key submit is attempted (used in
//  KeyDetailView, KeysListView, and the pre-submit re-check). The DIP-11
//  verdict itself comes from Rust and is covered there; these tests pin
//  how the gate renders it and the last-transfer-key guard.
//

import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class KeyDisableGateTests: XCTestCase {

    // MARK: - Fixtures

    /// Build an `IdentityPublicKey` test value with the fields the gate
    /// actually reads (`id`, `purpose`, `disabledAt`). The rest are
    /// irrelevant to the gate, so they get trivial defaults.
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

    private func refusal(_ keyId: KeyID, _ level: SecurityLevel) -> KeyDisableGate.RuleVerdict {
        .refused(ManagedPlatformWallet.KeyDisableRefusal(keyId: keyId, securityLevel: level))
    }

    // MARK: - 1. Already disabled

    func testAlreadyDisabledTarget_returnsAlreadyDisabled() {
        let target = makeKey(id: 5, purpose: .authentication, disabledAt: disabledTimestamp)

        // Whatever Rust says, a disabled key has nothing left to do.
        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target],
            ruleVerdict: refusal(5, .high)
        )

        XCTAssertEqual(result, .alreadyDisabled)
    }

    // MARK: - 2. DIP-11 verdict from Rust

    func testRefusedVerdict_isForbiddenNamingTheLevel() {
        let target = makeKey(id: 1, purpose: .authentication, securityLevel: .critical)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target],
            ruleVerdict: refusal(1, .critical)
        )

        guard case .forbidden(let reason) = result else {
            return XCTFail("expected .forbidden for a refused verdict, got \(result)")
        }
        XCTAssertTrue(reason.contains(SecurityLevel.critical.name), reason)
    }

    func testUnavailableVerdict_isForbiddenWithItsReason() {
        let target = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target],
            ruleVerdict: .unavailable(reason: "Wallet not loaded")
        )

        XCTAssertEqual(result, .forbidden(reason: "Wallet not loaded"))
    }

    func testAllowedVerdict_isAllowedEvenForAMasterKey() {
        // Which authentication keys may go is Rust's call: with another
        // MASTER key, DIP-11 lets this one be disabled.
        let target = makeKey(id: 0, purpose: .authentication, securityLevel: .master)
        let other = makeKey(id: 1, purpose: .authentication, securityLevel: .master)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, other],
            ruleVerdict: .allowed
        )

        XCTAssertEqual(result, .allowed)
    }

    // MARK: - 3. Last transfer key

    func testLastEnabledTransferKey_isForbidden() {
        let target = makeKey(id: 3, purpose: .transfer)
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, auth],
            ruleVerdict: .allowed
        )

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
            allKeys: [target, secondTransfer, auth],
            ruleVerdict: .allowed
        )

        XCTAssertEqual(result, .allowed)
    }

    /// A disabled sibling transfer key doesn't keep the last enabled
    /// transfer key disableable.
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
            allKeys: [target, disabledTransfer, auth],
            ruleVerdict: .allowed
        )

        guard case .forbidden = result else {
            return XCTFail(
                "expected .forbidden — a disabled sibling transfer key is not an enabled fallback, got \(result)"
            )
        }
    }

    // MARK: - 4. Normal key

    func testNormalKey_isAllowed() {
        let target = makeKey(id: 7, purpose: .encryption)
        let auth = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            allKeys: [target, auth],
            ruleVerdict: .allowed
        )

        XCTAssertEqual(result, .allowed)
    }
}
