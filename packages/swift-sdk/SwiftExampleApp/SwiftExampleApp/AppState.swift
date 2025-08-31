import Foundation
import SwiftData
import SwiftDashSDK

@MainActor
class AppState: ObservableObject {
    @Published var sdk: SDK?
    @Published var isLoading = false
    @Published var showError = false
    @Published var errorMessage = ""
    
    @Published var identities: [IdentityModel] = []
    @Published var contracts: [ContractModel] = []
    @Published var tokens: [TokenModel] = []
    @Published var documents: [DocumentModel] = []
    @Published var dataContracts: [DPPDataContract] = []
    
    @Published var currentNetwork: Network {
        didSet {
            UserDefaults.standard.set(currentNetwork.rawValue, forKey: "currentNetwork")
            Task {
                await switchNetwork(to: currentNetwork)
            }
        }
    }
    
    @Published var dataStatistics: (identities: Int, documents: Int, contracts: Int, tokenBalances: Int)?
    
    private let testSigner = TestSigner()
    private var dataManager: DataManager?
    private var modelContext: ModelContext?
    
    init() {
        // Load saved network preference or use default
        if let savedNetwork = UserDefaults.standard.string(forKey: "currentNetwork"),
           let network = Network(rawValue: savedNetwork) {
            self.currentNetwork = network
        } else {
            self.currentNetwork = .testnet
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
                // Initialize the SDK library
                SDK.initialize()
                
                // Enable debug logging to see gRPC endpoints
                SDK.enableLogging(level: .debug)
                NSLog("🔵 AppState: Enabled debug logging for gRPC requests")
                
                NSLog("🔵 AppState: Creating SDK instance for network: \(currentNetwork)")
                // Create SDK instance for current network
                let sdkNetwork = currentNetwork.sdkNetwork
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
        guard let dataManager = dataManager else { return }
        
        do {
            // Load identities
            identities = try dataManager.fetchIdentities()
            
            // Load contracts
            contracts = try dataManager.fetchContracts()
            
            // Load documents for all contracts
            var allDocuments: [DocumentModel] = []
            for contract in contracts {
                let docs = try dataManager.fetchDocuments(contractId: contract.id)
                allDocuments.append(contentsOf: docs)
            }
            documents = allDocuments
            
            // TODO: Load tokens from contracts with token support
        } catch {
            print("Error loading persisted data: \(error)")
        }
    }
    
    func loadSampleIdentities() async {
        guard let dataManager = dataManager else { return }
        
        // Add some sample local identities for testing
        let sampleIdentities = [
            IdentityModel(
                idString: "1111111111111111111111111111111111111111111111111111111111111111",
                balance: 1000000000,
                isLocal: true,
                alias: "Alice"
            ),
            IdentityModel(
                idString: "2222222222222222222222222222222222222222222222222222222222222222",
                balance: 500000000,
                isLocal: true,
                alias: "Bob"
            ),
            IdentityModel(
                idString: "3333333333333333333333333333333333333333333333333333333333333333",
                balance: 250000000,
                isLocal: true,
                alias: "Charlie"
            )
        ].compactMap { $0 }
        
        // Save to persistence
        for identity in sampleIdentities {
            do {
                try dataManager.saveIdentity(identity)
            } catch {
                print("Error saving sample identity: \(error)")
            }
        }
        
        // Update published array
        identities = sampleIdentities
    }
    
    func showError(message: String) {
        errorMessage = message
        showError = true
    }
    
    func switchNetwork(to network: Network) async {
        guard let modelContext = modelContext else { return }
        
        // Clear current data
        identities.removeAll()
        contracts.removeAll()
        documents.removeAll()
        tokens.removeAll()
        
        // Update DataManager's current network
        dataManager?.currentNetwork = network
        
        // Re-initialize SDK with new network
        do {
            isLoading = true
            
            // Create new SDK instance for the network
            let sdkNetwork = network.sdkNetwork
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
    
    func addIdentity(_ identity: IdentityModel, walletId: Data? = nil) {
        guard let dataManager = dataManager else { return }
        
        var updatedIdentity = identity
        if let walletId = walletId {
            updatedIdentity.walletId = walletId
        }
        
        if !identities.contains(where: { $0.id == identity.id }) {
            identities.append(updatedIdentity)
            
            // Save to persistence
            Task {
                do {
                    try dataManager.saveIdentity(updatedIdentity)
                } catch {
                    print("Error saving identity: \(error)")
                }
            }
        }
    }
    
    func updateIdentity(_ identity: IdentityModel) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == identity.id }) {
            identities[index] = identity
            
            // Save to persistence
            Task {
                do {
                    try dataManager.saveIdentity(identity)
                } catch {
                    print("Error updating identity: \(error)")
                }
            }
        }
    }
    
