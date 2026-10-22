import SwiftDashSDK

/// How an SDK rebuild started from Options ended.
enum SdkRebuildOutcome: Equatable {
    /// SPV did not stop; nothing changed.
    case stopFailed
    /// The network changed while SPV stopped; nothing changed.
    case networkChanged
    /// The SDK build or the wallet manager's move failed and was reported.
    /// The prepared settings were rolled back and nothing was published.
    case failed
    /// The new SDK, its network and the wallet manager on it are current.
    case rebuilt
}

/// Stop SPV, build the SDK for `target`, move the wallet manager onto it, and
/// only then publish the network and the SDK, as one main-actor sequence.
/// Options runs every SDK rebuild through it: a network switch, the Docker
/// toggle and devnet edits.
///
/// Each step runs only if the one before it succeeded:
///
/// 1. `stopSpv`. A running SPV client must never disagree with the settings
///    the new SDK is built from.
/// 2. The app must still be on `network`, re-checked because the stop
///    suspends.
/// 3. `prepare` writes settings the build reads, such as the Docker
///    endpoints.
/// 4. `buildSdk` builds the SDK for `target` without publishing it, or
///    returns `nil` after reporting its failure.
/// 5. `activateManager` moves the wallet manager onto that SDK, or returns
///    false after reporting a failure. The manager takes its SDK clone, and
///    the adaptive provider's SPV source slot, from this SDK.
/// 6. `publish` makes `target` and the SDK current. The manager already
///    runs on that SDK, so the selected network, the published SDK and the
///    active manager change in the same main-actor turn.
/// 7. `rebindWalletScopedServices` binds the services to the active manager.
///
/// If step 4 or 5 fails, `rollBack` undoes `prepare`. Nothing was published,
/// so the selected network, the published SDK and the active manager still
/// match.
///
/// Generic over the SDK type so tests can drive it without building one.
@MainActor
func rebuildSdkAndMoveWalletManager<Sdk>(
    from network: Network,
    to target: Network,
    stopSpv: @MainActor () async -> Bool,
    currentNetwork: @MainActor () -> Network,
    prepare: @MainActor () -> Void,
    buildSdk: @MainActor (Network) async -> Sdk?,
    activateManager: @MainActor (Network, Sdk) -> Bool,
    rollBack: @MainActor () -> Void,
    publish: @MainActor (Network, Sdk) -> Void,
    rebindWalletScopedServices: @MainActor () -> Void
) async -> SdkRebuildOutcome {
    guard await stopSpv() else { return .stopFailed }
    guard currentNetwork() == network else { return .networkChanged }
    prepare()
    guard let sdk = await buildSdk(target), activateManager(target, sdk) else {
        rollBack()
        return .failed
    }
    publish(target, sdk)
    rebindWalletScopedServices()
    return .rebuilt
}
