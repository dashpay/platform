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
                
                // Initialize the SDK library
                SDK.initialize()
                
                // Create SDK instance for current network
                let sdkNetwork = currentNetwork.sdkNetwork
                let newSDK = try SDK(network: sdkNetwork)
                sdk = newSDK
                
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
            
            // Reload data for the new network
            await loadPersistedData()
            
            isLoading = false
        } catch {
            showError(message: "Failed to switch network: \(error.localizedDescription)")
            isLoading = false
        }
    }
    
    func addIdentity(_ identity: IdentityModel) {
        guard let dataManager = dataManager else { return }
        
        if !identities.contains(where: { $0.id == identity.id }) {
            identities.append(identity)
            
            // Save to persistence
            Task {
                do {
                    try dataManager.saveIdentity(identity)
                } catch {
                    print("Error saving identity: \(error)")
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
    
    func updateIdentityBalance(id: Data, newBalance: UInt64) {
        guard let dataManager = dataManager else { return }
        
        if let index = identities.firstIndex(where: { $0.id == id }) {
            var identity = identities[index]
            identity = IdentityModel(
                id: identity.id,
                balance: newBalance,
                isLocal: identity.isLocal,
                alias: identity.alias,
                type: identity.type,
                privateKeys: identity.privateKeys,
                votingPrivateKey: identity.votingPrivateKey,
                ownerPrivateKey: identity.ownerPrivateKey,
                payoutPrivateKey: identity.payoutPrivateKey,
                dppIdentity: identity.dppIdentity,
                publicKeys: identity.publicKeys
            )
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