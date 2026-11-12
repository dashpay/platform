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
    /// Tracks the adaptive provider's current routing choice after the saved
    /// mode is applied. Drives the app's indicator;
    /// intentionally mirrors `SDK.quorumSource` (this one is the observable the
    /// UI binds to).
    @Published private(set) var quorumSource: SDK.QuorumSource = .trusted

    /// Changes only with the SDK, through `publish(_:for:)`.
    @Published private(set) var currentNetwork: Network {
        didSet {
            UserDefaults.standard.set(Int(currentNetwork.rawValue), forKey: "currentNetwork")
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

    /// Monotonic tick bumped after `rebuildSdk` moves the wallet manager
    /// onto a rebuilt SDK and publishes it: a network switch, the Docker
    /// toggle or a devnet edit. SwiftExampleAppApp observes it to re-run
    /// `rebindWalletScopedServices()` against the new manager. A
    /// same-network rebuild changes neither the network nor the wallet ID
    /// set, so without the tick `PlatformBalanceSyncService` and
    /// `ShieldedService` would keep the old manager.
    @Published private(set) var walletScopedServicesRebindTick: Int = 0

    /// Whether an SDK rebuild is running, started from any window. Options
    /// disables its rebuild controls on it; `rebuildSdk` refuses a second
    /// rebuild by itself.
    @Published private(set) var isRebuildingSdk = false

    @Published var useDockerSetup: Bool {
        didSet {
            UserDefaults.standard.set(useDockerSetup, forKey: "useDockerSetup")
            // Write to legacy keys so SDK.swift and SPVClient.swift pick them up
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhostPlatform")
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhostCore")
            UserDefaults.standard.set(useDockerSetup, forKey: "useLocalhost")
            // The SDK reads these keys when it is built. OptionsView's Docker
            // toggle rebuilds it, and moves the wallet manager onto it, after
            // changing this.
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

    /// Which quorum source the user wants. Every change is applied to every
    /// SDK this object built (see `applyQuorumMode`); `quorumSource` reflects
    /// the adaptive provider's current routing choice (SPV may not be ready
    /// yet).
    @Published var quorumMode: QuorumMode {
        didSet {
            UserDefaults.standard.set(quorumMode.rawValue, forKey: "quorumMode")
            applyQuorumMode()
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

    /// Every SDK `makeSdk` built that is still alive: the published one, one a
    /// rebuild is moving the wallet manager onto, and the wallet managers'
    /// background SDKs. `applyQuorumMode` applies the user's mode to all of
    /// them.
    private let builtSdks = NSHashTable<SDK>.weakObjects()

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
                let newSDK = try makeSdk(for: currentNetwork)
                sdk = newSDK
                quorumSource = newSDK.quorumSource
                NSLog("✅ AppState: SDK created successfully")

                // Eagerly learn the network's protocol version so
                // fee-sensitive flows reserve correctly before the
                // first metadata-bearing response ratchets the SDK.
                refreshProtocolVersion(for: newSDK)

                isLoading = false
            } catch {
                sdk = nil
                showError(message: "Failed to initialize SDK: \(error.localizedDescription)")
                NSLog("❌ AppState.initializeSDK: \(error)")
                isLoading = false
            }
        }
    }

    /// Apply the current policy to the adaptive provider of every SDK
    /// `makeSdk` built that is still alive, the wallet managers' background
    /// SDKs included, so none keeps a mode the user has left, then refresh
    /// the active-source indicator. Runs on every `quorumMode` change;
    /// `makeSdk` applies the mode to an SDK it builds.
    ///
    /// - `.auto`: SPV while it runs with the header + masternode lists fully
    ///   synced, trusted otherwise.
    /// - `.spv`: SPV unconditionally — installed even before the SPV client
    ///   starts. The lookup fails closed ("SPV Client not started" / proofs
    ///   fail until synced) rather than falling back to trusted.
    /// - `.trusted`: always the trusted HTTP quorum provider.
    ///
    /// `quorumSource` tracks the provider's current routing choice, which may
    /// lag the requested mode while SPV isn't ready.
    private func applyQuorumMode() {
        for builtSdk in builtSdks.allObjects {
            do {
                try builtSdk.setQuorumMode(sdkQuorumMode)
            } catch {
                NSLog("⚠️ AppState: failed to apply quorum mode: \(error.localizedDescription)")
            }
        }
        refreshQuorumSource()
    }

    /// Read the published SDK's current quorum source into `quorumSource`.
    /// In Auto the adaptive provider follows SPV readiness by itself, so when
    /// SPV progress or running state changes only the indicator needs
    /// refreshing, not the mode.
    func refreshQuorumSource() {
        guard let sdk else { return }
        // Called on every SPV progress tick. Assigning an unchanged value
        // still publishes, and every view observing this object, the root
        // tab view among them, would re-render with each tick.
        let source = sdk.quorumSource
        if quorumSource != source {
            quorumSource = source
        }
    }

    func showError(message: String) {
        errorMessage = message
        showError = true
    }

    /// Rebuild the SDK for `target` through `rebuildSdkAndMoveWalletManager`
    /// (see it for the steps and the rollback), or refuse with `.busy`,
    /// running none of the steps, while another rebuild runs.
    ///
    /// Every window shares this object, and a rebuild changes shared
    /// settings, the published SDK and network, and the active wallet
    /// manager: a second rebuild beside the first would roll its prepared
    /// settings back over the first one's, or publish its network in
    /// whichever order the two finish. The rebuild is reserved before its
    /// first suspension and released after its publish or rollback. This is
    /// the only way the SDK and its network change after the bootstrap.
    ///
    /// The steps this object does not own are injected: `stopSpv` (the
    /// wallet manager's), `prepare` and `rollBack` (the settings the flow
    /// changes), `activateManager` (the wallet manager store's) and
    /// `beforePublish` (the flow's own state).
    func rebuildSdk(
        from network: Network,
        to target: Network,
        stopSpv: @MainActor () async -> Bool,
        prepare: @MainActor () -> Void,
        activateManager: @MainActor (Network, SDK) async -> Bool,
        rollBack: @MainActor () -> Void,
        beforePublish: @MainActor () -> Void
    ) async -> SdkRebuildOutcome {
        guard !isRebuildingSdk else {
            SDKLogger.event(
                "sdk_rebuild_refused",
                category: .lifecycle,
                fields: ["network": .publicText(target.displayName)]
            )
            return .busy
        }
        isRebuildingSdk = true
        defer { isRebuildingSdk = false }
        return await rebuildSdkAndMoveWalletManager(
            from: network,
            to: target,
            stopSpv: stopSpv,
            currentNetwork: { self.currentNetwork },
            prepare: prepare,
            buildSdk: { self.buildSdk(for: $0) },
            activateManager: activateManager,
            rollBack: rollBack,
            publish: { (target: Network, sdk: SDK) in
                beforePublish()
                self.publish(sdk, for: target)
            },
            rebindWalletScopedServices: { self.walletScopedServicesRebindTick &+= 1 }
        )
    }

    /// Build an SDK for `network` the way this app builds every SDK, without
    /// publishing it: one adaptive provider, the quorum mode the user picked,
    /// and the stored contracts. The bootstrap, SDK rebuilds and the wallet
    /// managers' background SDKs all build here, and `applyQuorumMode`
    /// reaches every SDK built here while it lives.
    func makeSdk(for network: Network) throws -> SDK {
        let newSDK = try SDK(network: network, contextProvider: .adaptive)
        // Apply the mode before any proven query can run on this SDK. With
        // no SPV source yet, strict SPV fails closed until a wallet manager
        // serves one.
        try newSDK.setQuorumMode(sdkQuorumMode)
        if let modelContext {
            loadKnownContractsIntoSDK(sdk: newSDK, modelContext: modelContext)
        }
        builtSdks.add(newSDK)
        return newSDK
    }

    /// Build an SDK for a rebuild with `makeSdk`. Returns `nil` after showing
    /// the failure.
    private func buildSdk(for network: Network) -> SDK? {
        isLoading = true
        defer { isLoading = false }
        do {
            return try makeSdk(for: network)
        } catch {
            showError(
                message: "Could not build the SDK for \(network.displayName), so the app stays on its current SDK. SPV stays stopped until you restart it from the Wallet tab. \(error.localizedDescription)"
            )
            NSLog("❌ AppState.buildSdk: \(error)")
            return nil
        }
    }

    /// Make `sdk`, built for `network`, the current SDK and `network` the
    /// current network, together. The `publish` step of `rebuildSdk`, after
    /// the wallet manager moved onto `sdk`, so the selected network, the SDK
    /// and the manager change in one main-actor turn.
    private func publish(_ sdk: SDK, for network: Network) {
        // Identities, contracts, documents and token balances are scoped
        // per network inside SwiftData; `@Query` consumers filter by
        // `network` and follow the DataManager's scope.
        dataManager?.currentNetwork = network
        currentNetwork = network
        self.sdk = sdk
        quorumSource = sdk.quorumSource
        // Eagerly learn the network's protocol version (see
        // `initializeSDK`). Non-fatal: the SDK still ratchets from metadata
        // if this fails.
        refreshProtocolVersion(for: sdk)
    }

    private var sdkQuorumMode: SDK.QuorumMode {
        switch quorumMode {
        case .auto: return .auto
        case .spv: return .spv
        case .trusted: return .trusted
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

    private func loadKnownContractsIntoSDK(sdk: SDK, modelContext: ModelContext) {
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
                    } else if let balance = UInt64(jsonValue: dict["balance"]) {
                        // Credits are a protocol `u64`: DPP writes one above
                        // 2^53 - 1 as a decimal string, which a bare cast
                        // dropped to the generic "Result:" line below.
                        NSLog("   Balance: \(balance)")
                    } else {
                        NSLog("   Result: \(dict.keys.prefix(3).joined(separator: ", "))...")
                    }
                } else if let uint = UInt64(jsonValue: result) {
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
