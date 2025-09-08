import Foundation
import SwiftUI
import Combine

// MARK: - Wallet View Model

@MainActor
public class WalletViewModel: ObservableObject {
    // Published properties
    @Published public var currentWallet: HDWallet?
    @Published public var balance = Balance(confirmed: 0, unconfirmed: 0, immature: 0)
    @Published public var transactions: [HDTransaction] = []
    @Published public var addresses: [HDAddress] = []
    @Published public var isLoading = false
    @Published public var isSyncing = false
    @Published public var syncProgress: Double = 0
    @Published public var error: Error?
    @Published public var showError = false
    
    // Unlock state
    @Published public var isUnlocked = false
    @Published public var requiresPIN = false
    
    // Services
    private let walletService: WalletService
    private let walletManager: WalletManager?
    // private let spvClient: SPVClient  // Now managed by WalletService
    private var cancellables = Set<AnyCancellable>()
    private var unlockedSeed: Data?
    
    public init() throws {
        // Use the shared WalletService instance which has the properly initialized WalletManager
        self.walletService = WalletService.shared
        self.walletManager = walletService.walletManager
        
        // SPV client is now managed by WalletService
        // self.spvClient = try SPVClient()
        
        setupBindings()
        
        Task {
            await loadWallet()
        }
    }
    
    // MARK: - Setup
    
    private func setupBindings() {
        // Wallet changes
        walletManager?.$currentWallet
            .receive(on: DispatchQueue.main)
            .sink { [weak self] wallet in
                self?.currentWallet = wallet
                Task {
                    await self?.refreshBalance()
                    await self?.loadAddresses()
                }
            }
            .store(in: &cancellables)
        
        // Transaction changes (if service configured)
        if let ts = walletManager?.transactionService {
            ts.$transactions
                .receive(on: DispatchQueue.main)
                .assign(to: &$transactions)
        }
        
        // SPV sync progress now handled by WalletService
        // spvClient.syncProgressPublisher
        //     .receive(on: DispatchQueue.main)
        //     .sink { [weak self] progress in
        //         self?.syncProgress = progress.progress
        //         self?.isSyncing = progress.stage != .idle
        //     }
        //     .store(in: &cancellables)
    }
    
    // MARK: - Wallet Management
    
    public func createWallet(label: String, pin: String) async {
        isLoading = true
        defer { isLoading = false }
        
        do {
            guard let walletManager = walletManager else {
                throw WalletError.notImplemented("WalletManager not initialized")
            }
            let wallet = try await walletManager.createWallet(
                label: label,
                network: .testnet,
                pin: pin
            )
            
            currentWallet = wallet
            isUnlocked = true
            requiresPIN = false
            
            // Start sync
            await startSync()
        } catch {
            self.error = error
            showError = true
        }
    }
    
    public func importWallet(mnemonic: String, label: String, pin: String) async {
        isLoading = true
        defer { isLoading = false }
        
        do {
            guard let walletManager = walletManager else {
                throw WalletError.notImplemented("WalletManager not initialized")
            }
            let wallet = try await walletManager.importWallet(
                label: label,
                network: .testnet,
                mnemonic: mnemonic,
                pin: pin
            )
            
            currentWallet = wallet
            isUnlocked = true
            requiresPIN = false
            
            // Start sync
            await startSync()
        } catch {
            self.error = error
            showError = true
        }
    }
    
    public func unlockWallet(pin: String) async {
        do {
            guard let walletManager = walletManager else {
                throw WalletError.notImplemented("WalletManager not initialized")
            }
            unlockedSeed = try await walletManager.unlockWallet(with: pin)
            isUnlocked = true
            requiresPIN = false
            
            // Start sync after unlock
            await startSync()
        } catch {
            self.error = error
            showError = true
        }
    }
    
    // MARK: - Transaction Management
    
    public func sendTransaction(to address: String, amount: Double) async {
        guard isUnlocked else {
            requiresPIN = true
            return
        }
        
        isLoading = true
        defer { isLoading = false }
        
        do {
            // Convert Dash to duffs
            let amountDuffs = UInt64(amount * 100_000_000)
            
            // Create transaction
            guard let walletManager = walletManager else {
                throw WalletError.notImplemented("WalletManager not initialized")
            }
            guard let txService = walletManager.transactionService else {
                throw WalletError.notImplemented("Transaction service not configured")
            }
            let builtTx = try await txService.createTransaction(
                to: address,
                amount: amountDuffs
            )
            
            // Broadcast
            try await txService.broadcastTransaction(builtTx)
            
            // Refresh balance
            await refreshBalance()
        } catch {
            self.error = error
            showError = true
        }
    }
    
