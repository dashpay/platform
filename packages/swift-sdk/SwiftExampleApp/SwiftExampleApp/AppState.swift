import Foundation
import SwiftData
import SwiftDashSDK

@MainActor
class AppState: ObservableObject {
    @Published var sdk: SDK?
    @Published var isLoading = false
    @Published var showError = false
    @Published var errorMessage = ""

    @Published var currentNetwork: Network {
        didSet {
            UserDefaults.standard.set(Int(currentNetwork.rawValue), forKey: "currentNetwork")
            Task {
                await switchNetwork(to: currentNetwork)
            }
        }
    }

    @Published var dataStatistics: (identities: Int, documents: Int, contracts: Int, tokenBalances: Int)?

    /// The connected network's protocol version, learned by
    /// `refreshProtocolVersion(for:)` on app start and every network
    /// switch. `nil` until the refresh completes (or if it failed) —
    /// consumers that gate behavior on a protocol version (e.g. the
    /// shielded denomination picker in `CreateIdentityView`) should
    /// fall back to the currently-active network behavior when `nil`.
    @Published var platformProtocolVersion: UInt32?

    /// Monotonic tick incremented when a wallet-scoped service rebind
    /// is needed but neither of the standard triggers
    /// (`currentNetwork.onChange`, `wallets.keys.onChange`) will fire.
    /// Concretely: a devnet→devnet SDK rebuild from OptionsView swaps
    /// the cached `PlatformWalletManager` but leaves the network and
    /// wallet ID set unchanged, so `PlatformBalanceSyncService` and
    /// `ShieldedService` keep their references to the old manager.
    /// SwiftExampleAppApp observes this tick to re-run
    /// `rebindWalletScopedServices()` in that edge case.
    @Published var walletScopedServicesRebindTick: Int = 0

    @Published var useDockerSetup: Bool {
        didSet {
            UserDefaults.standard.set(useDockerSetup, forKey: "useDockerSetup")
            // Write to legacy keys so SDK.swift and SPVClient.swift pick them up
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhostPlatform")
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhostCore")
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhost")
            Task { await switchNetwork(to: currentNetwork) }
        }
    }

    // Identity-key signing is performed per-flow via a fresh
    // `KeychainSigner` constructed from the active `ModelContainer`
    // (see `CreateIdentityView.submit()`). `AppState` no longer holds
    // a long-lived signer field — there is no shared signing state to
    // amortize across flows, and the keychain-backed lookup makes
    // construction effectively free.
    private var dataManager: DataManager?
    private var modelContext: ModelContext?

    init() {
        // Load saved network preference or use default. Read via
        // `object(forKey:)` and cast — `integer(forKey:)` returns 0
        // for missing keys, which would silently pin to mainnet.
        if let rawInt = UserDefaults.standard.object(forKey: "currentNetwork") as? Int,
           let network = Network(rawValue: UInt32(rawInt)) {
            self.currentNetwork = network
        } else {
            self.currentNetwork = .testnet
        }
        // Migration: if legacy keys set, propagate to new unified key
        if let _ = UserDefaults.standard.object(forKey: "useDockerSetup") {
            self.useDockerSetup = UserDefaults.standard.bool(forKey: "useDockerSetup")
        } else {
            // Fall back to legacy keys
            let legacyLocal = UserDefaults.standard.bool(forKey: "useLocalhostPlatform")
                || UserDefaults.standard.bool(forKey: "useLocalhost")
            self.useDockerSetup = legacyLocal
            // Persist so SDK.swift can read it (didSet doesn't fire in init)
            UserDefaults.standard.set(legacyLocal, forKey: "useDockerSetup")
        }
    }

    func initializeSDK(modelContext: ModelContext) {
        // Save the model context for later use
        self.modelContext = modelContext

        // Initialize DataManager
        self.dataManager = DataManager(modelContext: modelContext, currentNetwork: currentNetwork)

        Task {
            do {
                isLoading = true

                NSLog("🔵 AppState: Initializing SDK library...")
                SDK.initialize()
                SDK.enableLogging(level: .debug)

                NSLog("🔵 AppState: Creating SDK for network=\(currentNetwork), docker=\(useDockerSetup)")
                let newSDK = try SDK(network: currentNetwork)
                sdk = newSDK
                NSLog("✅ AppState: SDK created successfully")

                // Eagerly learn the network's protocol version so
                // fee-sensitive flows reserve correctly before the
                // first metadata-bearing response ratchets the SDK.
                refreshProtocolVersion(for: newSDK)

                // Load known contracts into the SDK's trusted provider
                await loadKnownContractsIntoSDK(sdk: newSDK, modelContext: modelContext)

                isLoading = false
            } catch {
                sdk = nil
                showError(message: "Failed to initialize SDK: \(error.localizedDescription)")
                NSLog("❌ AppState.initializeSDK: \(error)")
                isLoading = false
            }
        }
    }

