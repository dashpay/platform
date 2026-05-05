import Foundation
import SwiftData
import SwiftUI
import SwiftDashSDK

/// Coordinator that holds one `PlatformWalletManager` per network.
///
/// The Rust `PlatformWalletManager` is intentionally network-locked
/// — its inner `WalletManager` is constructed with `sdk.network` at
/// `configure(...)` time and the SDK reference (plus the SPV
/// runtime, BLAST sync coordinator, and persistence handler hung off
/// it) all stay bound to that single network for the manager's
/// lifetime. Switching networks at runtime therefore needs a
/// **different manager**, not a reconfigured one.
///
/// `WalletManagerStore` materializes that design on the iOS side:
///
///   * Lazy-creates a `PlatformWalletManager` the first time a
///     network is activated, keying on `Network` so subsequent
///     activations of the same network return the warm manager.
///   * Each manager's persistence handler is constructed with the
///     same network (via `configure(sdk:modelContainer:)`), which
///     scopes `loadWalletList` to that network's `PersistentWallet`
///     rows — multiple managers writing into the shared SwiftData
///     store don't trip over each other on hydration.
///   * Publishes `activeManager` so the SwiftUI scene can re-inject
///     it via `.environmentObject(...)` on every switch. Existing
///     view consumers (`@EnvironmentObject var walletManager:
///     PlatformWalletManager`) keep working unchanged — they just
///     see the manager for the active network on each render after
///     a switch.
///
/// Inactive managers stay alive in the background. SPV / BLAST sync
/// continue running for them; the user can flip Testnet ↔ Local ↔
/// ... and resume per-network state without losing in-flight sync
/// progress. The cost is N×manager memory for N networks the user
/// has touched this session — tractable for the 4-network upper
/// bound the app supports today, and the right trade-off for a
/// dev-focused example app where flipping is common.
@MainActor
final class WalletManagerStore: ObservableObject {
    /// Currently-active manager. Reassigned when the user switches
    /// networks; SwiftUI re-injects this into the env object so
    /// every `@EnvironmentObject var walletManager:
    /// PlatformWalletManager` consumer rebinds to the right
    /// instance on each render.
    ///
    /// Starts as a placeholder, unconfigured `PlatformWalletManager`
    /// — the bootstrap path replaces it via `activate(...)` once
    /// the SDK for the initial network is ready, and views are
    /// gated behind `isInitialized` until that happens (same
    /// gating the pre-refactor single-manager flow used).
    @Published private(set) var activeManager: PlatformWalletManager

    /// Per-network managers. Lazily populated on first activation
    /// of each network; lookup is O(1).
    private var managers: [Network: PlatformWalletManager] = [:]

    /// SwiftData container shared across every manager. Each
    /// manager's persistence handler narrows its `loadWalletList`
    /// fetch to its own network so the shared store doesn't cause
    /// cross-network bleed at restore time.
    private let modelContainer: ModelContainer

    init(modelContainer: ModelContainer) {
        self.modelContainer = modelContainer
        // Placeholder. Held until the first `activate(...)` call
        // replaces it with a configured manager. Views gate against
        // bootstrap via `isInitialized` so they never see this
        // intermediate value.
        self.activeManager = PlatformWalletManager()
    }

    /// Activate the manager for `network`, lazily creating + caching
    /// one configured against `sdk` if it's not already warm.
    ///
    /// Idempotent: re-activating the current network is a no-op.
    /// Failures during a fresh manager's `configure` /
    /// `loadFromPersistor` propagate to the caller — the cache
    /// stays untouched in that case so a later retry can succeed.
    func activate(network: Network, sdk: SDK) throws {
        if let existing = managers[network] {
            if existing !== activeManager {
                activeManager = existing
            }
            return
        }
        let manager = PlatformWalletManager()
        try manager.configure(sdk: sdk, modelContainer: modelContainer)
        // Best-effort: a fresh manager comes up with no wallets in
        // memory; restore from the network-scoped persistence
        // handler so reopening the app on this network resurfaces
        // the wallets the user already created here. Failures here
        // are non-fatal — the user can still create / import
        // wallets, the in-memory set just starts empty.
        do {
            _ = try manager.loadFromPersistor()
        } catch {
            SDKLogger.error(
                "WalletManagerStore: load-from-persistor failed for "
                    + "\(network.displayName): \(error.localizedDescription)"
            )
        }
        managers[network] = manager
        activeManager = manager
    }

    /// Manager for `network` if one has been activated this session;
    /// otherwise `nil`. Used for diagnostics surfaces (e.g. Wallet
    /// Memory Explorer) that want to inspect a specific network's
    /// state without forcing it active.
    func manager(for network: Network) -> PlatformWalletManager? {
        managers[network]
    }
}
