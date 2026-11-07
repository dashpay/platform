import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the order of `rebuildSdkAndMoveWalletManager`, the sequence every
/// SDK rebuild from Options runs: stop SPV, prepare the settings the build
/// reads, build the SDK, move the wallet manager onto exactly that SDK, then
/// publish the SDK and its network together and rebind the wallet-scoped
/// services. A manager activated with an SDK other than the one published
/// keeps another network's SDK clone and adaptive provider SPV source slot,
/// and a network published before a failed build or move leaves the
/// selected network out of step with the active manager.
///
/// The steps are injected, so no real SDK or `PlatformWalletManager` is
/// built; `FakeSdk` stands in for the SDK and is compared by identity.
@MainActor
final class SdkRebuildSequenceTests: XCTestCase {

    private final class FakeSdk {}

    private enum Step: Equatable {
        case stopSpv
        case prepare
        case buildSdk(Network)
        case activateManager(Network)
        case rollBack
        case publish(Network)
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
        buildSucceeds: Bool = true,
        activationSucceeds: Bool = true
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
            prepare: { app.steps.append(.prepare) },
            buildSdk: { (network: Network) -> FakeSdk? in
                app.steps.append(.buildSdk(network))
                return buildSucceeds ? FakeSdk() : nil
            },
            activateManager: { (network: Network, sdk: FakeSdk) -> Bool in
                app.steps.append(.activateManager(network))
                guard activationSucceeds else { return false }
                app.activatedSdk = sdk
                return true
            },
            rollBack: { app.steps.append(.rollBack) },
            publish: { (network: Network, sdk: FakeSdk) in
                app.steps.append(.publish(network))
                app.currentNetwork = network
                app.publishedSdk = sdk
            },
            rebindWalletScopedServices: {
                app.steps.append(.rebindWalletScopedServices)
            }
        )
    }

    /// A network switch builds the new network's SDK and moves the manager
    /// onto it before publishing the network and that SDK; the services
    /// rebind last.
    func testNetworkSwitchPublishesTheSdkTheManagerMovedOnto() async {
        let app = AppModel(currentNetwork: .testnet)
        let previousSdk = app.publishedSdk

        let outcome = await run(app, from: .testnet, to: .regtest)

        XCTAssertEqual(outcome, .rebuilt)
        XCTAssertEqual(
            app.steps,
            [
                .stopSpv,
                .prepare,
                .buildSdk(.regtest),
                .activateManager(.regtest),
                .publish(.regtest),
                .rebindWalletScopedServices,
            ]
        )
        XCTAssertEqual(app.currentNetwork, .regtest)
        XCTAssertTrue(
            app.activatedSdk === app.publishedSdk,
            "the manager must run on the SDK that is published"
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

        XCTAssertEqual(outcome, .rebuilt)
        XCTAssertEqual(
            app.steps,
            [
                .stopSpv,
                .prepare,
                .buildSdk(.regtest),
                .activateManager(.regtest),
                .publish(.regtest),
                .rebindWalletScopedServices,
            ]
        )
        XCTAssertTrue(app.activatedSdk === app.publishedSdk)
        XCTAssertFalse(app.activatedSdk === previousSdk)
    }

    /// A failed stop may leave the old run loop parked. Nothing changes, so
    /// the app stays on its network and picking again retries the stop.
    func testFailedStopChangesNothing() async {
        let app = AppModel(currentNetwork: .testnet)

        let outcome = await run(app, from: .testnet, to: .regtest, stopSucceeds: false)

        XCTAssertEqual(outcome, .stopFailed)
        XCTAssertEqual(app.steps, [.stopSpv])
        XCTAssertEqual(app.currentNetwork, .testnet)
    }

    /// The stop suspends; a flow whose network was replaced meanwhile must
    /// not rebuild for a network the app has left.
    func testNetworkChangedDuringTheStopChangesNothing() async {
        let app = AppModel(currentNetwork: .testnet)

        let outcome = await run(app, from: .testnet, to: .testnet, networkAfterStop: .devnet)

        XCTAssertEqual(outcome, .networkChanged)
        XCTAssertEqual(app.steps, [.stopSpv])
    }

    /// A failed SDK build during a network switch must leave the selected
    /// network and the published SDK as they were, matching the manager that
    /// stays active, and undo the prepared settings.
    func testFailedBuildDuringANetworkSwitchKeepsTheSelectedNetwork() async {
        let app = AppModel(currentNetwork: .testnet)
        let previousSdk = app.publishedSdk

        let outcome = await run(app, from: .testnet, to: .devnet, buildSucceeds: false)

        XCTAssertEqual(outcome, .failed)
        XCTAssertEqual(app.steps, [.stopSpv, .prepare, .buildSdk(.devnet), .rollBack])
        XCTAssertEqual(app.currentNetwork, .testnet)
        XCTAssertTrue(app.publishedSdk === previousSdk)
        XCTAssertNil(app.activatedSdk)
    }

    /// A manager that could not move stays on the previous SDK, so that SDK
    /// and its network must stay published, and the services must not
    /// rebind.
    func testFailedActivationDuringANetworkSwitchKeepsTheSelectedNetwork() async {
        let app = AppModel(currentNetwork: .testnet)
        let previousSdk = app.publishedSdk

        let outcome = await run(app, from: .testnet, to: .devnet, activationSucceeds: false)

        XCTAssertEqual(outcome, .failed)
        XCTAssertEqual(
            app.steps,
            [.stopSpv, .prepare, .buildSdk(.devnet), .activateManager(.devnet), .rollBack]
        )
        XCTAssertEqual(app.currentNetwork, .testnet)
        XCTAssertTrue(app.publishedSdk === previousSdk)
    }
}
