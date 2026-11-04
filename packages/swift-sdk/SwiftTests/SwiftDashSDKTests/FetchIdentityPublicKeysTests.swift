import Foundation
import XCTest
@testable import SwiftDashSDK

/// ``SDK/fetchIdentityPublicKeys(identityId:)`` end to end against the FFI
/// mock: a recorded proved answer decoded into ``FetchedIdentityPublicKeys``,
/// a failed request, and an invalid id. The not-found mapping, which no
/// recorded answer covers, is tested on the FFI out-params in
/// `IdentityPublicKeyInfoDecodingTests`.
final class FetchIdentityPublicKeysTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        SDK.initialize()
    }

    /// The block time in the metadata of the recorded
    /// `test_identity_public_keys_all_read` response.
    private static let recordedBlockTimeMs: TimestampMillis = 1_741_850_423_685

    /// The mock replays rs-sdk's recorded `test_identity_public_keys_all_read`
    /// response (the same all-keys request), the SDK verifies its proof
    /// against the recorded quorum key, and the keys come back typed, with
    /// the proof's block time. Main actor, as in `SDKMethodTests`: loading the
    /// recorded vectors needs more stack than a cooperative-pool thread has.
    @MainActor
    func testShouldReturnTheRecordedKeysWithTheProofBlockTime() async throws {
        let vectors = rsSdkVectors("test_identity_public_keys_all_read")
        guard FileManager.default.fileExists(atPath: vectors) else {
            XCTFail("missing rs-sdk offline vectors at \(vectors)")
            return
        }
        let sdk = try SDK(mockVectorsDirectory: vectors)

        // The vectors record identity [1; 32], rs-sdk's IDENTITY_ID_1.
        let fetched = try await sdk.fetchIdentityPublicKeys(
            identityId: Data(repeating: 1, count: 32)
        )

        XCTAssertEqual(fetched.blockTimeMs, Self.recordedBlockTimeMs)
        let keys = try XCTUnwrap(fetched.keys, "the recorded identity exists")

        // The recorded identity holds three authentication keys: 0 (master),
        // 1 (critical) and 2 (high), none disabled and none with an expiry,
        // so none had expired at the proof's time.
        XCTAssertEqual(keys.map(\.key.keyId), [0, 1, 2])
        XCTAssertEqual(keys.map(\.key.securityLevel), [.master, .critical, .high])
        for key in keys {
            XCTAssertEqual(key.key.purpose, .authentication)
            XCTAssertEqual(key.key.keyType, .ecdsaSecp256k1)
            XCTAssertEqual(key.key.data.count, 33, "a compressed secp256k1 key")
            XCTAssertNil(key.key.disabledAt)
            XCTAssertNil(key.key.expiresAt)
            XCTAssertFalse(key.expiredAtProofTime)
        }
    }

    func testShouldSurfaceAFailedFetchAsSDKError() async throws {
        // No recorded responses: the request fails.
        let sdk = try SDK(mockVectorsDirectory: nil)

        do {
            _ = try await sdk.fetchIdentityPublicKeys(identityId: Data(repeating: 6, count: 32))
            XCTFail("the fetch should fail without recorded responses")
        } catch is SDKError {
        } catch {
            XCTFail("expected an SDKError, got \(error)")
        }
    }

    func testShouldRefuseAnIdThatIsNot32Bytes() async throws {
        let sdk = try SDK(mockVectorsDirectory: nil)

        do {
            _ = try await sdk.fetchIdentityPublicKeys(identityId: Data(repeating: 6, count: 31))
            XCTFail("a 31-byte id should be refused")
        } catch SDKError.invalidParameter {
        } catch {
            XCTFail("expected SDKError.invalidParameter, got \(error)")
        }
    }
}
