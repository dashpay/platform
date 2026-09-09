import XCTest
@testable import SwiftExampleApp

/// Coverage for `engineBindOtherWallets` — the pure iteration seam
/// behind the multi-wallet shielded engine-bind in
/// `SwiftExampleAppApp.rebindWalletScopedServices()`.
///
/// The app engine-binds EVERY loaded wallet into the shared
/// network-scoped shielded coordinator so a single sync pass
/// trial-decrypts against the union of all wallets' viewing keys
/// (SH-14/15/16 cross-wallet flows). The mirror wallet (`firstWallet`)
/// is bound separately via `ShieldedService.bind(...)`, so this seam
/// binds every OTHER wallet.
///
/// The real `ShieldedService.bindEngine` calls into FFI and needs a
/// configured `PlatformWalletManager`, so the loop logic — "visit every
/// non-mirror id" and "one id's failure doesn't stop the rest" — is
/// factored into this pure helper and tested with a recording closure.
///
/// `@MainActor` because `engineBindOtherWallets` is (its production
/// `bindEngine` closure touches `@MainActor` `ShieldedService` state).
@MainActor
final class ShieldedEngineBindPlanTests: XCTestCase {

    private func id(_ byte: UInt8) -> Data {
        Data(repeating: byte, count: 32)
    }

    /// Every non-mirror wallet is engine-bound exactly once; the mirror
    /// wallet is skipped (it's bound separately via the UI-mirror path).
    func testBindsEveryWalletExceptMirror() {
        let mirror = id(0x01)
        let others = [id(0x02), id(0x03), id(0x04)]
        let all = [mirror] + others

        var bound: [Data] = []
        engineBindOtherWallets(allWalletIds: all, mirrorWalletId: mirror) { walletId in
            bound.append(walletId)
        }

        XCTAssertEqual(
            Set(bound),
            Set(others),
            "every non-mirror wallet must be engine-bound"
        )
        XCTAssertFalse(
            bound.contains(mirror),
            "the mirror wallet is bound separately and must be skipped here"
        )
        XCTAssertEqual(
            bound.count,
            others.count,
            "each non-mirror wallet must be bound exactly once"
        )
    }

    /// A throwing bind for ONE wallet must not stop the others — the
    /// production requirement that one wallet's missing mnemonic /
    /// declined resolver can't dark every other wallet's shielded state.
    func testOneFailureDoesNotStopTheRest() {
        let mirror = id(0x01)
        let failing = id(0x03)
        let all = [mirror, id(0x02), failing, id(0x04)]

        struct BindError: Error {}
        var attempted: [Data] = []
        engineBindOtherWallets(allWalletIds: all, mirrorWalletId: mirror) { walletId in
            attempted.append(walletId)
            if walletId == failing { throw BindError() }
        }

        // Every non-mirror wallet is still ATTEMPTED even though the
        // middle one threw.
        XCTAssertEqual(
            Set(attempted),
            Set([id(0x02), failing, id(0x04)]),
            "a throwing bind for one wallet must not skip the remaining wallets"
        )
    }

    /// A single-wallet device (only the mirror loaded) binds nothing
    /// extra — the mirror's own engine registration is the UI-mirror
    /// path's job, not this seam's.
    func testSingleMirrorWalletBindsNothing() {
        let mirror = id(0x01)

        var bound: [Data] = []
        engineBindOtherWallets(allWalletIds: [mirror], mirrorWalletId: mirror) { walletId in
            bound.append(walletId)
        }

        XCTAssertTrue(
            bound.isEmpty,
            "with only the mirror wallet loaded there is nothing else to engine-bind"
        )
    }

    /// The mirror is skipped even when it is not the first element of
    /// the sequence — `Dictionary.Keys` has no defined order, so the
    /// skip must be by identity, not position.
    func testMirrorSkippedRegardlessOfPosition() {
        let mirror = id(0x05)
        let all = [id(0x02), id(0x05), id(0x08)] // mirror in the middle

        var bound: [Data] = []
        engineBindOtherWallets(allWalletIds: all, mirrorWalletId: mirror) { walletId in
            bound.append(walletId)
        }

        XCTAssertEqual(Set(bound), Set([id(0x02), id(0x08)]))
        XCTAssertFalse(bound.contains(mirror))
    }
}
