import Foundation
import SwiftData
import SwiftDashSDK
import CSwiftDashSDK

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
    
    private let testSigner = TestSigner()
    private var dataManager: DataManager?
    
    func initializeSDK(modelContext: ModelContext) {
        // Initialize DataManager
        self.dataManager = DataManager(modelContext: modelContext)
        
        Task {
            do {
                isLoading = true
                
                // Initialize the SDK library
                SDK.initialize()
                
                // Create SDK instance for testnet
                let newSDK = try SDK(network: SwiftDashSwiftDashNetwork(rawValue: 1))
                sdk = newSDK
                
                // Load persisted data first
                await loadPersistedData()
                
                // If no identities exist, load sample identities
                if identities.isEmpty {
                    await loadSampleIdentities()
                }
                
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
                id: "11111111111111111111111111111111",
                balance: 1000000000,
                isLocal: true,
                alias: "Alice"
            ),
            IdentityModel(
                id: "22222222222222222222222222222222",
                balance: 500000000,
                isLocal: true,
                alias: "Bob"
            ),
            IdentityModel(
                id: "33333333333333333333333333333333",
                balance: 250000000,
                isLocal: true,
                alias: "Charlie"
            )
        ]
        
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
    
    func updateIdentityBalance(id: String, newBalance: UInt64) {
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