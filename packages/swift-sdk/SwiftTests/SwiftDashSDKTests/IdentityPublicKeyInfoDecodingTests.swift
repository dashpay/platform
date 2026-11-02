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

    func testShouldExposeTheBlockTimeAsADate() {
        let fetched = FetchedIdentityPublicKeys(keys: nil, blockTimeMs: 1_750_000_000_123)
        XCTAssertNil(fetched.keys)
        XCTAssertEqual(fetched.blockTime.timeIntervalSince1970, 1_750_000_000.123, accuracy: 0.0005)
    }

    func testShouldCarryEachKeysExpiryAtTheProofTime() throws {
        let entries = [
            FetchedIdentityPublicKeyFFI(
                key: entry(keyId: 0, data: nil, dataLen: 0),
                expired_at_proof_time: false
            ),
            FetchedIdentityPublicKeyFFI(
                key: entry(keyId: 1, data: nil, dataLen: 0, expiresAt: 1_000),
                expired_at_proof_time: true
            ),
        ]
        let keys = try entries.withUnsafeBufferPointer {
            try FetchedIdentityPublicKey.decode($0)
        }

        XCTAssertEqual(keys.map(\.key.keyId), [0, 1])
        XCTAssertEqual(keys.map(\.expiredAtProofTime), [false, true])
        XCTAssertEqual(keys[1].key.expiresAt, 1_000)
    }

    func testShouldDecodeTheFetchedEmptySentinelToNoKeys() throws {
        XCTAssertEqual(try FetchedIdentityPublicKey.keys(from: nil, count: 0).count, 0)
    }

    /// What the FFI writes for an identity Platform does not have:
    /// `found == false` with `(null, 0)` keys. The keys are `nil`, and the
    /// proof's block time is kept.
    func testShouldMapNotFoundToNilKeysWithTheBlockTime() throws {
        let fetched = try FetchedIdentityPublicKeys(
            found: false,
            blockTimeMs: 1_741_850_423_685,
            keys: nil,
            count: 0
        )

        XCTAssertNil(fetched.keys)
        XCTAssertEqual(fetched.blockTimeMs, 1_741_850_423_685)
    }

    /// A found identity always holds a key, so `found` with no keys is
    /// refused rather than read as an identity without keys.
    func testShouldRefuseAFoundIdentityWithoutKeys() {
        XCTAssertThrowsError(
            try FetchedIdentityPublicKeys(found: true, blockTimeMs: 1, keys: nil, count: 0)
        ) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("Unexpected error: \(error)")
            }
        }
    }
}
