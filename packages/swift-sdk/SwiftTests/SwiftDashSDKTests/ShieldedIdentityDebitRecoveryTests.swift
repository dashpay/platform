import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

final class ShieldedIdentityDebitRecoveryTests: XCTestCase {
    func testShouldKeepUnreadableFieldsAbsentAndPreserveScope() throws {
        var ffi = ShieldedIdentityDebitRecoveryRecordFFI()
        ffi.account_index = 9
        _ = withUnsafeMutableBytes(of: &ffi.activity_id) { $0.initializeMemory(as: UInt8.self, repeating: 7) }
        ffi.status = 1
        // Payload values are ignored when their presence bits are false.
        ffi.nonce = .max
        ffi.amount = .max
        let record = try ShieldedIdentityDebitRecoveryRecord(ffi: ffi)
        XCTAssertEqual(record.accountIndex, 9)
        XCTAssertEqual(record.activityId, Data(repeating: 7, count: 32))
        XCTAssertNil(record.identityId)
        XCTAssertNil(record.nonce)
        XCTAssertNil(record.amount)
        XCTAssertEqual(record.status, .parked)
    }

    func testShouldPreserveUnsignedValuesAndUnknownOutcome() throws {
        var ffi = ShieldedIdentityDebitRecoveryRecordFFI()
        ffi.account_index = .max
        ffi.has_identity_id = true
        ffi.has_nonce = true
        ffi.has_amount = true
        ffi.nonce = .max
        ffi.amount = .max
        ffi.status = 2
        let record = try ShieldedIdentityDebitRecoveryRecord(ffi: ffi)
        XCTAssertEqual(record.accountIndex, .max)
        XCTAssertEqual(record.identityId, Data(repeating: 0, count: 32))
        XCTAssertEqual(record.nonce, .max)
        XCTAssertEqual(record.amount, .max)
        XCTAssertEqual(record.status, .unknown)
    }

    func testShouldRejectUnrecognizedRecoveryStatus() {
        var ffi = ShieldedIdentityDebitRecoveryRecordFFI()
        ffi.status = 255
        XCTAssertThrowsError(try ShieldedIdentityDebitRecoveryRecord(ffi: ffi))
    }

    @MainActor
    func testShouldRejectUnconfiguredManagerBeforeCallingRecoveryFFI() async {
        let manager = PlatformWalletManager()
        do {
            _ = try await manager.shieldedIdentityDebitRecoveryRecords(walletId: Data(repeating: 7, count: 32))
            XCTFail("an unconfigured manager cannot list recovery records")
        } catch {
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("unexpected error: \(error)")
            }
        }
        do {
            try await manager.abandonShieldedIdentityDebit(
                walletId: Data(repeating: 7, count: 32), accountIndex: 0,
                activityId: Data(repeating: 8, count: 32), acknowledgePossibleExecution: true
            )
            XCTFail("an unconfigured manager cannot abandon a record")
        } catch {
            guard case PlatformWalletError.invalidHandle = error else {
                return XCTFail("unexpected error: \(error)")
            }
        }
    }
}
