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
    
    // Services from Core
    let walletService: WalletService
    
    // State from Platform
    let platformState: AppState
    
    // Unified state manager
    let unifiedState: UnifiedStateManager
    
    // SwiftData container
    let modelContainer: ModelContainer
    
    // Transition state for temporary data
    @Published var transitionState = TransitionState()
    
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
        self.walletService = WalletService.shared
        self.walletService.configure(modelContainer: modelContainer)
        
        self.platformState = AppState()
        
        // Initialize unified state (will be updated with real SDKs during async init)
        self.unifiedState = UnifiedStateManager()
    }
    
    func initialize() async {
        do {
            // Initialize Platform SDK
            await MainActor.run {
                platformState.initializeSDK(modelContext: modelContainer.mainContext)
            }
            
            // Wait for Platform SDK to be ready
            try? await Task.sleep(nanoseconds: 500_000_000) // 0.5 second
            
            isInitialized = true
        } catch {
            self.error = error
        }
    }
    
    func reset() async {
        isInitialized = false
        error = nil
        
        // Reset services
        await walletService.stopSync()
        
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
}