import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the supersession guard in `CoreSpvLauncher.start`: a start that
/// is superseded during its off-main peer resolution must throw
/// `CancellationError` and must NOT reach `startSpv`, so it can't revive
/// a stopped/replaced manager. Exercised through the injectable
/// `startSpv` seam so no real (network-locked) `PlatformWalletManager`
/// is needed.
///
/// `.testnet` is used throughout so `peerOverride` takes its
/// non-blocking branch (no `SDK.discoverActiveMasternodes` round-trip):
/// with the default `UserDefaults` (`useLocalhostCore`/`useDockerSetup`
/// unset) it returns an empty peer list synchronously.
@MainActor
final class CoreSpvLauncherStartTests: XCTestCase {

    func testStaleStartThrowsCancellationAndNeverCallsStartSpv() async {
        var startSpvInvoked = false
        do {
            try await CoreSpvLauncher.start(
                network: .testnet,
                stillCurrent: { false },
                startSpv: { _ in startSpvInvoked = true }
            )
            XCTFail("Expected CancellationError for a superseded start")
        } catch is CancellationError {
            // Expected.
        } catch {
            XCTFail("Expected CancellationError, got \(error)")
        }
        XCTAssertFalse(
            startSpvInvoked,
            "startSpv must not run once the start is superseded"
        )
    }

    func testFreshStartInvokesStartSpvWithRequestedNetwork() async throws {
        var capturedConfig: PlatformSpvStartConfig?
        try await CoreSpvLauncher.start(
            network: .testnet,
            stillCurrent: { true },
            startSpv: { config in capturedConfig = config }
        )
        XCTAssertEqual(
            capturedConfig?.network,
            .testnet,
            "A current start must reach startSpv with the requested network"
        )
    }

    /// Mirrors the call-site closure shape (`spvStartGeneration ==
    /// captured`): when the generation moves after the start was armed,
    /// the guard bails even though nothing else changed.
    func testGenerationMismatchBailsWithoutStarting() async {
        final class GenerationBox { var value: UInt64 = 0 }
        let box = GenerationBox()
        let captured = box.value
        // A supersession bumps the generation while the start is in
        // flight (armed with `captured`).
        box.value &+= 1

        var startSpvInvoked = false
        do {
            try await CoreSpvLauncher.start(
                network: .testnet,
                stillCurrent: { box.value == captured },
                startSpv: { _ in startSpvInvoked = true }
            )
            XCTFail("Expected CancellationError for a generation mismatch")
        } catch is CancellationError {
            // Expected.
        } catch {
            XCTFail("Expected CancellationError, got \(error)")
        }
        XCTAssertFalse(startSpvInvoked)
    }
}
