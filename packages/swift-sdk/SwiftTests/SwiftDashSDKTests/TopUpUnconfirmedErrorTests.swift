import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

final class TopUpUnconfirmedErrorTests: XCTestCase {
    func testTopUpUnconfirmedResultPreservesExistingTypedError() throws {
        XCTAssertEqual(PlatformWalletResultCode.errorTransactionBroadcastUnconfirmed.rawValue, 20)
        for detail in [
            "Identity top-up may have executed; reconcile before submitting again",
            "wait timeout",
            "networkError: connection lost"
        ] {
            let ffi = PlatformWalletFFIResult(
                code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_UNCONFIRMED,
                message: strdup(detail)
            )
            XCTAssertThrowsError(try ffi.check()) { error in
                guard case PlatformWalletError.transactionBroadcastUnconfirmed(let message) = error else {
                    return XCTFail("Expected typed unconfirmed result, got \(error)")
                }
                XCTAssertEqual(message, detail)
                XCTAssertEqual(error.localizedDescription, detail)
                let formatted = ErrorFormatter.userFriendlyMessage(from: error)
                let displayed = ErrorCategorizer.toUserFacingError(error)
                XCTAssertTrue(formatted.contains("before submitting again"))
                XCTAssertTrue(formatted.contains(detail))
                XCTAssertEqual(displayed.message, formatted)
                XCTAssertFalse(displayed.isRetryable)
                XCTAssertNil(displayed.recoverySuggestion)
            }
        }
        XCTAssertTrue(ErrorCategorizer.toUserFacingError(SDKError.timeout("timeout")).isRetryable)
    }
}
