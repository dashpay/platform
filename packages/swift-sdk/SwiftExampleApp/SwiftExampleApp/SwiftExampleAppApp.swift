//
//  SwiftExampleAppApp.swift
//  SwiftExampleApp
//
//  Created by Sam Westrich on 8/6/25.
//

import SwiftUI
import SwiftData

// Pull in logging helpers for SDKLogger and presets
import SwiftDashSDK

/// UI-only state that used to live on UnifiedAppState. Small bag of flags that
/// coordinate between views.
@MainActor
final class AppUIState: ObservableObject {
    /// Whether the detailed sync banner should be shown on the Wallets tab.
    @Published var showWalletsSyncDetails: Bool = true
}

@main
struct SwiftExampleAppApp: App {
    // SwiftData container — shared across services and views.
    private let modelContainer: ModelContainer

    // Platform identity / document / contract state.
    @StateObject private var platformState = AppState()

    // The one wallet manager — drives SPV, BLAST sync, wallet creation, etc.
    @StateObject private var walletManager = PlatformWalletManager()

    // Remaining services.
    @StateObject private var shieldedService = ShieldedService()
    @StateObject private var platformBalanceSyncService = PlatformBalanceSyncService()
    @StateObject private var unifiedStateManager = UnifiedStateManager()
    @StateObject private var transitionState = TransitionState()
    @StateObject private var appUIState = AppUIState()

    @State private var isInitialized = false
    @State private var bootstrapError: Error?
    @State private var bootstrapTask: Task<Void, Never>?
    @State private var platformBalanceSyncTask: Task<Void, Never>?

    init() {
        // Suppress auto layout constraint warnings in debug builds
        // These are typically harmless keyboard-related warnings
        #if DEBUG
        UserDefaults.standard.set(false, forKey: "_UIConstraintBasedLayoutLogUnsatisfiable")
        #endif

        do {
            self.modelContainer = try ModelContainerHelper.createContainer()
        } catch {
            fatalError("Failed to create ModelContainer: \(error)")
        }
    }

    var body: some Scene {
        WindowGroup {
            ContentView(isInitialized: isInitialized, bootstrapError: bootstrapError, onRetry: retryBootstrap)
                .environmentObject(platformState)
                .environmentObject(walletManager)
                .environmentObject(shieldedService)
                .environmentObject(platformBalanceSyncService)
                .environmentObject(unifiedStateManager)
                .environmentObject(transitionState)
                .environmentObject(appUIState)
                .environment(\.modelContext, modelContainer.mainContext)
                .task {
                    SDKLogger.log("🚀 SwiftExampleApp: Starting initialization...", minimumLevel: .medium)
                    await bootstrap()
                    SDKLogger.log("🚀 SwiftExampleApp: Initialization complete", minimumLevel: .medium)
                }
        }
    }

    @MainActor
    private func bootstrap() async {
        do {
            LoggingPreferences.configure()

            platformState.initializeSDK(modelContext: modelContainer.mainContext)

            // Give the Platform SDK a moment to finish its internal init.
            try? await Task.sleep(for: .milliseconds(500))

            if let sdk = platformState.sdk {
                // Configure the wallet manager.
                try walletManager.configure(sdk: sdk, modelContainer: modelContainer)

                // Initialize shielded pool using first available wallet's data.
                initializeShieldedService()

                // Start SPV on the chosen network.
                let dataDirURL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
                    .first!
                    .appendingPathComponent("SPV")
                    .appendingPathComponent(platformState.currentNetwork.rawValue)
                try? FileManager.default.createDirectory(at: dataDirURL, withIntermediateDirectories: true)

                // Respect trusted mode reported by the platform SDK for masternode sync.
                var masternodeEnabled = true
                if let status: SwiftDashSDK.SDKStatus = try? sdk.getStatus() {
                    masternodeEnabled = status.mode.lowercased() != "trusted"
                }

                let useLocalCore = UserDefaults.standard.bool(forKey: "useLocalhostCore")
                let peers: [String] = useLocalCore ? readLocalCorePeers() : []

                let spvConfig = PlatformSpvStartConfig(
                    dataDir: dataDirURL.path,
                    network: platformNetwork(from: platformState.currentNetwork),
                    peers: peers,
                    restrictToConfiguredPeers: useLocalCore,
                    masternodeSyncEnabled: masternodeEnabled
                )

                do {
                    try walletManager.startSpv(config: spvConfig)
                } catch {
                    SDKLogger.error("Failed to start SPV: \(error.localizedDescription)")
                }

                // Start the periodic BLAST balance sync loop.
                startPlatformBalanceSyncLoop()
            }

            isInitialized = true
        } catch {
            bootstrapError = error
        }
    }

    @MainActor
    private func retryBootstrap() {
        bootstrapError = nil
        bootstrapTask?.cancel()
        bootstrapTask = Task {
            await bootstrap()
        }
    }

    // MARK: - Helpers

    private func platformNetwork(from network: AppNetwork) -> PlatformNetwork {
        switch network {
        case .mainnet: return .mainnet
        case .testnet: return .testnet
        case .devnet: return .devnet
        case .regtest: return .testnet
        }
    }

    /// Read local Core peers from UserDefaults (comma-separated addresses).
    private func readLocalCorePeers() -> [String] {
        if let csv = UserDefaults.standard.string(forKey: "localCorePeers"), !csv.isEmpty {
            return csv.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        }
        return ["127.0.0.1"]
    }

    /// Initialize the shielded pool client. Best-effort — does nothing if no
    /// wallet is available yet.
    private func initializeShieldedService() {
        // TODO(platform-wallet): Derive a ZIP32 spending key from the managed
        // wallet instead of reusing the legacy HDWallet.serializedWalletBytes.
        // For now the shielded service starts empty; it will be re-initialized
        // once the user creates/loads a wallet via `createWallet(...)`.
    }

    /// Start a background loop that drives `platformBalanceSyncService` every
    /// 15 seconds. Cancelled when the app is torn down.
    private func startPlatformBalanceSyncLoop() {
        platformBalanceSyncTask?.cancel()
        platformBalanceSyncTask = Task { [platformBalanceSyncService] in
            // Small initial delay to let the SDK pre-fetch quorums.
            try? await Task.sleep(for: .seconds(2))
            await platformBalanceSyncService.performSync()

            while !Task.isCancelled {
                do {
                    try await Task.sleep(for: .seconds(15))
                } catch {
                    break
                }
                await platformBalanceSyncService.performSync()
            }
        }
    }
}
