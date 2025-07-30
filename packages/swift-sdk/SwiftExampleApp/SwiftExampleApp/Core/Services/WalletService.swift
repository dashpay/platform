import Foundation
import SwiftData
import Combine

@MainActor
public class WalletService: ObservableObject {
    public static let shared = WalletService()
    
    // Published properties
    @Published public var currentWallet: HDWallet?
    @Published public var balance = Balance()
    @Published public var isSyncing = false
    @Published public var syncProgress: Double?
    @Published public var detailedSyncProgress: SyncProgress?
    @Published public var lastSyncError: Error?
    @Published public var transactions: [Transaction] = []
    
    // Internal properties
    private var modelContext: ModelContext?
    private var syncTask: Task<Void, Never>?
    private var balanceUpdateTask: Task<Void, Never>?
    
    // Mock SDK for now - will be replaced with real SDK
    private var sdk: Any?
    
    private init() {}
    
    public func configure(modelContext: ModelContext) {
        self.modelContext = modelContext
        loadCurrentWallet()
    }
    
    public func setSharedSDK(_ sdk: Any) {
        self.sdk = sdk
        print("✅ WalletService configured with shared SDK")
    }
    
    // MARK: - Wallet Management
    
    public func createWallet(label: String, mnemonic: String? = nil) async throws -> HDWallet {
        guard let modelContext = modelContext else {
            throw WalletError.unknown("Model context not configured")
        }
        
        // Create wallet
        let wallet = HDWallet(label: label)
        wallet.mnemonic = mnemonic ?? generateMnemonic()
        
        // Save to SwiftData
        modelContext.insert(wallet)
        try modelContext.save()
        
        // Set as current wallet
        currentWallet = wallet
        
        // Start initial sync
        await startSync()
        
        return wallet
    }
    
    public func loadWallet(_ wallet: HDWallet) async {
        currentWallet = wallet
        
        // Load transactions
        await loadTransactions()
        
        // Update balance
        updateBalance()
        
        // Start sync if needed
        if wallet.syncProgress < 1.0 {
            await startSync()
        }
    }
    
    private func loadCurrentWallet() {
        guard let modelContext = modelContext else { return }
        
        let descriptor = FetchDescriptor<HDWallet>(
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        
        do {
            let wallets = try modelContext.fetch(descriptor)
            currentWallet = wallets.first
            
            if let wallet = currentWallet {
                Task {
                    await loadWallet(wallet)
                }
            }
        } catch {
            print("Failed to load wallets: \(error)")
        }
    }
    
    // MARK: - Sync Management
    
    public func startSync() async {
        guard !isSyncing else { return }
        
        isSyncing = true
        lastSyncError = nil
        
        syncTask?.cancel()
        syncTask = Task {
            do {
                // Mock sync progress
                for i in 0...100 {
                    if Task.isCancelled { break }
                    
                    let progress = Double(i) / 100.0
                    await MainActor.run {
                        self.syncProgress = progress
                        self.detailedSyncProgress = SyncProgress(
                            percentage: progress * 100,
                            currentBlock: Int(1000 * progress),
                            totalBlocks: 1000,
                            estimatedTimeRemaining: TimeInterval(100 - i) * 2
                        )
                    }
                    
                    try await Task.sleep(nanoseconds: 100_000_000) // 0.1 second
                }
                
                // Update wallet sync status
                if let wallet = currentWallet {
                    wallet.syncProgress = 1.0
                    wallet.lastSyncedAt = Date()
                    try? modelContext?.save()
                }
                
            } catch {
                lastSyncError = error
            }
            
            isSyncing = false
            syncProgress = nil
            detailedSyncProgress = nil
        }
    }
    
    public func stopSync() {
        syncTask?.cancel()
        syncTask = nil
        isSyncing = false
        syncProgress = nil
        detailedSyncProgress = nil
    }
    
    // MARK: - Transaction Management
    
    public func sendTransaction(to address: String, amount: UInt64, memo: String? = nil) async throws -> String {
        guard let wallet = currentWallet else {
            throw WalletError.unknown("No active wallet")
        }
        
        guard wallet.confirmedBalance >= amount else {
            throw WalletError.insufficientFunds
        }
        
        // Mock transaction creation
        let txid = UUID().uuidString
        let transaction = HDTransaction(
            txid: txid,
            amount: -Int64(amount),
            fee: 1000,
            timestamp: Date(),
            type: .sent
        )
        transaction.memo = memo
        transaction.wallet = wallet
        
        modelContext?.insert(transaction)
        try? modelContext?.save()
        
        // Update balance
        updateBalance()
        
        return txid
    }
    
    private func loadTransactions() async {
        guard let wallet = currentWallet else { return }
        
        // Convert HDTransaction to Transaction
        transactions = wallet.transactions.map { hdTx in
            Transaction(
                id: hdTx.txid,
                amount: hdTx.amount,
                fee: hdTx.fee,
                timestamp: hdTx.timestamp,
                blockHeight: hdTx.blockHeight,
                confirmations: hdTx.confirmations,
                type: TransactionType(rawValue: hdTx.type) ?? .received,
                memo: hdTx.memo,
                isInstantSend: hdTx.isInstantSend,
                isAssetLock: hdTx.isAssetLock
            )
        }.sorted { $0.timestamp > $1.timestamp }
    }
    
    // MARK: - Balance Management
    
    private func updateBalance() {
        guard let wallet = currentWallet else {
            balance = Balance()
            return
        }
        
        balance = Balance(
            confirmed: wallet.confirmedBalance,
            unconfirmed: wallet.unconfirmedBalance
        )
    }
    
    // MARK: - Address Management
    
    public func getNewAddress() async throws -> String {
        guard let wallet = currentWallet else {
            throw WalletError.unknown("No active wallet")
        }
        
        // Find next unused address or create new one
        let existingAddresses = wallet.addresses.filter { $0.type == AddressType.external.rawValue }
        let nextIndex = UInt32(existingAddresses.count)
        
        // Mock address generation
        let address = "yMockAddress\(nextIndex)"
        
        let hdAddress = HDAddress(
            address: address,
            index: nextIndex,
            type: .external
        )
        hdAddress.wallet = wallet
        
        modelContext?.insert(hdAddress)
        try? modelContext?.save()
        
        return address
    }
    
    // MARK: - Helpers
    
    private func generateMnemonic() -> String {
        // Mock mnemonic generation
        let words = ["abandon", "ability", "able", "about", "above", "absent",
                    "absorb", "abstract", "absurd", "abuse", "access", "accident"]
        return words.joined(separator: " ")
    }
}

// MARK: - Sync Progress

public struct SyncProgress {
    public let percentage: Double
    public let currentBlock: Int
    public let totalBlocks: Int
    public let estimatedTimeRemaining: TimeInterval
    
    public var formattedPercentage: String {
        String(format: "%.1f%%", percentage)
    }
    
    public var formattedTimeRemaining: String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.hour, .minute, .second]
        formatter.unitsStyle = .abbreviated
        return formatter.string(from: estimatedTimeRemaining) ?? "Unknown"
    }
    
    public var formattedBlocks: String {
        "\(currentBlock) / \(totalBlocks)"
    }
}