    func removeIdentity(_ identity: IdentityModel) {
        guard let dataManager = dataManager else { return }
        
        identities.removeAll { $0.id == identity.id }
        
        // Remove from persistence
        Task {
            do {
                try dataManager.deleteIdentity(withId: identity.id)
            } catch {
                print("Error deleting identity: \(error)")
            }
        }
    }
    
    func associateIdentityWithWallet(identityId: Data, walletId: Data) {
        guard let dataManager = dataManager else { return }
        
        // Find and update the identity
        if let index = identities.firstIndex(where: { $0.id == identityId }) {
            identities[index].walletId = walletId
            
            // Update persistence
            Task {
                do {
                    try dataManager.saveIdentity(identities[index])
                } catch {
                    print("Error updating identity wallet association: \(error)")
                }
            }
        }
    }
    
    func updateIdentityBalance(id: Data, newBalance: UInt64) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            var identity = identities[index]
            identity.balance = newBalance
            identities[index] = identity
            
            // Update in persistence
            Task {
                do {
                    try dataManager.saveIdentity(identity)
                } catch {
                    print("Error updating identity balance: \(error)")
                }
            }
        }
    }
    
    func updateIdentityDPNSName(id: Data, dpnsName: String) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            var identity = identities[index]
            identity.dpnsName = dpnsName
            identities[index] = identity
            
            // Update in persistence
            Task {
                do {
                    try dataManager.saveIdentity(identity)
                } catch {
                    print("Error updating identity DPNS name: \(error)")
                }
            }
        }
    }
    
    func updateIdentityMainName(id: Data, mainName: String?) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            let oldIdentity = identities[index]
            let updatedIdentity = IdentityModel(
                id: oldIdentity.id,
                balance: oldIdentity.balance,
                isLocal: oldIdentity.isLocal,
                alias: oldIdentity.alias,
                type: oldIdentity.type,
                privateKeys: oldIdentity.privateKeys,
                votingPrivateKey: oldIdentity.votingPrivateKey,
                ownerPrivateKey: oldIdentity.ownerPrivateKey,
                payoutPrivateKey: oldIdentity.payoutPrivateKey,
                dpnsName: oldIdentity.dpnsName,
                mainDpnsName: mainName,
                dpnsNames: oldIdentity.dpnsNames,
                contestedDpnsNames: oldIdentity.contestedDpnsNames,
                contestedDpnsInfo: oldIdentity.contestedDpnsInfo,
                publicKeys: oldIdentity.publicKeys
            )
            identities[index] = updatedIdentity
            
            // Update in persistence
            Task {
                do {
                    try dataManager.saveIdentity(updatedIdentity)
                } catch {
                    print("Error updating identity main name: \(error)")
                }
            }
        }
    }
    
    func updateIdentityDPNSNames(id: Data, dpnsNames: [String], contestedNames: [String], contestedInfo: [String: Any]) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            var identity = identities[index]
            identity.dpnsNames = dpnsNames
            identity.contestedDpnsNames = contestedNames
            identity.contestedDpnsInfo = contestedInfo
            
            // Set the primary dpnsName if we have registered names
            if !dpnsNames.isEmpty && identity.dpnsName == nil {
                identity.dpnsName = dpnsNames.first
            }
            
            identities[index] = identity
            
            // Update in persistence
            Task {
                do {
                    try dataManager.saveIdentity(identity)
                } catch {
                    print("Error updating identity DPNS names: \(error)")
                }
            }
        }
    }
    
    func removePrivateKeyReference(identityId: Data, keyId: Int32) {
        guard let dataManager = dataManager else { return }
        
        Task {
            do {
                try dataManager.removePrivateKeyReference(identityId: identityId, keyId: keyId)
            } catch {
                print("Error removing private key reference: \(error)")
            }
        }
    }
    
    func updateIdentityPublicKeys(id: Data, publicKeys: [IdentityPublicKey]) {
        print("🔵 updateIdentityPublicKeys called with \(publicKeys.count) keys for identity \(id.toHexString())")
        guard let dataManager = dataManager else { 
            print("❌ No dataManager available")
            return 
        }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            print("🔵 Found identity at index \(index)")
            // Create a new identity with updated public keys
            let oldIdentity = identities[index]
            let updatedIdentity = IdentityModel(
                id: oldIdentity.id,
                balance: oldIdentity.balance,
                isLocal: oldIdentity.isLocal,
                alias: oldIdentity.alias,
                type: oldIdentity.type,
                privateKeys: oldIdentity.privateKeys,
                votingPrivateKey: oldIdentity.votingPrivateKey,
                ownerPrivateKey: oldIdentity.ownerPrivateKey,
                payoutPrivateKey: oldIdentity.payoutPrivateKey,
                dpnsName: oldIdentity.dpnsName,
                mainDpnsName: oldIdentity.mainDpnsName,
                dpnsNames: oldIdentity.dpnsNames,
                contestedDpnsNames: oldIdentity.contestedDpnsNames,
                contestedDpnsInfo: oldIdentity.contestedDpnsInfo,
                publicKeys: publicKeys
            )
            identities[index] = updatedIdentity
            print("🔵 Updated identity in array, now has \(updatedIdentity.publicKeys.count) public keys")
            
            // Update in persistence
            Task {
                do {
                    try dataManager.saveIdentity(updatedIdentity)
                    print("✅ Saved identity to persistence")
                } catch {
                    print("Error updating identity public keys: \(error)")
                }
            }
        } else {
            print("❌ Identity not found in identities array")
        }
    }
    
    func addContract(_ contract: ContractModel) {
        guard let dataManager = dataManager else { return }
        
        if !contracts.contains(where: { $0.id == contract.id }) {
            contracts.append(contract)
            
            // Save to persistence
            Task {
                do {
                    try dataManager.saveContract(contract)
                } catch {
                    print("Error saving contract: \(error)")
                }
            }
        }
    }
    
    func addDocument(_ document: DocumentModel) {
        guard let dataManager = dataManager else { return }
        
        if !documents.contains(where: { $0.id == document.id }) {
            documents.append(document)
            
            // Save to persistence
            Task {
                do {
                    try dataManager.saveDocument(document)
                } catch {
                    print("Error saving document: \(error)")
                }
            }
        }
    }
    
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
        let diagnosticQueries: [(name: String, test: () async throws -> Any)] = [
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
    
    private func runSimpleDiagnostic(sdk: SDK) async {
        var diagnosticReport = "====== SIMPLE DIAGNOSTIC TEST ======\n"
        diagnosticReport += "Date: \(Date())\n\n"
        
        // Test 1: Get Platform Status
        do {
            diagnosticReport += "Testing: Get Platform Status...\n"
            let status = try await sdk.getStatus()
            diagnosticReport += "✅ Platform Status Success\n"
            if let dict = status as? [String: Any] {
                diagnosticReport += "   Version: \(dict["version"] ?? "unknown")\n"
                diagnosticReport += "   Mode: \(dict["mode"] ?? "unknown")\n"
                diagnosticReport += "   QuorumCount: \(dict["quorumCount"] ?? "unknown")\n"
            }
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