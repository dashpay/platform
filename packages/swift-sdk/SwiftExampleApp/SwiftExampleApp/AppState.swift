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
        Task.detached { [weak self] in
            do {
                let version = try sdk.refreshProtocolVersion()
                NSLog("✅ AppState: refreshed protocol version to \(version)")
                await MainActor.run {
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

    // MARK: - Startup Diagnostics

    @MainActor
    private func runStartupDiagnostics(sdk: SDK) async {
        NSLog("====== PLATFORM QUERY DIAGNOSTICS (STARTUP) ======")

        // Test data based on WASM SDK examples
        struct TestData {
            static let testIdentityId = "6ZhrNvhzD7Qm1nJhWzvipH9cPRLqBamdnXnKjnrrKA2c"
            static let testIdentityId2 = "HqyuZoKnHRdKP88Tz5L37whXHa27RuLRoQHzGgJGvCdU"
            static let dpnsContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
            static let testPublicKeyHash = "b7e904ce25ed97594e72f7af0e66f298031c1754"
            static let testNonUniquePublicKeyHash = "518038dc858461bcee90478fd994bba8057b7531"
            static let testDocumentType = "domain"
            static let testUsername = "dash"
            static let testTokenId = "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
            static let testContractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
            static let testDocumentId = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
        }

        // Run a few key queries to test connectivity
        let diagnosticQueries: [(name: String, test: @MainActor () async throws -> Any)] = [
            ("Get Platform Status", {
                try await sdk.getStatus()
            }),

            ("Get Total Credits", {
                try await sdk.getTotalCreditsInPlatform()
            }),

            ("Get Identity", {
                try await sdk.identityGet(identityId: TestData.testIdentityId)
            }),

            ("Get DPNS Contract", {
                try await sdk.dataContractGet(id: TestData.dpnsContractId)
            }),

            ("DPNS Check Availability", {
                try await sdk.dpnsCheckAvailability(name: "test-name-\(Int.random(in: 1000...9999))")
            })
        ]

        var successCount = 0
        var failureCount = 0

        for query in diagnosticQueries {
            NSLog("\n🔍 Testing: \(query.name)")

            do {
                let startTime = Date()
                let result = try await query.test()
                let duration = Date().timeIntervalSince(startTime)

                successCount += 1
                NSLog("✅ Success (\(String(format: "%.3fs", duration)))")

                // Print a summary of the result
                if let dict = result as? [String: Any] {
                    if let version = dict["version"] as? String {
                        NSLog("   Platform version: \(version)")
                    } else if let id = dict["id"] as? String {
                        NSLog("   ID: \(id)")
                    } else if let balance = dict["balance"] as? UInt64 {
                        NSLog("   Balance: \(balance)")
                    } else {
                        NSLog("   Result: \(dict.keys.prefix(3).joined(separator: ", "))...")
                    }
                } else if let uint = result as? UInt64 {
                    NSLog("   Value: \(uint)")
                } else if let bool = result as? Bool {
                    NSLog("   Available: \(bool)")
                }

            } catch {
                failureCount += 1
                NSLog("❌ Failed: \(error.localizedDescription)")
            }
        }

        NSLog("\n====== DIAGNOSTIC SUMMARY ======")
        NSLog("Total queries: \(diagnosticQueries.count)")
        NSLog("Successful: \(successCount)")
        NSLog("Failed: \(failureCount)")
        NSLog("Success rate: \(String(format: "%.0f%%", Double(successCount) / Double(diagnosticQueries.count) * 100))")
        NSLog("================================\n")
    }

    @MainActor
    private func runSimpleDiagnostic(sdk: SDK) async {
        var diagnosticReport = "====== SIMPLE DIAGNOSTIC TEST ======\n"
        diagnosticReport += "Date: \(Date())\n\n"

        // Test 1: Get Platform Status
        do {
            diagnosticReport += "Testing: Get Platform Status...\n"
            let status = try await sdk.getStatus()
            diagnosticReport += "✅ Platform Status Success\n"
            let dict = status
            diagnosticReport += "   Version: \(dict["version"] ?? "unknown")\n"
            diagnosticReport += "   Mode: \(dict["mode"] ?? "unknown")\n"
            diagnosticReport += "   QuorumCount: \(dict["quorumCount"] ?? "unknown")\n"
        } catch {
            diagnosticReport += "❌ Platform Status Failed: \(error)\n"
        }

        diagnosticReport += "\n"

        // Test 2: Get Total Credits
        do {
            diagnosticReport += "Testing: Get Total Credits...\n"
            let credits = try await sdk.getTotalCreditsInPlatform()
            diagnosticReport += "✅ Total Credits Success: \(credits)\n"
        } catch {
            diagnosticReport += "❌ Total Credits Failed: \(error)\n"
        }

        diagnosticReport += "\n"

        // Test 3: Check DPNS availability
        do {
            diagnosticReport += "Testing: DPNS Check Availability...\n"
            let name = "test-diagnostic-\(Int.random(in: 1000...9999))"
            let available = try await sdk.dpnsCheckAvailability(name: name)
            diagnosticReport += "✅ DPNS Check Success: name '\(name)' available = \(available)\n"
        } catch {
            diagnosticReport += "❌ DPNS Check Failed: \(error)\n"
        }

        diagnosticReport += "\n====== DIAGNOSTIC COMPLETE ======\n"

        // Write to documents directory
        if let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first {
            let diagnosticPath = documentsPath.appendingPathComponent("diagnostic_report.txt")
            do {
                try diagnosticReport.write(to: diagnosticPath, atomically: true, encoding: .utf8)
                NSLog("Diagnostic report written to: \(diagnosticPath)")
            } catch {
                NSLog("Failed to write diagnostic report: \(error)")
            }
        }

        // Also log to console
        NSLog(diagnosticReport)
    }
}
