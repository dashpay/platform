import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

final class TopUpUnconfirmedErrorTests: XCTestCase {
    func testTopUpUnconfirmedResultPreservesExistingTypedError() throws {
        XCTAssertEqual(PlatformWalletResultCode.errorTransactionBroadcastUnconfirmed.rawValue, 20)
        for detail in [
            "State transition may have executed; reconcile before submitting again",
            "wait timeout",
            "networkError: connection lost"
        ] {
            // Rust classes every unconfirmed broadcast as maybe sent.
            let ffi = PlatformWalletFFIResult(
                code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_UNCONFIRMED,
                message: strdup(detail),
                consensus_code: 0,
                consensus_kind: PLATFORM_WALLET_FFI_CONSENSUS_ERROR_KIND_NONE,
                payment_delivery: PLATFORM_WALLET_FFI_PAYMENT_DELIVERY_MAYBE_SENT,
                held_asset_lock: OutPointFFI(),
                retryable: false
            )
            XCTAssertThrowsError(try ffi.check()) { error in
                guard case PlatformWalletError.transactionBroadcastUnconfirmed(let message, _) = error else {
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