    func showError(message: String) {
        errorMessage = message
        showError = true
    }

    func switchNetwork(to network: Network) async {
        guard let modelContext = modelContext else { return }

        // Identities, contracts, documents, and token balances are
        // scoped per-network inside SwiftData. `@Query` consumers
        // filter by `network` and update reactively once we swap
        // the DataManager's scope below — nothing to clear here.

        // Update DataManager's current network
        dataManager?.currentNetwork = network

        // Re-initialize SDK with new network
        do {
            isLoading = true

            // Create new SDK instance for the network
            let newSDK = try SDK(network: network)
            sdk = newSDK

            // Eagerly learn the new network's protocol version (see
            // `initializeSDK`). Non-fatal: the SDK still ratchets from
            // metadata if this fails.
            refreshProtocolVersion(for: newSDK)

            // Load known contracts into the SDK's trusted provider
            await loadKnownContractsIntoSDK(sdk: newSDK, modelContext: modelContext)

            isLoading = false
        } catch {
            sdk = nil
            showError(message: "Failed to switch network: \(error.localizedDescription)")
            NSLog("❌ AppState.switchNetwork: \(error)")
            isLoading = false
        }
    }

    /// Kick off a network protocol-version refresh for `sdk` without
    /// blocking UI readiness.
    ///
    /// `SDK.refreshProtocolVersion()` blocks (it drives a proven
    /// `getEpochsInfo` query to completion on the Rust runtime), so run
    /// it on a background task. The ratchet propagates to the shared
    /// `Arc<AtomicU32>` behind every clone of the SDK — including the
    /// one a `PlatformWalletManager` holds — so shielded fee math sees
    /// the network's real version. Failure is non-fatal: the SDK still
    /// learns the version later from response metadata.
    private func refreshProtocolVersion(for sdk: SDK) {
        // Reset so a network switch never carries the previous
        // network's version while the refresh is in flight.
        platformProtocolVersion = nil
        Task.detached {
            do {
                let version = try sdk.refreshProtocolVersion()
                NSLog("✅ AppState: refreshed protocol version to \(version)")
                await MainActor.run { [weak self] in
                    // Drop a stale result if the SDK was swapped (e.g.
                    // another network switch) while we were querying.
                    guard let self, self.sdk === sdk else { return }
                    self.platformProtocolVersion = version
                }
            } catch {
                NSLog("⚠️ AppState: protocol version refresh failed (non-fatal): \(error.localizedDescription)")
            }
        }
    }

    // Identity, contract, and document mutations are performed
    // directly on SwiftData now. Views own their `ModelContext` and
    // write via `PersistentIdentity` / `PersistentDataContract` /
    // `PersistentDocument` helpers, so the fan-out mutators that
    // used to live here are gone.

    // MARK: - Contract Loading

    private func loadKnownContractsIntoSDK(sdk: SDK, modelContext: ModelContext) async {
        do {
            // Fetch all stored contracts from SwiftData
            let descriptor = FetchDescriptor<PersistentDataContract>()
            let storedContracts = try modelContext.fetch(descriptor)

            guard !storedContracts.isEmpty else {
                NSLog("📦 No stored contracts to load into SDK")
                return
            }

            NSLog("📦 Loading \(storedContracts.count) known contracts into SDK...")

            // Prepare contracts for loading
            var contractsToLoad: [(id: String, data: Data)] = []

            for persistentContract in storedContracts {
                // Use binary serialization if available, otherwise skip
                guard let binaryData = persistentContract.binarySerialization else {
                    NSLog("⚠️ Contract \(persistentContract.idBase58) has no binary serialization, skipping")
                    continue
                }

                contractsToLoad.append((
                    id: persistentContract.idBase58,
                    data: binaryData
                ))
            }

            if !contractsToLoad.isEmpty {
                try sdk.loadKnownContracts(contractsToLoad)
                NSLog("✅ Successfully loaded \(contractsToLoad.count) contracts into SDK's trusted provider")
            } else {
                NSLog("⚠️ No contracts with binary serialization to load")
            }

        } catch {
            NSLog("❌ Failed to load known contracts: \(error)")
            // Don't throw - this is not critical for SDK operation
        }
    }

    // MARK: - Data Statistics

    func getDataStatistics() async -> (identities: Int, documents: Int, contracts: Int, tokenBalances: Int)? {
        guard let dataManager = dataManager else { return nil }

        do {
            return try dataManager.getDataStatistics()
        } catch {
            print("Error getting data statistics: \(error)")
            return nil
        }
    }
}
