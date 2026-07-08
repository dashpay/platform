import XCTest
@testable import SwiftDashSDK

/// Regression: clearing SPV storage must work *after* the client is stopped
final class SpvClearStorageIntegrationTests: IntegrationTestCase {
    func testClearStorageAfterStopWipesDisk() async throws {
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.waitUntilUpToDate(
            height: try await env.coreRPC.getBlockCount(),
            timeout: 30
        )

        let before = Self.dirSignature(env.spvDataDir)
        XCTAssertFalse(
            before.isEmpty,
            "expected synced SPV data on disk before clearing"
        )

        try await env.walletManager.stopSpv()
        let running = try await env.walletManager.isSpvRunning()
        XCTAssertFalse(running, "SPV should be stopped")

        try await env.walletManager.clearSpvStorage()

        let after = Self.dirSignature(env.spvDataDir)
        XCTAssertNotEqual(
            after, before,
            "clearSpvStorage after stopSpv must wipe the synced data on disk"
        )

        // The stopped-path clear must release the storage lock: a fresh start
        // on the same data dir must still succeed (no leftover lockfile).
        try await env.walletManager.startSpv(config: env.spvConfig)
        try await env.walletManager.stopSpv()
    }

    /// Sorted "relativePath:size" list of every regular file under `path`.
    /// Mirrors the harness's own cache signature so a wipe is observable as a
    /// change in the file/size set.
    private static func dirSignature(_ path: String) -> String {
        let fm = FileManager.default
        let root = URL(fileURLWithPath: path).resolvingSymlinksInPath()
        let rootCount = root.pathComponents.count

        guard let walker = fm.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey]
        ) else { return "" }

        var entries: [String] = []
        for case let url as URL in walker {
            let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
            guard values?.isRegularFile == true else { continue }

            let rel = url.resolvingSymlinksInPath().pathComponents
                .dropFirst(rootCount)
                .joined(separator: "/")

            entries.append("\(rel):\(values?.fileSize ?? 0)")
        }

        return entries.sorted().joined(separator: "\n")
    }
}
