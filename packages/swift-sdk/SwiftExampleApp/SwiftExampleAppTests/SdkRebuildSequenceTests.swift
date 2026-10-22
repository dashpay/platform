import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the order of `rebuildSdkAndMoveWalletManager`, the sequence every
/// SDK rebuild from Options runs: stop SPV, apply the new settings, build
/// the SDK, move the wallet manager onto exactly that SDK, then rebind the
/// wallet-scoped services. A manager activated with the SDK published
/// before the rebuild keeps the previous network's SDK clone and its
/// adaptive provider's SPV source slot.
///
/// The steps are injected, so no real SDK or `PlatformWalletManager` is
/// built; `FakeSdk` stands in for the SDK and is compared by identity.
@MainActor
final class SdkRebuildSequenceTests: XCTestCase {

    private final class FakeSdk {}

    private enum Step: Equatable {
        case stopSpv
        case apply
        case rebuildSdk(Network)
        case activateManager(Network)
        case rebindWalletScopedServices
    }

    /// Records the steps and models the app's published network and SDK.
    private final class AppModel {
        var steps: [Step] = []
        var currentNetwork: Network
        var publishedSdk = FakeSdk()
        var activatedSdk: FakeSdk?

        init(currentNetwork: Network) {
            self.currentNetwork = currentNetwork
        }
    }

    private func run(
        _ app: AppModel,
        from network: Network,
        to target: Network,
        stopSucceeds: Bool = true,
        networkAfterStop: Network? = nil,
        rebuildSucceeds: Bool = true,
        activationSucceeds: Bool = true,
        apply: @MainActor () -> Void = {}
    ) async -> SdkRebuildOutcome {
        await rebuildSdkAndMoveWalletManager(
            from: network,
            to: target,
            stopSpv: {
                app.steps.append(.stopSpv)
                if let networkAfterStop {
                    app.currentNetwork = networkAfterStop
                }
                return stopSucceeds
            },
            currentNetwork: { app.currentNetwork },
            apply: {
                app.steps.append(.apply)
                apply()
            },
            rebuildSdk: { (network: Network) -> FakeSdk? in
                app.steps.append(.rebuildSdk(network))
                guard rebuildSucceeds else { return nil }
                let sdk = FakeSdk()
                app.publishedSdk = sdk
                return sdk
            },
            activateManager: { (network: Network, sdk: FakeSdk) -> Bool in
                app.steps.append(.activateManager(network))
                app.activatedSdk = sdk
                return activationSucceeds
            },
            rebindWalletScopedServices: {
                app.steps.append(.rebindWalletScopedServices)
            }
        )
    }

    /// A network switch publishes the new network after SPV stops, builds
    /// that network's SDK, and moves the manager onto that SDK before the
    /// services rebind.
    func testNetworkSwitchMovesTheManagerOntoTheNewNetworksSdk() async {
        let app = AppModel(currentNetwork: .testnet)
        let previousSdk = app.publishedSdk

        let outcome = await run(app, from: .testnet, to: .regtest) {
            app.currentNetwork = .regtest
        }

        XCTAssertEqual(outcome, .finished)
        XCTAssertEqual(
            app.steps,
            [
                .stopSpv,
                .apply,
                .rebuildSdk(.regtest),
                .activateManager(.regtest),
                .rebindWalletScopedServices,
            ]
        )
        XCTAssertTrue(
            app.activatedSdk === app.publishedSdk,
            "the manager must move onto the SDK the rebuild published"
        )
        XCTAssertFalse(
            app.activatedSdk === previousSdk,
            "the manager must not stay on the previous network's SDK"
        )
    }

    /// The Docker toggle and devnet edits rebuild on the same network. The
    /// network and the wallet ID set stay unchanged, so only this sequence
    /// rebinds the services, after the manager moved onto the new SDK.
    func testSameNetworkRebuildMovesTheManagerOntoTheNewSdk() async {
        let app = AppModel(currentNetwork: .regtest)
        let previousSdk = app.publishedSdk

        let outcome = await run(app, from: .regtest, to: .regtest)

        XCTAssertEqual(outcome, .finished)
        XCTAssertEqual(
            app.steps,
            [
                .stopSpv,
                .apply,
                .rebuildSdk(.regtest),
                .activateManager(.regtest),
                .rebindWalletScopedServices,
            ]
        )
        XCTAssertTrue(app.activatedSdk === app.publishedSdk)
        XCTAssertFalse(app.activatedSdk === previousSdk)
    }

    /// A failed stop may leave the old run loop parked. Nothing changes, so
    /// the app stays on its network and picking again retries the stop.
    func testFailedStopAppliesAndRebuildsNothing() async {
        let app = AppModel(currentNetwork: .testnet)

        let outcome = await run(app, from: .testnet, to: .regtest, stopSucceeds: false) {
            app.currentNetwork = .regtest
        }

        XCTAssertEqual(outcome, .stopFailed)
        XCTAssertEqual(app.steps, [.stopSpv])
        XCTAssertEqual(app.currentNetwork, .testnet)
    }

    /// The stop suspends; a flow whose network was replaced meanwhile must
    /// not rebuild for a network the app has left.
    func testNetworkChangedDuringTheStopRebuildsNothing() async {
        let app = AppModel(currentNetwork: .testnet)

        let outcome = await run(app, from: .testnet, to: .testnet, networkAfterStop: .devnet)

        XCTAssertEqual(outcome, .networkChanged)
        XCTAssertEqual(app.steps, [.stopSpv])
    }

    /// With no new SDK there is nothing to move the manager onto, and the
    /// services must not rebind against the manager left behind.
    func testFailedRebuildSkipsActivationAndRebind() async {
        let app = AppModel(currentNetwork: .testnet)

        let outcome = await run(app, from: .testnet, to: .regtest, rebuildSucceeds: false)

        XCTAssertEqual(outcome, .finished)
        XCTAssertEqual(app.steps, [.stopSpv, .apply, .rebuildSdk(.regtest)])
        XCTAssertNil(app.activatedSdk)
    }

    /// The services bind to the active manager. If it did not move, a rebind
    /// would attach them to the previous one.
    func testFailedActivationSkipsRebind() async {
        let app = AppModel(currentNetwork: .regtest)

        let outcome = await run(app, from: .regtest, to: .regtest, activationSucceeds: false)

        XCTAssertEqual(outcome, .finished)
        XCTAssertEqual(
            app.steps,
            [.stopSpv, .apply, .rebuildSdk(.regtest), .activateManager(.regtest)]
        )
    }
}
