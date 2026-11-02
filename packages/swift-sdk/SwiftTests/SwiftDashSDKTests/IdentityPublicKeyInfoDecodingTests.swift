import Foundation
import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Hermetic tests for the decoding of the flat `IdentityPublicKeyFFI` array
/// that `managed_identity_get_public_keys` and
/// `platform_wallet_fetch_identity_public_keys` hand to Swift. The entries
/// are built in Swift, so only the copy (`decode`) and the empty sentinel of
/// `keys(from:count:)` run; no buffer is handed to Rust to free.
final class IdentityPublicKeyInfoDecodingTests: XCTestCase {
    private typealias Info = ManagedIdentity.IdentityPublicKeyInfo

    private func entry(
        keyId: UInt32,
        purpose: UInt8 = KeyPurpose.authentication.rawValue,
        data: UnsafeMutablePointer<UInt8>?,
        dataLen: Int,
        disabledAt: UInt64? = nil,
        totalBudget: UInt64? = nil,
        expiresAt: UInt64? = nil
    ) -> IdentityPublicKeyFFI {
        IdentityPublicKeyFFI(
            key_id: keyId,
            purpose: purpose,
            security_level: SecurityLevel.high.rawValue,
            key_type: KeyType.ecdsaSecp256k1.rawValue,
            read_only: true,
            disabled_at_is_some: disabledAt != nil,
            disabled_at: disabledAt ?? 0,
            data_ptr: data,
            data_len: UInt(dataLen),
            total_budget_is_some: totalBudget != nil,
            total_budget: totalBudget ?? 0,
            expires_at_is_some: expiresAt != nil,
            expires_at: expiresAt ?? 0
        )
    }

    func testShouldDecodeEveryFieldOfEachKey() throws {
        var bytes: [UInt8] = Array(repeating: 2, count: 33)
        let keys = try bytes.withUnsafeMutableBufferPointer { blob -> [Info] in
            let entries = [
                entry(keyId: 0, data: blob.baseAddress, dataLen: blob.count),
                entry(
                    keyId: 1,
                    data: blob.baseAddress,
                    dataLen: blob.count,
                    disabledAt: 1_700_000_000_000
                ),
                entry(
                    keyId: 2,
                    data: nil,
                    dataLen: 0,
                    totalBudget: 5_000_000,
                    expiresAt: 1_800_000_000_000
                ),
            ]
            return try entries.withUnsafeBufferPointer { try Info.decode($0) }
        }

        XCTAssertEqual(keys.map(\.keyId), [0, 1, 2])
        for key in keys {
            XCTAssertEqual(key.purpose, .authentication)
            XCTAssertEqual(key.securityLevel, .high)
            XCTAssertEqual(key.keyType, .ecdsaSecp256k1)
            XCTAssertTrue(key.readOnly)
        }
        XCTAssertEqual(keys[0].data, Data(bytes))
        XCTAssertNil(keys[0].disabledAt)
        XCTAssertNil(keys[0].totalBudget)
        XCTAssertNil(keys[0].expiresAt)
        XCTAssertEqual(keys[1].disabledAt, 1_700_000_000_000)
        XCTAssertEqual(keys[2].data, Data())
        XCTAssertEqual(keys[2].totalBudget, 5_000_000)
        XCTAssertEqual(keys[2].expiresAt, 1_800_000_000_000)
    }

    func testShouldRefuseAnUnknownPurpose() {
        let entries = [entry(keyId: 0, purpose: 200, data: nil, dataLen: 0)]
        XCTAssertThrowsError(
            try entries.withUnsafeBufferPointer { try Info.decode($0) }
        ) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("Unexpected error: \(error)")
            }
        }
    }

    func testShouldDecodeTheEmptySentinelToNoKeys() throws {
        XCTAssertEqual(try Info.keys(from: nil, count: 0).count, 0)
        XCTAssertEqual(try Info.keys(from: nil, count: 3).count, 0)
    }
}