    public func estimateFee(for amount: Double) async -> Double {
        let amountDuffs = UInt64(amount * 100_000_000)
        
        do {
            guard let walletManager = walletManager else {
                return 0.00002 // Default fee
            }
            guard let txService = walletManager.transactionService else { return 0.00002 }
            let feeDuffs = try txService.estimateFee(for: amountDuffs)
            return Double(feeDuffs) / 100_000_000
        } catch {
            return 0.00002 // Default fee
        }
    }
    
    // MARK: - Address Management
    
    public func generateNewAddress() async {
        guard let account = currentWallet?.accounts.first else { return }
        
        do {
            guard let walletManager = walletManager else {
                throw WalletError.notImplemented("WalletManager not initialized")
            }
            let address = try await walletManager.getUnusedAddress(for: account)
            await loadAddresses()
            
            // Watch new address in SPV
            // TODO: Implement watch address with new SPV client
            // try await spvClient.watchAddress(address.address)
            print("Would watch address: \(address.address)")
        } catch {
            self.error = error
            showError = true
        }
    }
    
    private func loadAddresses() async {
        guard let account = currentWallet?.accounts.first else { return }
        
        // Get recent external addresses
        addresses = account.externalAddresses
            .sorted { $0.index > $1.index }
            .prefix(10)
            .map { $0 }
    }
    
    // MARK: - Sync Management
    
    public func startSync() async {
        guard let wallet = currentWallet else { return }
        
        isSyncing = true
        
        do {
            // Watch all addresses
            for account in wallet.accounts {
                let allAddresses = account.externalAddresses + account.internalAddresses
                
                for address in allAddresses {
                    // TODO: Implement watch address with new SPV client
            // try await spvClient.watchAddress(address.address)
            print("Would watch address: \(address.address)")
                }
            }
            
            // Set up callbacks for new transactions
            // TODO: Set up transaction callbacks with new SPV client
            // await spvClient.onTransaction { [weak self] txInfo in
            //     Task { @MainActor in
            //         await self?.processIncomingTransaction(txInfo)
            //     }
            // }
            
            // Start sync
            // TODO: Implement start sync with new SPV client
            // try await spvClient.startSync()
            print("Would start sync")
        } catch {
            self.error = error
            showError = true
            isSyncing = false
        }
    }
    
    public func stopSync() async {
        do {
            // TODO: Implement stop sync with new SPV client
            // try await spvClient.stopSync()
            print("Would stop sync")
            isSyncing = false
        } catch {
            self.error = error
            showError = true
        }
    }
    
    // MARK: - Transaction Processing
    
    private func processIncomingTransaction(_ txInfo: TransactionInfo) async {
        do {
            // Process transaction
            guard let walletManager = walletManager else {
                print("WalletManager not available")
                return
            }
            guard let txService = walletManager.transactionService else { return }
            try await txService.processIncomingTransaction(
                txid: txInfo.txid,
                rawTx: txInfo.rawTransaction,
                blockHeight: txInfo.blockHeight,
                timestamp: Date(timeIntervalSince1970: TimeInterval(txInfo.timestamp))
            )
            
            // Refresh balance
            await refreshBalance()
        } catch {
            print("Failed to process transaction: \(error)")
        }
    }
    
    private func findAddress(_ addressString: String) -> HDAddress? {
        guard let wallet = currentWallet else { return nil }
        
        for account in wallet.accounts {
            let allAddresses = account.externalAddresses + account.internalAddresses +
                             account.coinJoinAddresses + account.identityFundingAddresses
            
            if let address = allAddresses.first(where: { $0.address == addressString }) {
                return address
            }
        }
        
        return nil
    }
    
    // MARK: - Balance Management
    
    private func refreshBalance() async {
        guard let account = currentWallet?.accounts.first else { return }
        
        guard let walletManager = walletManager else { return }
        await walletManager.updateBalance(for: account)
        balance = Balance(confirmed: account.confirmedBalance, unconfirmed: account.unconfirmedBalance, immature: 0)
    }
    
    // MARK: - Wallet Loading
    
    private func loadWallet() async {
        // Check if we have existing wallets
        if let walletManager = walletManager, !walletManager.wallets.isEmpty {
            currentWallet = walletManager.wallets.first
            requiresPIN = true // Require PIN to unlock
        }
    }
}

// MARK: - Transaction Info (from SPV)

public struct TransactionInfo {
    public let txid: String
    public let rawTransaction: Data
    public let blockHeight: Int?
    public let timestamp: Int64
    public let outputs: [TransactionOutput]?
}
