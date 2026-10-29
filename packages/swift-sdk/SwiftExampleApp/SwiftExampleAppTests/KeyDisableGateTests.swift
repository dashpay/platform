//
//  KeyDisableGateTests.swift
//  SwiftExampleAppTests
//
//  Unit coverage for `KeyDisableGate.evaluate` — what a Disable Key
//  action shows for the wallet's verdict (used in KeyDetailView,
//  KeysListView, and the pre-submit re-check). The verdict itself comes
//  from Rust (DIP-11 and the last transfer key) and is covered there.
//

import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

final class KeyDisableGateTests: XCTestCase {

    // MARK: - Fixtures

    /// Build an `IdentityPublicKey` test value with the fields the gate
    /// actually reads (`disabledAt`). The rest get trivial defaults.
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

        // Whatever Rust says, a disabled key has nothing left to do.
        let result = KeyDisableGate.evaluate(
            target: target,
            ruleVerdict: .refused(.lastAuthenticationKey(keyId: 5, securityLevel: .high))
        )

        XCTAssertEqual(result, .alreadyDisabled)
    }

    // MARK: - 2. The wallet's verdict

    func testAllowedVerdict_isAllowed() {
        // Which keys may go is Rust's call: with another MASTER key, even a
        // MASTER key may be disabled.
        let target = makeKey(id: 0, purpose: .authentication, securityLevel: .master)

        XCTAssertEqual(KeyDisableGate.evaluate(target: target, ruleVerdict: .allowed), .allowed)
    }

    func testLastAuthenticationKeyRefusal_isForbiddenNamingTheLevel() {
        let target = makeKey(id: 1, purpose: .authentication, securityLevel: .critical)

        let result = KeyDisableGate.evaluate(
            target: target,
            ruleVerdict: .refused(.lastAuthenticationKey(keyId: 1, securityLevel: .critical))
        )

        guard case .forbidden(let reason) = result else {
            return XCTFail("expected .forbidden for a DIP-11 refusal, got \(result)")
        }
        XCTAssertTrue(reason.contains(SecurityLevel.critical.name), reason)
    }

    func testLastEnabledKeyRefusal_isForbiddenNamingThePurpose() {
        let target = makeKey(id: 3, purpose: .transfer)

        let result = KeyDisableGate.evaluate(
            target: target,
            ruleVerdict: .refused(.lastEnabledKey(keyId: 3, purpose: .transfer))
        )

        guard case .forbidden(let reason) = result else {
            return XCTFail("expected .forbidden for the last transfer key, got \(result)")
        }
        XCTAssertTrue(reason.contains("transfer"), reason)
    }

    func testUnavailableVerdict_isForbiddenWithItsReason() {
        let target = makeKey(id: 1, purpose: .authentication)

        let result = KeyDisableGate.evaluate(
            target: target,
            ruleVerdict: .unavailable(reason: "Wallet not loaded")
        )

        XCTAssertEqual(result, .forbidden(reason: "Wallet not loaded"))
    }
}
