import Foundation
import SwiftData
import SwiftDashSDK

@MainActor
class AppState: ObservableObject {
    @Published var sdk: SDK?
    @Published var isLoading = false
    @Published var showError = false
    @Published var errorMessage = ""

    // Identity, contract, document, and token state is now read
    // directly from SwiftData via `@Query` on the
    // `PersistentIdentity`, `PersistentDataContract`,
    // `PersistentDocument`, and `PersistentToken` /
    // `PersistentTokenBalance` models. AppState no longer mirrors
    // any of them as `@Published` arrays.
    @Published var dataContracts: [DPPDataContract] = []

    @Published var currentNetwork: AppNetwork {
        didSet {
            UserDefaults.standard.set(currentNetwork.rawValue, forKey: "currentNetwork")
            Task {
                await switchNetwork(to: currentNetwork)
            }
        }
    }

    @Published var dataStatistics: (identities: Int, documents: Int, contracts: Int, tokenBalances: Int)?

    @Published var useLocalPlatform: Bool {
        didSet {
            UserDefaults.standard.set(useLocalPlatform, forKey: "useLocalhostPlatform")
            // Maintain backward-compat key for older SDK builds
            UserDefaults.standard.set(useLocalPlatform, forKey: "useLocalhost")
            Task { await switchNetwork(to: currentNetwork) }
        }
    }

    @Published var useLocalCore: Bool {
        didSet {
            UserDefaults.standard.set(useLocalCore, forKey: "useLocalhostCore")
            // TODO: Reconfigure SPV client peers when supported
        }
    }

    private let testSigner = TestSigner()
    private var dataManager: DataManager?
    private var modelContext: ModelContext?

    init() {
        // Load saved network preference or use default
        if let savedNetwork = UserDefaults.standard.string(forKey: "currentNetwork"),
           let network = AppNetwork(rawValue: savedNetwork) {
            self.currentNetwork = network
        } else {
            self.currentNetwork = .testnet
        }
        // Migration: if legacy key set and new keys absent, propagate
        let legacyLocal = UserDefaults.standard.bool(forKey: "useLocalhost")
        let hasPlatformKey = UserDefaults.standard.object(forKey: "useLocalhostPlatform") != nil
        let hasCoreKey = UserDefaults.standard.object(forKey: "useLocalhostCore") != nil
        self.useLocalPlatform = hasPlatformKey ? UserDefaults.standard.bool(forKey: "useLocalhostPlatform") : legacyLocal
        self.useLocalCore = hasCoreKey ? UserDefaults.standard.bool(forKey: "useLocalhostCore") : legacyLocal
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
                // Initialize the SDK library
                SDK.initialize()

                // Enable debug logging to see gRPC endpoints
                SDK.enableLogging(level: .debug)
                NSLog("🔵 AppState: Enabled debug logging for gRPC requests")

                NSLog("🔵 AppState: Creating SDK instance for network: \(currentNetwork)")
                // Create SDK instance for current network
                let sdkNetwork: DashSDKNetwork = currentNetwork.sdkNetwork
                NSLog("🔵 AppState: SDK network value: \(sdkNetwork)")

                let newSDK = try SDK(network: sdkNetwork)
                sdk = newSDK
                NSLog("✅ AppState: SDK created successfully with handle: \(newSDK.handle != nil ? "exists" : "nil")")

                // Load known contracts into the SDK's trusted provider
                await loadKnownContractsIntoSDK(sdk: newSDK, modelContext: modelContext)

                // Load persisted data first
                await loadPersistedData()

                isLoading = false
            } catch {
                showError(message: "Failed to initialize SDK: \(error.localizedDescription)")
                isLoading = false
            }
        }
    }

    func loadPersistedData() async {
        // No-op: identities, contracts, and documents are sourced
        // directly from SwiftData via @Query in each view. Kept as a
        // stub for call-site parity until all initializers migrate
        // to reading from SwiftData directly.
        _ = dataManager
    }

    func showError(message: String) {
        errorMessage = message
        showError = true
    }

    func switchNetwork(to network: AppNetwork) async {
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
            let sdkNetwork: DashSDKNetwork = network.sdkNetwork
            let newSDK = try SDK(network: sdkNetwork)
            sdk = newSDK

            // Load known contracts into the SDK's trusted provider
            await loadKnownContractsIntoSDK(sdk: newSDK, modelContext: modelContext)

            // Reload data for the new network
            await loadPersistedData()

            isLoading = false
        } catch {
            showError(message: "Failed to switch network: \(error.localizedDescription)")
            isLoading = false
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
