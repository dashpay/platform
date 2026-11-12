import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins that the quorum mode the user picks reaches every SDK the app built
/// that is still alive, not only the published one: the wallet managers'
/// background SDKs verify proofs too, and one left in Auto after the user
/// chose SPV would keep falling back to trusted quorum keys.
@MainActor
final class AppStateQuorumModeTests: XCTestCase {

    /// Save the quorum mode the app keeps in the UserDefaults these tests
    /// share, and point the SDKs they build at loopback endpoints; return
    /// what restores both.
    private func isolateSettings() -> () -> Void {
        let saved = UserDefaults.standard.string(forKey: "quorumMode")
        let restoreEndpoints = AppStateSdkRebuildTests.pinLoopbackEndpoints()
        return {
            if let saved {
                UserDefaults.standard.set(saved, forKey: "quorumMode")
            } else {
                UserDefaults.standard.removeObject(forKey: "quorumMode")
            }
            restoreEndpoints()
        }
    }

    /// An SDK is built in the mode the user picked, before anything can
    /// verify a proof with it: built in Auto and switched later, it would
    /// fall back to trusted keys in between.
    func testShouldBuildAnSdkInTheSelectedQuorumMode() throws {
        let restore = isolateSettings()
        defer { restore() }
        let appState = AppState()
        appState.quorumMode = .spv

        let built = try appState.makeSdk(for: .regtest)

        XCTAssertEqual(built.quorumSource, .spv)
    }

    func testShouldApplyTheQuorumModeToEverySdkItBuilt() throws {
        let restore = isolateSettings()
        defer { restore() }
        let appState = AppState()
        appState.quorumMode = .auto

        let first = try appState.makeSdk(for: .regtest)
        let second = try appState.makeSdk(for: .regtest)
        XCTAssertEqual(second.quorumSource, .trusted, "Auto without SPV asks the trusted service")

        appState.quorumMode = .spv
        appState.applyQuorumMode()
        XCTAssertEqual(first.quorumSource, .spv)
        XCTAssertEqual(second.quorumSource, .spv)
    }
}
