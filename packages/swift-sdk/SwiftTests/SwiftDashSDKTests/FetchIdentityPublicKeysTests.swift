import Foundation
import XCTest
@testable import SwiftDashSDK

/// ``SDK/fetchIdentityPublicKeys(identityId:)`` against the FFI mock with no
/// recorded responses: the request fails, and the failure surfaces as an
/// `SDKError`, the method's one error type.
final class FetchIdentityPublicKeysTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        SDK.initialize()
    }

    func testShouldSurfaceAFailedFetchAsSDKError() async throws {
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
