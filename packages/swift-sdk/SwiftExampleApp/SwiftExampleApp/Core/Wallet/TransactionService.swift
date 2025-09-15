import Foundation
import SwiftData
import SwiftDashSDK

// MARK: - Transaction Service

@MainActor
class TransactionService: ObservableObject {
    @Published public private(set) var transactions: [HDTransaction] = []
    @Published public private(set) var isLoading = false
    @Published public private(set) var isBroadcasting = false
    @Published public private(set) var lastError: Error?
    
    private let walletManager: WalletManager
    private let modelContainer: ModelContainer
    private let spvClient: SwiftDashSDK.SPVClient?
    
    init(
        walletManager: WalletManager,
        modelContainer: ModelContainer,
        spvClient: SwiftDashSDK.SPVClient? = nil
    ) {
        self.walletManager = walletManager
        self.modelContainer = modelContainer
        self.spvClient = spvClient
        
        Task {
            await loadTransactions()
        }
    }
    
    // MARK: - Transaction Creation
    
    func createTransaction(
        to address: String,
        amount: UInt64,
        from account: HDAccount? = nil,
        feePerKB: UInt64 = 1000
    ) async throws -> BuiltTransaction {
        // Route to SDK transaction builder (stubbed for now)
        guard let wallet = walletManager.currentWallet else { throw TransactionError.invalidState }
        let builder = SwiftDashSDK.SDKTransactionBuilder(network: wallet.dashNetwork.sdkNetwork, feePerKB: feePerKB)
        // TODO: integrate coin selection + key derivation via SDK and add inputs/outputs
        _ = builder // silence unused
        throw TransactionError.notSupported("Transaction building is not yet wired to SwiftDashSDK")
    }
    
    // MARK: - Transaction Broadcasting
    
    func broadcastTransaction(_ transaction: BuiltTransaction) async throws {
        guard let _ = spvClient else {
            throw TransactionError.invalidState
        }
        
        isBroadcasting = true
        defer { isBroadcasting = false }
        
        do {
            // Broadcast through SPV
            // TODO: Implement broadcast with new SPV client
            // try await spvClient.broadcastTransaction(transaction.rawTransaction)
            throw TransactionError.broadcastFailed("SPV broadcast not yet implemented")
        } catch {
            lastError = error
            throw TransactionError.broadcastFailed(error.localizedDescription)
        }
    }
    
    // MARK: - Transaction History
    
    public func loadTransactions() async {
        isLoading = true
        defer { isLoading = false }
        
        do {
            let descriptor = FetchDescriptor<HDTransaction>(
                sortBy: [SortDescriptor(\.timestamp, order: .reverse)]
            )
            transactions = try modelContainer.mainContext.fetch(descriptor)
        } catch {
            print("Failed to load transactions: \(error)")
        }
    }
    
    public func processIncomingTransaction(
        txid: String,
        rawTx: Data,
        blockHeight: Int?,
        timestamp: Date = Date()
    ) async throws {
        // Check if transaction already exists
        let existingDescriptor = FetchDescriptor<HDTransaction>(
            predicate: #Predicate { $0.txHash == txid }
        )
        
        let existing = try modelContainer.mainContext.fetch(existingDescriptor)
        if let existingTx = existing.first {
            // Update existing transaction
            existingTx.blockHeight = blockHeight
            existingTx.confirmations = blockHeight != nil ? 1 : 0
            existingTx.isPending = blockHeight == nil
        } else {
            // Create new transaction
            let hdTransaction = HDTransaction(txHash: txid, timestamp: timestamp)
            hdTransaction.rawTransaction = rawTx
            hdTransaction.blockHeight = blockHeight
            hdTransaction.isPending = blockHeight == nil
            hdTransaction.wallet = walletManager.currentWallet
            
            // TODO: Parse transaction to determine type and amount
            // This would require deserializing the transaction and checking outputs
            
            modelContainer.mainContext.insert(hdTransaction)
        }
        
        try modelContainer.mainContext.save()
        await loadTransactions()
    }
    
    // MARK: - SPV Integration
    
    public func syncWithSPV() async throws {
        guard let spvClient = spvClient,
              let wallet = walletManager.currentWallet else {
            return
        }
        
        // Watch all addresses
        for account in wallet.accounts {
            let allAddresses = account.externalAddresses + account.internalAddresses +
                             account.coinJoinAddresses + account.identityFundingAddresses
            
            for address in allAddresses {
                // TODO: Implement watch address with new SPV client
                // try await spvClient.watchAddress(address.address)
                print("Would watch address: \(address.address)")
            }
        }
        
        // Start sync without blocking UI; inherit MainActor to avoid sending non-Sendable captures
        let client = spvClient
        Task(priority: .userInitiated) {
            try? await client.startSync()
        }
    }
    
    // MARK: - Fee Estimation
    
    public func estimateFee(for amount: UInt64, account: HDAccount? = nil) throws -> UInt64 {
        // Placeholder fixed fee until SDK fee estimator is wired
        return 2000
    }
}
