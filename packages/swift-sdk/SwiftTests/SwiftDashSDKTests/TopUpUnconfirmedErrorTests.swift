import DashSDKFFI
import Foundation
import XCTest
@testable import SwiftDashSDK

final class TopUpUnconfirmedErrorTests: XCTestCase {
    func testTopUpUnconfirmedResultPreservesExistingTypedError() throws {
        let detail = "Identity top-up may have executed; reconcile before submitting again"
        let ffi = PlatformWalletFFIResult(
            code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_TRANSACTION_BROADCAST_UNCONFIRMED,
            message: strdup(detail)
        )
        XCTAssertEqual(PlatformWalletResultCode.errorTransactionBroadcastUnconfirmed.rawValue, 20)
        XCTAssertThrowsError(try ffi.check()) { error in
            guard case PlatformWalletError.transactionBroadcastUnconfirmed(let message) = error else {
                return XCTFail("Expected typed unconfirmed result, got \(error)")
            }
            XCTAssertEqual(message, detail)
            XCTAssertEqual(error.localizedDescription, detail)
        }
    }
}
