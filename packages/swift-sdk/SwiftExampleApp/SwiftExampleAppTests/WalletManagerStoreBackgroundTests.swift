import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// The SDK builder the store tests give `WalletManagerStore`: a strict SPV
/// SDK builds without a network and starts no sync.
@MainActor
func strictSpvSdk(_ network: Network) throws -> SDK {
    try SDK(network: network, contextProvider: .spv)
}

/// Pins that `WalletManagerStore.backgroundManager(for:)` builds its SDK
/// with the app's builder, the one that applies the user's quorum mode. A
/// background SDK of its own would verify orphan recovery and other-network
/// wallet creation with trusted quorum keys while the user chose SPV.
@MainActor
final class WalletManagerStoreBackgroundTests: XCTestCase {

    /// The networks the builder was asked for.
    @MainActor
    private final class Builds {
        var networks: [Network] = []
    }

    func testShouldBuildABackgroundManagersSdkWithTheAppsBuilder() async throws {
        let builds = Builds()
        let store = WalletManagerStore(
            modelContainer: try DashModelContainer.createInMemory(),
            makeSdk: { network in
                builds.networks.append(network)
                return try strictSpvSdk(network)
            }
        )

        let manager = try await store.backgroundManager(for: .regtest)
        XCTAssertEqual(builds.networks, [.regtest])
        XCTAssertTrue(manager.isConfigured)
        XCTAssertNotIdentical(store.activeManager, manager, "a background manager is not made active")

        let again = try await store.backgroundManager(for: .regtest)
        XCTAssertIdentical(again, manager)
        XCTAssertEqual(builds.networks, [.regtest], "a cached manager needs no new SDK")
        await manager.shutdown()
    }
}
