import SwiftUI
import SwiftData
import SwiftDashSDK

// Holds temporary state for state transitions
@MainActor
class TransitionState: ObservableObject {
    @Published var documentPrice: UInt64?
    @Published var canPurchaseDocument: Bool = false
    @Published var documentPurchaseError: String?

    func reset() {
        documentPrice = nil
        canPurchaseDocument = false
        documentPurchaseError = nil
    }
}

@MainActor
class UnifiedAppState: ObservableObject {
    @Published var isInitialized = false
    @Published var error: Error?
    // Controls whether the detailed sync banner should be shown on Wallets tab
    @Published var showWalletsSyncDetails: Bool = true

    // Services from Core
    let walletService: WalletService

    // Shielded pool service
    let shieldedService = ShieldedService()

    // Platform address balance sync service (BLAST sync)
    let platformBalanceSyncService = PlatformBalanceSyncService()

    // State from Platform
    let platformState: AppState

    // Unified state manager
    let unifiedState: UnifiedStateManager

    // SwiftData container
    let modelContainer: ModelContainer

    // Transition state for temporary data
    @Published var transitionState = TransitionState()

    // Task for the periodic sync loop
    private var syncLoopTask: Task<Void, Never>?

    // Computed property for easy SDK access
    var sdk: SDK? {
        platformState.sdk
    }

    init() {
        // Initialize SwiftData
        do {
            modelContainer = try ModelContainerHelper.createContainer()
        } catch {
            fatalError("Failed to create ModelContainer: \(error)")
        }

        // Initialize services
        self.platformState = AppState()
        self.walletService = WalletService(modelContainer: modelContainer, network: platformState.currentNetwork)
        // Initialize unified state (will be updated with real SDKs during async init)
        self.unifiedState = UnifiedStateManager()
    }

    func initialize() async {
        // Initialize Platform SDK
        await MainActor.run {
            platformState.initializeSDK(modelContext: modelContainer.mainContext)
        }

        // Wait for Platform SDK to be ready
        try? await Task.sleep(nanoseconds: 500_000_000) // 0.5 second

        // If SDK reports trusted mode, disable masternode SPV sync
        if let sdk = platformState.sdk {
            do {
                let status: SwiftDashSDK.SDKStatus = try sdk.getStatus()
                let isTrusted = status.mode.lowercased() == "trusted"
                await MainActor.run { self.walletService.setMasternodesEnabled(!isTrusted) }
            } catch {
                // Ignore status errors; keep default (false) until known
            }
        }

        // Initialize shielded pool client
        initializeShieldedService()

        // Start periodic BLAST address sync
        startPlatformBalanceSync()

        isInitialized = true
    }

    func reset() async {
        isInitialized = false
        error = nil

        // Reset services
        walletService.stopSync()
        shieldedService.reset()
        syncLoopTask?.cancel()
        syncLoopTask = nil
        platformBalanceSyncService.reset()

        // Reset platform state
        platformState.sdk = nil
        platformState.isLoading = false
        platformState.showError = false
        platformState.errorMessage = ""
        platformState.identities = []
        platformState.contracts = []
        platformState.tokens = []
        platformState.documents = []
    }

    // Handle network switching - called when platformState.currentNetwork changes
    func handleNetworkSwitch(to network: AppNetwork) async {
        // Switch wallet service to new network (convert to DashNetwork)
        await walletService.switchNetwork(to: network)

        // Reinitialize shielded service for the new network
        initializeShieldedService()

        // Restart BLAST sync for the new network
        startPlatformBalanceSync()

        // The platform state handles its own network switching in AppState.switchNetwork
    }

    /// Start periodic BLAST address balance sync (every 15 seconds).
    func startPlatformBalanceSync() {
        // Cancel any previous sync loop
        syncLoopTask?.cancel()

        let network = platformState.currentNetwork
        platformBalanceSyncService.startPeriodicSync(network: network)

        // Run a repeating async loop
        syncLoopTask = Task { [weak self] in
            // Initial delay for SDK quorum prefetch
            try? await Task.sleep(for: .seconds(2))
            await self?.performPlatformBalanceSync()

            // Repeat every 15 seconds
            while !Task.isCancelled {
                do {
                    try await Task.sleep(for: .seconds(15))
                } catch {
                    break // Task was cancelled
                }
                await self?.performPlatformBalanceSync()
            }
        }
    }

    /// Perform a single BLAST address sync using the wallet's DIP-17 platform payment addresses.
    /// Skips silently if there are no wallets or no platform addresses to sync.
    func performPlatformBalanceSync() async {
        guard let sdk = platformState.sdk else { return }

        // No wallet → nothing to sync
        let wallets = walletService.walletManager.wallets
        guard let firstWallet = wallets.first else { return }

        do {
            // Ensure platform payment account exists (no-op if already there)
            try walletService.walletManager.ensurePlatformPaymentAccount(for: firstWallet)

            // Get addresses from the wallet's DIP-17 address pool
            let addresses = try walletService.walletManager.getPlatformAddresses(for: firstWallet)

            NSLog("BLAST sync: got \(addresses.count) platform addresses for wallet \(firstWallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined())...")

            // No addresses → skip sync entirely (don't hit the network)
            guard !addresses.isEmpty else { return }

            await platformBalanceSyncService.performSync(sdk: sdk, addresses: addresses)
        } catch {
            NSLog("BLAST sync: error getting platform addresses: \(error)")
            SDKLogger.log(
                "BLAST sync: failed to get platform addresses: \(error.localizedDescription)",
                minimumLevel: .medium
            )
        }
    }

    /// Initialize the shielded service using the first wallet's seed.
    /// Call after wallet seed becomes available or on network switch.
    func initializeShieldedService() {
        // Get the first wallet's seed data (first 32 bytes used as spending key)
        // For now, use the wallet's serializedWalletBytes as a seed source
        let wallets = walletService.walletManager.wallets
        guard let firstWallet = wallets.first else { return }

        // Use first 32 bytes of the wallet's serialized bytes as the spending key
        // (proper ZIP32 derivation can be added later)
        let seedData = firstWallet.serializedWalletBytes
        guard seedData.count >= 32 else { return }

        shieldedService.initialize(seed: seedData, network: platformState.currentNetwork)
    }
}
