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

    init() {
        // Suppress auto layout constraint warnings in debug builds
        // These are typically harmless keyboard-related warnings
        #if DEBUG
        UserDefaults.standard.set(false, forKey: "_UIConstraintBasedLayoutLogUnsatisfiable")
        #endif

        // Wipe legacy keychain residue before any SDK component gets
        // a chance to read under a stale service name. Covers the
        // old `org.dash.wallet` / `com.dash.sdk.keys` /
        // `com.dash.swiftexampleapp.keys` services (now consolidated
        // under `org.dashfoundation.wallet`) and the pre-per-wallet
        // `wallet.seed` / `wallet.mnemonic` / `wallet.pin` accounts.
        // Best-effort — failures are silently ignored inside
        // `cleanupLegacyItems` itself.
        WalletStorage.cleanupLegacyItems()

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
                // Rebind wallet-scoped services whenever the set of
                // managed wallets changes — wallet creation, restore
                // on launch, or a full wipe. The Rust manager holds
                // N wallets concurrently; BLAST sync is started once
                // at manager scope and iterates all of them.
                //
                // Keyed on the sorted id set so unrelated republishes
                // of the same dictionary don't retrigger.
                .onChange(of: walletManager.wallets.keys.sorted(by: {
                    $0.lexicographicallyPrecedes($1)
                })) { _, _ in
                    rebindWalletScopedServices()
                }
        }
    }

    /// Drive manager-wide BLAST sync state from the set of loaded
    /// wallets. With no wallets present, sync is stopped and the
    /// per-wallet `PlatformBalanceSyncService` UI surface is reset.
    /// Otherwise, we ensure sync is running and bind the balance
    /// service to `firstWallet` for the single-wallet UI surfaces
    /// that still expect one focused wallet (detail views reconfigure
    /// the service per-wallet themselves).
    @MainActor
    private func rebindWalletScopedServices() {
        guard let wallet = walletManager.firstWallet else {
            do {
                try walletManager.stopPlatformAddressSync()
            } catch {
                SDKLogger.error(
                    "Failed to stop platform address sync: \(error.localizedDescription)"
                )
            }
            platformBalanceSyncService.reset()
            return
        }
        do {
            let platformAddressWallet = try wallet.platformAddressWallet()
            platformBalanceSyncService.configure(
                platformAddressWallet: platformAddressWallet,
                walletManager: walletManager,
                persistenceHandler: walletManager.persistence,
                walletId: wallet.walletId
            )
            if try !walletManager.isPlatformAddressSyncRunning() {
                try walletManager.startPlatformAddressSync()
            }
            SDKLogger.log(
                "🔗 BLAST sync running; balance-sync UI bound to wallet \(wallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined())… (of \(walletManager.wallets.count) loaded)",
                minimumLevel: .medium
            )
        } catch {
            SDKLogger.error(
                "Failed to bind platform address wallet: \(error.localizedDescription)"
            )
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

                // Restore wallets from the persister (SwiftData). If
                // no wallets have been persisted yet this is a no-op.
                // Restored wallets come back watch-only; signing is
                // deferred until the user unlocks via biometric +
                // Keychain-stored mnemonic (future work).
                do {
                    let restored = try walletManager.loadFromPersistor()
                    if !restored.isEmpty {
                        SDKLogger.log(
                            "🔓 Restored \(restored.count) wallet(s) from persister",
                            minimumLevel: .medium
                        )
                    }
                } catch {
                    SDKLogger.error(
                        "Failed to restore wallets from persister: \(error.localizedDescription)"
                    )
                }

                // Initialize shielded pool using first available wallet's data.
                initializeShieldedService()
                rebindWalletScopedServices()
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
        // TODO(platform-wallet): Derive a ZIP32 spending key from
        // the managed wallet. The legacy code path reused the
        // seed bytes stashed on the (now-deleted) HDWallet row;
        // the seed now lives only in the keychain, so a fresh
        // derivation path is needed. For now the shielded
        // service starts empty; it will be re-initialized once
        // the user creates/loads a wallet via `createWallet(...)`.
    }
}
