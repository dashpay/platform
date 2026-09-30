import XCTest
import SwiftDashSDK

/// Live read of a known testnet identity through the default testnet SDK
/// (trusted context provider, seed DAPI nodes). It depends on testnet being
/// up and producing blocks, so it is opt-in via `RUN_TESTNET_TESTS=1` and
/// does not need the local devnet. The hermetic counterpart is
/// `SDKMethodTests.testSimpleIdentityFetch` in SwiftDashSDKTests.
final class TestnetIdentityFetchTests: XCTestCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        try XCTSkipUnless(
            ProcessInfo.processInfo.environment["RUN_TESTNET_TESTS"] == "1",
            "Testnet tests skipped: set RUN_TESTNET_TESTS=1 to enable"
        )
    }

    @MainActor
    func testFetchKnownTestnetIdentity() async throws {
        SDK.initialize()
        let sdk = try SDK(network: .testnet)

        let identityId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
        let identity = try await sdk.identityGet(identityId: identityId)

        XCTAssertEqual(identity["id"] as? String, identityId)
    }
}
