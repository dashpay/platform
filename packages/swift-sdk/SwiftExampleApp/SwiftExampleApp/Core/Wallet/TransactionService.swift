import Foundation
import SwiftData

// MARK: - Transaction Service

@MainActor
public class TransactionService: ObservableObject {
    @Published public private(set) var transactions: [HDTransaction] = []
    @Published public private(set) var isLoading = false
    @Published public private(set) var isBroadcasting = false
    @Published public private(set) var lastError: Error?
    
    private let walletManager: WalletManager
    private let utxoManager: UTXOManager
    private let modelContainer: ModelContainer
    private let spvClient: SPVClient?
    
    public init(
        walletManager: WalletManager,
        utxoManager: UTXOManager,
        modelContainer: ModelContainer,
        spvClient: SPVClient? = nil
    ) {
        self.walletManager = walletManager
        self.utxoManager = utxoManager
        self.modelContainer = modelContainer
        self.spvClient = spvClient
        
        Task {
            await loadTransactions()
        }
    }
    
    // MARK: - Transaction Creation
    
    public func createTransaction(
        to address: String,
        amount: UInt64,
        from account: HDAccount? = nil,
        feePerKB: UInt64 = 1000
    ) async throws -> BuiltTransaction {
        guard let wallet = walletManager.currentWallet else {
            throw TransactionError.noWallet
        }
        
        // Select coins
        let coinSelection = try utxoManager.selectCoins(
            amount: amount,
            feePerKB: feePerKB,
            account: account ?? wallet.accounts.first
        )
        
        // Get change address
        let changeAddress = try await walletManager.getUnusedAddress(
            for: account ?? wallet.accounts[0],
            type: .internal
        )
        
        // Build transaction
        let builder = TransactionBuilder(network: wallet.dashNetwork, feePerKB: feePerKB)
        try builder.setChangeAddress(changeAddress.address)
        
        // Add inputs with private keys
        for utxo in coinSelection.utxos {
            guard let address = utxo.address,
                  let account = address.account else {
                throw TransactionError.invalidInput("UTXO missing address or account")
            }
            
            // Derive private key for the address
            guard let seed = walletManager.decryptSeed(wallet.encryptedSeed ?? Data()) else {
                throw TransactionError.seedNotAvailable
            }
            
            let path: DerivationPath
            switch address.type {
            case .external:
                path = DerivationPath.dashBIP44(
                    account: account.accountNumber,
                    change: 0,
                    index: address.index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .internal:
                path = DerivationPath.dashBIP44(
                    account: account.accountNumber,
                    change: 1,
                    index: address.index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .coinJoin:
                path = DerivationPath.coinJoin(
                    account: account.accountNumber,
                    change: address.addressType.contains("external") ? 0 : 1,
                    index: address.index,
                    testnet: wallet.dashNetwork == .testnet
                )
            case .identity:
                path = DerivationPath.dip13Identity(
                    account: account.accountNumber,
                    identityIndex: 0,
                    keyType: .topup,
                    keyIndex: address.index,
                    testnet: wallet.dashNetwork == .testnet
                )
            }
            
            guard let derivedKey = WalletFFIBridge.shared.deriveKey(
                seed: seed,
                path: path.stringRepresentation,
                network: wallet.dashNetwork
            ) else {
                throw TransactionError.keyDerivationFailed
            }
            
            try builder.addInput(utxo: utxo, address: address, privateKey: derivedKey.privateKey)
        }
        
        // Add output
        try builder.addOutput(address: address, amount: amount)
        
        // Build and sign
        return try builder.build()
    }
    
    // MARK: - Transaction Broadcasting
    
    public func broadcastTransaction(_ transaction: BuiltTransaction) async throws {
        guard let spvClient = spvClient else {
            throw TransactionError.noSPVClient
        }
        
        isBroadcasting = true
        defer { isBroadcasting = false }
        
        do {
            // Broadcast through SPV
            // TODO: Implement broadcast with new SPV client
            // try await spvClient.broadcastTransaction(transaction.rawTransaction)
            throw TransactionError.broadcastFailed("SPV broadcast not yet implemented")
            
            // Create transaction record
            let hdTransaction = HDTransaction(txHash: transaction.txid)
            hdTransaction.rawTransaction = transaction.rawTransaction
            hdTransaction.fee = transaction.fee
            hdTransaction.type = "sent"
            hdTransaction.amount = -Int64(transaction.fee) // Will be updated when we process outputs
            hdTransaction.isPending = true
            hdTransaction.wallet = walletManager.currentWallet
            
            // Mark UTXOs as spent
            for (index, utxo) in transaction.inputs.enumerated() {
                try await utxoManager.markUTXOAsSpent(
                    txHash: utxo.txHash,
                    outputIndex: utxo.outputIndex,
                    spendingTxHash: transaction.txid,
                    spendingInputIndex: UInt32(index)
                )
            }
            
            modelContainer.mainContext.insert(hdTransaction)
            try modelContainer.mainContext.save()
            
            await loadTransactions()
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
        
        // Start sync
        try await spvClient.startSync()
    }
    
    // MARK: - Fee Estimation
    
    public func estimateFee(for amount: UInt64, account: HDAccount? = nil) throws -> UInt64 {
        let feePerKB: UInt64 = 1000 // Default fee rate
        
        // Try to select coins to get accurate fee estimate
        do {
            let coinSelection = try utxoManager.selectCoins(
                amount: amount,
                feePerKB: feePerKB,
                account: account
            )
            return coinSelection.fee
        } catch {
            // Fallback estimate
            return 2000 // 2000 duffs as fallback
        }
    }
}

// MARK: - Transaction Errors Extension

extension TransactionError {
    static let noWallet = TransactionError.invalidState
    static let noSPVClient = TransactionError.invalidState
    static let seedNotAvailable = TransactionError.signingFailed
    static let keyDerivationFailed = TransactionError.signingFailed
}