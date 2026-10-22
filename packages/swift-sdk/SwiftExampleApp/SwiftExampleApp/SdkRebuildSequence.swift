import SwiftDashSDK

/// How an SDK rebuild started from Options ended.
enum SdkRebuildOutcome: Equatable {
    /// SPV did not stop; nothing was applied or rebuilt.
    case stopFailed
    /// The network changed while SPV stopped; nothing was applied or rebuilt.
    case networkChanged
    /// The rebuild ran. Its failures were already shown to the user.
    case finished
}

/// Stop SPV, rebuild the SDK for `target`, and move the wallet manager and the
/// wallet-scoped services onto that SDK, as one main-actor sequence. Options
/// runs every SDK rebuild through it: a network switch, the Docker toggle and
/// devnet edits.
///
/// Each step runs only if the one before it succeeded:
///
/// 1. `stopSpv`. A running SPV client must never disagree with the settings
///    the new SDK is built from.
/// 2. The app must still be on `network`, re-checked because the stop
///    suspends.
/// 3. `apply` changes what the new SDK reads when it is built, such as the
///    published network or the Docker endpoints.
/// 4. `rebuildSdk` builds and publishes the SDK for `target`, or returns `nil`
///    after reporting its failure.
/// 5. `activateManager` receives the SDK step 4 returned. The manager takes
///    its SDK clone, and the adaptive provider's SPV source slot, from that
///    SDK, so it must never be given an SDK read before the rebuild
///    finished. It returns false after reporting a failure.
/// 6. `rebindWalletScopedServices` runs last because it binds the services to
///    the active manager.
///
/// Generic over the SDK type so tests can drive it without building one.
@MainActor
func rebuildSdkAndMoveWalletManager<Sdk>(
    from network: Network,
    to target: Network,
    stopSpv: @MainActor () async -> Bool,
    currentNetwork: @MainActor () -> Network,
    apply: @MainActor () -> Void,
    rebuildSdk: @MainActor (Network) async -> Sdk?,
    activateManager: @MainActor (Network, Sdk) -> Bool,
    rebindWalletScopedServices: @MainActor () -> Void
) async -> SdkRebuildOutcome {
    guard await stopSpv() else { return .stopFailed }
    guard currentNetwork() == network else { return .networkChanged }
    apply()
    guard let sdk = await rebuildSdk(target) else { return .finished }
    guard activateManager(target, sdk) else { return .finished }
    rebindWalletScopedServices()
    return .finished
}
