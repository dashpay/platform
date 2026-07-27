import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

open class IntegrationTestCase: XCTestCase {
    private(set) var env: IntegrationTestEnv!
    nonisolated(unsafe) private static var bootstrapResult: Result<IntegrationTestEnv, Error>?

    open override func setUp() async throws {
        try await super.setUp()
        try skipIfDisabled()
        env = try await Self.sharedEnv()
        try await env.assertCleanState()
    }

    open override func tearDown() async throws {
        if let env {
            try? await env.resetState()
        }

        try await super.tearDown()
    }

    private func skipIfDisabled() throws {
        let enabled = ProcessInfo.processInfo.environment["RUN_INTEGRATION_TESTS"] == "1"
        try XCTSkipUnless(
            enabled,
            "Integration tests skipped — set RUN_INTEGRATION_TESTS=1 to enable"
        )
    }

    private static func sharedEnv() async throws -> IntegrationTestEnv {
        if let cached = bootstrapResult {
            return try cached.get()
        }

        do {
            let env = try await IntegrationTestEnv.bootstrap()
            bootstrapResult = .success(env)

            await MainActor.run {
                SpvSuiteCleanupObserver.shared.env = env
                XCTestObservationCenter.shared.addTestObserver(SpvSuiteCleanupObserver.shared)
            }

            return env
        } catch {
            bootstrapResult = .failure(error)
            throw error
        }
    }

    /// All txids currently in `PersistentTransaction`
    func readTxids() async throws -> Set<Data> {
        let container = env.modelContainer

        return try await MainActor.run {
            let ctx = ModelContext(container)
            return Set(try ctx.fetch(FetchDescriptor<PersistentTransaction>()).map { $0.txid })
        }
    }

    /// Polls `readTxids()` until a txid not in `before` shows up,
    /// then returns it. Returns nil on timeout (60s).
    func waitForNewTxid(notIn before: Set<Data>) async throws -> Data? {
        let deadline = Date().addingTimeInterval(60)

        while Date() < deadline {
            let after = try await readTxids()

            if let found = after.subtracting(before).first {
                return found
            }

            try await Task.sleep(nanoseconds: 50_000_000)
        }

        return nil
    }
}

private final class SpvSuiteCleanupObserver: NSObject, XCTestObservation {
    nonisolated(unsafe) static let shared = SpvSuiteCleanupObserver()
    nonisolated(unsafe) var env: IntegrationTestEnv?

    func testBundleDidFinish(_ testBundle: Bundle) {
        env?.cleanupSpvCache()
    }
}
