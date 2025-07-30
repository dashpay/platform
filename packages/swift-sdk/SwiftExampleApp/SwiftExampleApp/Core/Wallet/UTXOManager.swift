import Foundation
import SwiftData

// MARK: - UTXO Manager

@MainActor
public class UTXOManager: ObservableObject {
    @Published public private(set) var utxos: [HDUTXO] = []
    @Published public private(set) var isLoading = false
    
    private let modelContainer: ModelContainer
    private let walletManager: WalletManager
    
    public init(walletManager: WalletManager, modelContainer: ModelContainer) {
        self.walletManager = walletManager
        self.modelContainer = modelContainer
        
        Task {
            await loadUTXOs()
        }
    }
    
    // MARK: - UTXO Management
    
    public func loadUTXOs() async {
        isLoading = true
        defer { isLoading = false }
        
        do {
            let descriptor = FetchDescriptor<HDUTXO>(
                predicate: #Predicate { !$0.isSpent },
                sortBy: [SortDescriptor(\.amount, order: .reverse)]
            )
            utxos = try modelContainer.mainContext.fetch(descriptor)
        } catch {
            print("Failed to load UTXOs: \(error)")
        }
    }
    
    public func addUTXO(
        txHash: String,
        outputIndex: UInt32,
        amount: UInt64,
        scriptPubKey: Data,
        address: HDAddress,
        blockHeight: Int? = nil
    ) async throws {
        // Check if UTXO already exists
        let existingDescriptor = FetchDescriptor<HDUTXO>(
            predicate: #Predicate { utxo in
                utxo.txHash == txHash && utxo.outputIndex == outputIndex
            }
        )
        
        let existing = try modelContainer.mainContext.fetch(existingDescriptor)
        if !existing.isEmpty {
            // Update existing UTXO
            if let utxo = existing.first {
                utxo.blockHeight = blockHeight
                utxo.isCoinbase = false // Would need to check this properly
            }
        } else {
            // Create new UTXO
            let utxo = HDUTXO(
                txHash: txHash,
                outputIndex: outputIndex,
                amount: amount,
                scriptPubKey: scriptPubKey,
                address: address
            )
            utxo.blockHeight = blockHeight
            
            modelContainer.mainContext.insert(utxo)
            address.utxos.append(utxo)
            address.balance += amount
            address.isUsed = true
            address.lastSeenTime = Date()
        }
        
        try modelContainer.mainContext.save()
        await loadUTXOs()
        
        // Update account balance
        if let account = address.account {
            await walletManager.updateBalance(for: account)
        }
    }
    
    public func markUTXOAsSpent(
        txHash: String,
        outputIndex: UInt32,
        spendingTxHash: String,
        spendingInputIndex: UInt32
    ) async throws {
        let descriptor = FetchDescriptor<HDUTXO>(
            predicate: #Predicate { utxo in
                utxo.txHash == txHash && utxo.outputIndex == outputIndex
            }
        )
        
        let utxos = try modelContainer.mainContext.fetch(descriptor)
        guard let utxo = utxos.first else {
            throw UTXOError.notFound
        }
        
        utxo.isSpent = true
        utxo.spendingTxHash = spendingTxHash
        utxo.spendingInputIndex = spendingInputIndex
        
        // Update address balance
        if let address = utxo.address {
            address.balance = max(0, address.balance - utxo.amount)
        }
        
        try modelContainer.mainContext.save()
        await loadUTXOs()
        
        // Update account balance
        if let account = utxo.address?.account {
            await walletManager.updateBalance(for: account)
        }
    }
    
    // MARK: - Coin Selection
    
    public func selectCoins(
        amount: UInt64,
        feePerKB: UInt64 = 1000,
        account: HDAccount? = nil
    ) throws -> CoinSelection {
        var availableUTXOs = utxos
        
        // Filter by account if specified
        if let account = account {
            availableUTXOs = availableUTXOs.filter { utxo in
                utxo.address?.account?.id == account.id
            }
        }
        
        // Sort by amount (largest first for now)
        availableUTXOs.sort { $0.amount > $1.amount }
        
        var selectedUTXOs: [HDUTXO] = []
        var totalSelected: UInt64 = 0
        var estimatedSize = 10 // Base transaction size
        
        for utxo in availableUTXOs {
            selectedUTXOs.append(utxo)
            totalSelected += utxo.amount
            estimatedSize += 148 // Approximate size per input
            
            // Calculate required amount including fee
            let outputSize = 34 * 2 // Recipient + change
            let totalSize = estimatedSize + outputSize
            let estimatedFee = UInt64(totalSize) * feePerKB / 1000
            let requiredAmount = amount + max(estimatedFee, 1000)
            
            if totalSelected >= requiredAmount {
                break
            }
        }
        
        // Final fee calculation
        let outputSize = 34 * 2 // Recipient + change
        let totalSize = estimatedSize + outputSize
        let fee = UInt64(totalSize) * feePerKB / 1000
        let finalFee = max(fee, 1000)
        
        guard totalSelected >= amount + finalFee else {
            throw UTXOError.insufficientFunds
        }
        
        return CoinSelection(
            utxos: selectedUTXOs,
            totalAmount: totalSelected,
            fee: finalFee,
            change: totalSelected - amount - finalFee
        )
    }
    
    // MARK: - Balance Calculation
    
    public func calculateBalance(for account: HDAccount? = nil) -> Balance {
        var confirmedBalance: UInt64 = 0
        var unconfirmedBalance: UInt64 = 0
        
        let relevantUTXOs = account != nil ? utxos.filter { $0.address?.account?.id == account?.id } : utxos
        
        for utxo in relevantUTXOs {
            if utxo.blockHeight != nil {
                confirmedBalance += utxo.amount
            } else {
                unconfirmedBalance += utxo.amount
            }
        }
        
        return Balance(
            confirmed: confirmedBalance,
            unconfirmed: unconfirmedBalance,
            immature: 0
        )
    }
}

// MARK: - Supporting Types

public struct CoinSelection {
    public let utxos: [HDUTXO]
    public let totalAmount: UInt64
    public let fee: UInt64
    public let change: UInt64
}

// Balance struct is now defined in Balance.swift

public enum UTXOError: LocalizedError {
    case notFound
    case insufficientFunds
    case invalidUTXO
    
    public var errorDescription: String? {
        switch self {
        case .notFound:
            return "UTXO not found"
        case .insufficientFunds:
            return "Insufficient funds"
        case .invalidUTXO:
            return "Invalid UTXO"
        }
    }
}