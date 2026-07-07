import XCTest
import SwiftData
@testable import SwiftDashSDK

final class SpvStartErrorIntegrationTests: IntegrationTestCase {
    func testStartSpvSurfacesLockedDataDir() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)

        let sdk = env.sdk
        let container = env.modelContainer
        let other = try await MainActor.run {
            try PlatformWalletManager(sdk: sdk, modelContainer: container)
        }

        do {
            try await other.startSpv(config: env.spvConfig)
            XCTFail("startSpv should throw: data dir is locked by the first manager")
        } catch {
            let message = (error as? LocalizedError)?.errorDescription ?? "\(error)"
            XCTAssertTrue(
                message.localizedCaseInsensitiveContains("in use")
                    || message.localizedCaseInsensitiveContains("lock"),
                "expected a data-dir lock error, got: \(message)"
            )
        }
    }
}
