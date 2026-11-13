import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Swift-side guards of `shieldedFreshAddress` / `isOwnShieldedAddress`:
/// an unconfigured manager and a wallet id of the wrong length throw before
/// any FFI call, the same way `shieldedDefaultAddress` does.
@MainActor
final class ShieldedFreshAddressTests: XCTestCase {

    private static let walletId = Data(repeating: 7, count: 32)
    private static let raw43 = Data(repeating: 1, count: 43)

    /// A manager configured with a fake handle and no-op teardown, so the
    /// wallet-id guard is reached without a native manager behind it.
    private func configuredManager() throws -> PlatformWalletManager {
        let noOp: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let manager = PlatformWalletManager()
        try manager.configureForTesting(
            handle: 17,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: noOp,
                platformAddressSyncStop: noOp,
                shieldedSyncStop: noOp,
                dashPaySyncStop: noOp,
                dpnsSyncStop: noOp,
                destroy: noOp
            )
        )
        return manager
    }

    func testFreshAddressThrowsInvalidHandleWhenNotConfigured() {
        let manager = PlatformWalletManager()
        XCTAssertThrowsError(try manager.shieldedFreshAddress(walletId: Self.walletId)) { error in
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("Expected invalidHandle, got \(error)")
            }
        }
    }

    func testIsOwnThrowsInvalidHandleWhenNotConfigured() {
        let manager = PlatformWalletManager()
        XCTAssertThrowsError(
            try manager.isOwnShieldedAddress(walletId: Self.walletId, raw43: Self.raw43)
        ) { error in
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("Expected invalidHandle, got \(error)")
            }
        }
    }

    func testFreshAddressRejectsShortWalletId() throws {
        let manager = try configuredManager()
        XCTAssertThrowsError(
            try manager.shieldedFreshAddress(walletId: Data(repeating: 7, count: 31))
        ) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("Expected invalidParameter, got \(error)")
            }
        }
    }

    func testIsOwnRejectsShortWalletId() throws {
        let manager = try configuredManager()
        XCTAssertThrowsError(
            try manager.isOwnShieldedAddress(
                walletId: Data(repeating: 7, count: 31),
                raw43: Self.raw43
            )
        ) { error in
            guard case PlatformWalletError.invalidParameter = error else {
                return XCTFail("Expected invalidParameter, got \(error)")
            }
        }
    }
}
