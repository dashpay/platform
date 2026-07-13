import Foundation
import SwiftData
import SwiftDashSDK

@MainActor
class AppState: ObservableObject {
    @Published var sdk: SDK?
    @Published var isLoading = false
    @Published var showError = false
    @Published var errorMessage = ""

    /// The quorum-key source the current SDK uses for proof verification.
    /// Starts `.trusted` on every SDK build and flips to `.spv` once
    /// `applyQuorumMode` installs the SPV provider. Drives the app's indicator;
    /// intentionally mirrors `SDK.quorumSource` (this one is the observable the
    /// UI binds to).
    @Published private(set) var quorumSource: SDK.QuorumSource = .trusted

    @Published var currentNetwork: Network {
        didSet {
            UserDefaults.standard.set(Int(currentNetwork.rawValue), forKey: "currentNetwork")
            Task {
                await switchNetwork(to: currentNetwork)
            }
        }
    }

    @Published var dataStatistics: (identities: Int, documents: Int, contracts: Int, tokenBalances: Int)?

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

    /// User-selected policy for which quorum source proof verification uses.
    enum QuorumMode: String, CaseIterable, Identifiable {
        /// Trusted until the SPV masternode list is synced, then SPV.
        case auto
        /// Force SPV as soon as the SPV client is running (proofs fail closed
        /// until synced; no trusted fallback).
        case spv
        /// Force the trusted HTTP quorum provider.
        case trusted

        var id: String { rawValue }
        var label: String {
            switch self {
            case .auto: return "Auto"
            case .spv: return "SPV"
            case .trusted: return "Trusted"
            }
        }
    }

    /// Which quorum source the user wants. Applied live on change (see
    /// `applyQuorumMode`); `quorumSource` reflects what is *actually* installed
    /// (SPV may not be ready yet).
    @Published var quorumMode: QuorumMode {
        didSet {
            UserDefaults.standard.set(quorumMode.rawValue, forKey: "quorumMode")
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
        // Default: Auto — trusted until SPV syncs, then SPV.
        self.quorumMode =
            UserDefaults.standard.string(forKey: "quorumMode").flatMap(QuorumMode.init(rawValue:))
            ?? .auto
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
                // Build with the trusted quorum provider. Proof verification is
                // switched to SPV-synced quorums later via `applyQuorumMode`,
                // once the active wallet manager's SPV masternode list is synced
                // (the manager can only be created *from* an SDK, so the SDK
                // must exist first — hence attach-after rather than build-from).
                let newSDK = try SDK(network: currentNetwork)
                sdk = newSDK
                quorumSource = .trusted
                NSLog("✅ AppState: SDK created successfully")

                // Eagerly learn the network's protocol version so
                // fee-sensitive flows reserve correctly before the
                // first metadata-bearing response ratchets the SDK.
                refreshProtocolVersion(for: newSDK)

                // Load known contracts into the SDK's provider cache.
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

    /// Install the quorum source the current `quorumMode` calls for, via a live
    /// provider swap on the shared slot (no SDK rebuild). Idempotent. Call
    /// whenever `quorumMode` or the manager's `spvProgress` changes.
    ///
    /// - `.auto`: SPV once the header + masternode lists are fully synced,
    ///   trusted until then.
    /// - `.spv`: SPV as soon as the SPV client is running (proofs fail closed
    ///   until synced; no trusted fallback). Stays trusted if SPV isn't running.
    /// - `.trusted`: always the trusted HTTP quorum provider.
    ///
    /// `quorumSource` tracks what is *actually* installed, which may lag the
    /// requested mode while SPV isn't ready.
    @MainActor
    func applyQuorumMode(manager: PlatformWalletManager) {
        guard let sdk else { return }

        let progress = manager.spvProgress
        let running =
            progress.overallState == .syncing || progress.overallState == .synced
        let fullySynced =
            progress.headers?.state == .synced && progress.masternodes?.state == .synced

        let wantSpv: Bool
        switch quorumMode {
        case .auto: wantSpv = fullySynced
        case .spv: wantSpv = running
        case .trusted: wantSpv = false
        }

        do {
            if wantSpv, quorumSource == .trusted {
                try sdk.attachSpvQuorums(from: manager)
                quorumSource = .spv
                NSLog("✅ AppState: proof verification now uses SPV-synced quorums")
            } else if !wantSpv, quorumSource == .spv {
                try sdk.restoreTrustedQuorums()
                quorumSource = .trusted
                NSLog("↩️ AppState: proof verification reverted to trusted quorums")
            }
        } catch {
            NSLog("⚠️ AppState: failed to apply quorum mode: \(error.localizedDescription)")
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

            // Create new SDK instance for the network (trusted provider; SPV is
            // re-attached by `applyQuorumMode` once the new network's manager
            // has synced).
            let newSDK = try SDK(network: network)
            sdk = newSDK
            quorumSource = .trusted

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
        Task.detached {
            do {
                let version = try sdk.refreshProtocolVersion()
                NSLog("✅ AppState: refreshed protocol version to \(version)")
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
