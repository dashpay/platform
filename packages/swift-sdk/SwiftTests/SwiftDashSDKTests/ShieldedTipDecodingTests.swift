// ShieldedTipDecodingTests.swift
//
// The shielded tip wrappers marshal: these tests pin how the C values Rust
// returns become the Swift types, and that text which cannot cross the C
// boundary intact is refused before the native call.

import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class ShieldedTipDecodingTests: XCTestCase {

    private func review(
        change: ShieldedTipRecipientChangeFFI,
        unresolved: Bool = false
    ) -> ShieldedTipReviewFFI {
        var ffi = ShieldedTipReviewFFI()
        Swift.withUnsafeMutableBytes(of: &ffi.recipient_identity_id) {
            $0.copyBytes(from: [UInt8](repeating: 1, count: 32))
        }
        Swift.withUnsafeMutableBytes(of: &ffi.recipient_address) {
            $0.copyBytes(from: [UInt8](repeating: 2, count: 43))
        }
        Swift.withUnsafeMutableBytes(of: &ffi.previous_identity_id) {
            $0.copyBytes(from: [UInt8](repeating: 3, count: 32))
        }
        Swift.withUnsafeMutableBytes(of: &ffi.previous_address) {
            $0.copyBytes(from: [UInt8](repeating: 4, count: 43))
        }
        ffi.change = change
        ffi.unresolved = unresolved
        return ffi
    }

    private func balance(
        state: ShieldedIdentityBalanceStateFFI,
        account: UInt32,
        credits: UInt64
    ) -> ShieldedIdentityBalanceFFI {
        var ffi = ShieldedIdentityBalanceFFI()
        ffi.state = state
        ffi.account = account
        ffi.credits = credits
        return ffi
    }

    private let recipient = ShieldedTipRecipient(
        identityId: Data(repeating: 1, count: 32),
        address: Data(repeating: 2, count: 43))
    private let previous = ShieldedTipRecipient(
        identityId: Data(repeating: 3, count: 32),
        address: Data(repeating: 4, count: 43))

    func testShouldExposeThePreviousRecipientOnlyWhenTheRecipientChanged() throws {
        // The previous fields are set in every fixture: only `changed`
        // reads them.
        let new = try ShieldedTipReview(ffi: review(change: SHIELDED_TIP_RECIPIENT_CHANGE_FFI_NEW))
        XCTAssertEqual(new, ShieldedTipReview(recipient: recipient, change: .new, unresolved: false))

        let unchanged = try ShieldedTipReview(
            ffi: review(change: SHIELDED_TIP_RECIPIENT_CHANGE_FFI_UNCHANGED, unresolved: true))
        XCTAssertEqual(
            unchanged,
            ShieldedTipReview(recipient: recipient, change: .unchanged, unresolved: true))

        let changed = try ShieldedTipReview(
            ffi: review(change: SHIELDED_TIP_RECIPIENT_CHANGE_FFI_CHANGED))
        XCTAssertEqual(
            changed,
            ShieldedTipReview(
                recipient: recipient, change: .changed(previous: previous), unresolved: false))
    }

    func testShouldRefuseAnUnknownRecipientChange() {
        XCTAssertThrowsError(
            try ShieldedTipReview(ffi: review(change: ShieldedTipRecipientChangeFFI(rawValue: 9)))
        ) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected a deserialization error, got \(error)")
            }
        }
    }

    func testShouldDecodeEachIdentityBalanceState() throws {
        // An unverified balance carries no account, whatever the fields hold.
        XCTAssertEqual(
            try ShieldedIdentityBalance(
                ffi: balance(state: SHIELDED_IDENTITY_BALANCE_STATE_FFI_UNVERIFIED, account: 7, credits: 9)),
            .unverified)
        // Unavailable is not an available zero, and carries no credits.
        XCTAssertEqual(
            try ShieldedIdentityBalance(
                ffi: balance(
                    state: SHIELDED_IDENTITY_BALANCE_STATE_FFI_UNAVAILABLE,
                    account: 0x4000_0002, credits: 9)),
            .unavailable(account: 0x4000_0002))
        XCTAssertEqual(
            try ShieldedIdentityBalance(
                ffi: balance(
                    state: SHIELDED_IDENTITY_BALANCE_STATE_FFI_AVAILABLE,
                    account: 0x4000_0002, credits: 0)),
            .available(account: 0x4000_0002, credits: 0))
        XCTAssertEqual(
            try ShieldedIdentityBalance(
                ffi: balance(
                    state: SHIELDED_IDENTITY_BALANCE_STATE_FFI_AVAILABLE,
                    account: 0x4000_0002, credits: UInt64.max)),
            .available(account: 0x4000_0002, credits: UInt64.max))
    }

    func testShouldRefuseAnUnknownIdentityBalanceState() {
        XCTAssertThrowsError(
            try ShieldedIdentityBalance(
                ffi: balance(
                    state: ShieldedIdentityBalanceStateFFI(rawValue: 9), account: 1, credits: 1))
        ) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected a deserialization error, got \(error)")
            }
        }
    }

    /// A C string ends at its first NUL, so `"1\0x"` would reach Rust as
    /// `"1"` and parse as one DASH. It is refused before the native call.
    func testShouldRefuseTextThatCannotCrossAsACString() {
        for text in ["1\u{0}2", "\u{0}", "1\u{0}"] {
            XCTAssertThrowsError(try PlatformWalletManager.parseShieldedTipAmount(text)) { error in
                guard case PlatformWalletError.invalidParameter = error else {
                    return XCTFail("expected an invalid parameter error, got \(error)")
                }
            }
        }
        XCTAssertNoThrow(try PlatformWalletManager.requireCString("alice.dash", "username"))
        XCTAssertNoThrow(try PlatformWalletManager.requireCString("", "memo"))
        XCTAssertThrowsError(try PlatformWalletManager.requireCString("ali\u{0}ce", "username"))
    }
}
