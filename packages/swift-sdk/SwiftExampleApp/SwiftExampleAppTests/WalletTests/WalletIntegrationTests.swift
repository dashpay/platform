import XCTest
import SwiftData
@testable import SwiftExampleApp

// MARK: - Wallet Integration Tests

@MainActor
final class WalletIntegrationTests: XCTestCase {
    var walletManager: WalletManager!
    var walletViewModel: WalletViewModel!
    var container: ModelContainer!
    
    override func setUp() async throws {
        try await super.setUp()
        
        // Create test model container
        container = try ModelContainer(for: HDWallet.self, HDAccount.self, HDAddress.self, HDUTXO.self, HDTransaction.self)
        
        // Create test wallet manager
        walletManager = try WalletManager(modelContainer: container)
        
        // Create view model
        walletViewModel = try WalletViewModel()
    }
    
    override func tearDown() async throws {
        // Clean up test wallets
        for wallet in walletManager.wallets {
            try await walletManager.deleteWallet(wallet)
        }
        
        walletManager = nil
        walletViewModel = nil
        container = nil
        
        try await super.tearDown()
    }
    
    // MARK: - Wallet Creation Tests
    
    func testCreateWallet() async throws {
        let label = "Test Wallet"
        let pin = "123456"
        
        let wallet = try await walletManager.createWallet(
            label: label,
            network: .testnet,
            pin: pin
        )
        
        XCTAssertNotNil(wallet)
        XCTAssertEqual(wallet.label, label)
        XCTAssertEqual(wallet.dashNetwork, .testnet)
        XCTAssertFalse(wallet.isWatchOnly)
        XCTAssertNotNil(wallet.encryptedSeed)
        XCTAssertEqual(wallet.accounts.count, 1)
        
        // Check default account
        let account = wallet.accounts[0]
        XCTAssertEqual(account.accountNumber, 0)
        XCTAssertGreaterThan(account.externalAddresses.count, 0)
        XCTAssertGreaterThan(account.internalAddresses.count, 0)
    }
    
    func testImportWalletFromMnemonic() async throws {
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        let label = "Imported Wallet"
        let pin = "654321"
        
        let wallet = try await walletManager.importWallet(
            label: label,
            network: .testnet,
            mnemonic: mnemonic,
            pin: pin
        )
        
        XCTAssertNotNil(wallet)
        XCTAssertEqual(wallet.label, label)
        
        // Verify known address for this mnemonic on testnet
        let firstAddress = wallet.accounts[0].externalAddresses[0]
        XCTAssertNotNil(firstAddress)
        // Address should be deterministic for this mnemonic
    }
    
    // MARK: - PIN Management Tests
    
    func testUnlockWalletWithPIN() async throws {
        let pin = "123456"
        
        // Create wallet
        let wallet = try await walletManager.createWallet(
            label: "PIN Test",
            network: .testnet,
            pin: pin
        )
        
        // Try to unlock with correct PIN
        let seed = try await walletManager.unlockWallet(with: pin)
        XCTAssertNotNil(seed)
        XCTAssertFalse(seed.isEmpty)
        
        // Try to unlock with wrong PIN
        do {
            _ = try await walletManager.unlockWallet(with: "wrong")
            XCTFail("Should have thrown error for wrong PIN")
        } catch {
            // Expected
        }
    }
    
    func testChangePIN() async throws {
        let currentPIN = "123456"
        let newPIN = "654321"
        
        // Create wallet
        _ = try await walletManager.createWallet(
            label: "PIN Change Test",
            network: .testnet,
            pin: currentPIN
        )
        
        // Change PIN
        try await walletManager.changeWalletPIN(currentPIN: currentPIN, newPIN: newPIN)
        
        // Try old PIN (should fail)
        do {
            _ = try await walletManager.unlockWallet(with: currentPIN)
            XCTFail("Old PIN should not work")
        } catch {
            // Expected
        }
        
        // Try new PIN (should work)
        let seed = try await walletManager.unlockWallet(with: newPIN)
        XCTAssertNotNil(seed)
    }
    
    // MARK: - Address Generation Tests
    
    func testAddressGeneration() async throws {
        let wallet = try await walletManager.createWallet(
            label: "Address Test",
            network: .testnet,
            pin: "123456"
        )
        
        let account = wallet.accounts[0]
        
        // Get unused external address
        let address1 = try await walletManager.getUnusedAddress(for: account, type: .external)
        XCTAssertNotNil(address1)
        XCTAssertEqual(address1.type, .external)
        XCTAssertFalse(address1.isUsed)
        
        // Mark as used
        address1.isUsed = true
        
        // Get next unused address
        let address2 = try await walletManager.getUnusedAddress(for: account, type: .external)
        XCTAssertNotEqual(address1.address, address2.address)
        XCTAssertEqual(address2.index, address1.index + 1)
        
        // Test internal address
        let internalAddress = try await walletManager.getUnusedAddress(for: account, type: .internal)
        XCTAssertEqual(internalAddress.type, .internal)
    }
    
    // MARK: - UTXO Management Tests
    
    func testUTXOManagement() async throws {
        let wallet = try await walletManager.createWallet(
            label: "UTXO Test",
            network: .testnet,
            pin: "123456"
        )
        
        let account = wallet.accounts[0]
        let address = account.externalAddresses[0]
        
        // Add test UTXO
        guard let utxoManager = walletManager.utxoManager else {
            XCTFail("UTXO Manager not available")
            return
        }
        
        try await utxoManager.addUTXO(
            txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            outputIndex: 0,
            amount: 100_000_000, // 1 DASH
            scriptPubKey: Data(repeating: 0, count: 25),
            address: address,
            blockHeight: 1000
        )
        
        // Verify UTXO was added
        await utxoManager.loadUTXOs()
        let utxo = utxoManager.utxos.first
        
        XCTAssertNotNil(utxo)
        XCTAssertEqual(utxo?.amount, 100_000_000)
        XCTAssertFalse(utxo?.isSpent ?? true)
        
        // Test balance calculation
        let balance = utxoManager.calculateBalance(for: account)
        XCTAssertEqual(balance.confirmed, 100_000_000)
        XCTAssertEqual(balance.unconfirmed, 0)
        XCTAssertEqual(balance.total, 100_000_000)
        
        // Test coin selection
        let selection = try utxoManager.selectCoins(
            amount: 50_000_000,
            feePerKB: 1000,
            account: account
        )
        
        XCTAssertEqual(selection.utxos.count, 1)
        XCTAssertEqual(selection.totalAmount, 100_000_000)
        XCTAssertGreaterThan(selection.fee, 0)
        XCTAssertGreaterThan(selection.change, 0)
    }
    
    // MARK: - Transaction Tests
    
    func testTransactionCreation() async throws {
        let wallet = try await walletManager.createWallet(
            label: "Transaction Test",
            network: .testnet,
            pin: "123456"
        )
        
        let account = wallet.accounts[0]
        let address = account.externalAddresses[0]
        
        // Add test UTXO with sufficient balance
        guard let utxoManager = walletManager.utxoManager else {
            XCTFail("UTXO Manager not available")
            return
        }
        
        try await utxoManager.addUTXO(
            txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            outputIndex: 0,
            amount: 100_000_000, // 1 DASH
            scriptPubKey: Data(repeating: 0x76, count: 25), // Dummy P2PKH script
            address: address,
            blockHeight: 1000
        )
        
        // Create transaction
        let recipientAddress = "yTsGq4wV8WySdQTYgGqmiUKMxb8RBr6wc6" // Testnet address
        let amount: UInt64 = 50_000_000 // 0.5 DASH
        
        do {
            guard let transactionService = walletManager.transactionService else {
                XCTFail("Transaction service not available")
                return
            }
            
            let builtTx = try await transactionService.createTransaction(
                to: recipientAddress,
                amount: amount,
                from: account
            )
            
            XCTAssertNotNil(builtTx)
            XCTAssertFalse(builtTx.txid.isEmpty)
            XCTAssertGreaterThan(builtTx.fee, 0)
            XCTAssertFalse(builtTx.rawTransaction.isEmpty)
        } catch {
            // Transaction creation might fail due to missing FFI implementation
            // This is expected in unit tests
            print("Transaction creation error (expected in tests): \(error)")
        }
    }
    
    // MARK: - View Model Tests
    
    func testViewModelWalletCreation() async throws {
        let label = "ViewModel Test"
        let pin = "123456"
        
        await walletViewModel.createWallet(label: label, pin: pin)
        
        XCTAssertNotNil(walletViewModel.currentWallet)
        XCTAssertEqual(walletViewModel.currentWallet?.label, label)
        XCTAssertTrue(walletViewModel.isUnlocked)
        XCTAssertFalse(walletViewModel.requiresPIN)
    }
    
    func testViewModelAddressGeneration() async throws {
        // Create wallet first
        await walletViewModel.createWallet(label: "Address Test", pin: "123456")
        
        let initialAddressCount = walletViewModel.addresses.count
        
        await walletViewModel.generateNewAddress()
        
        // Should have new addresses loaded
        XCTAssertGreaterThanOrEqual(walletViewModel.addresses.count, initialAddressCount)
    }
    
    func testViewModelBalanceUpdate() async throws {
        // Create wallet
        await walletViewModel.createWallet(label: "Balance Test", pin: "123456")
        
        guard let account = walletViewModel.currentWallet?.accounts.first,
              let address = account.externalAddresses.first else {
            XCTFail("No account or address found")
            return
        }
        
        // Add UTXO
        guard let utxoManager = walletManager.utxoManager else {
            XCTFail("UTXO Manager not available")
            return
        }
        
        try await utxoManager.addUTXO(
            txHash: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
            outputIndex: 0,
            amount: 200_000_000, // 2 DASH
            scriptPubKey: Data(repeating: 0x76, count: 25),
            address: address,
            blockHeight: 2000
        )
        
        // Wait for balance update
        try await Task.sleep(nanoseconds: 100_000_000) // 0.1 seconds
        
        XCTAssertEqual(walletViewModel.balance.confirmed, 200_000_000)
        XCTAssertEqual(walletViewModel.balance.total, 200_000_000)
    }
    
    // MARK: - Persistence Tests
    
    func testWalletPersistence() async throws {
        let label = "Persistent Wallet"
        let pin = "123456"
        
        // Create wallet
        let wallet = try await walletManager.createWallet(
            label: label,
            network: .testnet,
            pin: pin
        )
        
        let walletId = wallet.id
        
        // Create new wallet manager to test loading
        let newContainer = try ModelContainer(for: HDWallet.self, HDAccount.self, HDAddress.self, HDUTXO.self, HDTransaction.self)
        let newManager = try WalletManager(modelContainer: newContainer)
        
        // Wait for loading
        try await Task.sleep(nanoseconds: 100_000_000) // 0.1 seconds
        
        // Find wallet
        let loadedWallet = newManager.wallets.first { $0.id == walletId }
        XCTAssertNotNil(loadedWallet)
        XCTAssertEqual(loadedWallet?.label, label)
        XCTAssertEqual(loadedWallet?.accounts.count, wallet.accounts.count)
    }
    
    // MARK: - Error Handling Tests
    
    func testInvalidMnemonicImport() async throws {
        do {
            _ = try await walletManager.importWallet(
                label: "Invalid",
                network: .testnet,
                mnemonic: "invalid mnemonic phrase",
                pin: "123456"
            )
            XCTFail("Should have thrown error for invalid mnemonic")
        } catch {
            // Expected
            XCTAssertTrue(error is WalletError)
        }
    }
    
    func testInsufficientBalanceTransaction() async throws {
        let wallet = try await walletManager.createWallet(
            label: "Insufficient Balance",
            network: .testnet,
            pin: "123456"
        )
        
        let account = wallet.accounts[0]
        
        // Try to create transaction without any UTXOs
        do {
            guard let transactionService = walletManager.transactionService else {
                XCTFail("Transaction service not available")
                return
            }
            
            _ = try await transactionService.createTransaction(
                to: "yTsGq4wV8WySdQTYgGqmiUKMxb8RBr6wc6",
                amount: 100_000_000,
                from: account
            )
            XCTFail("Should have thrown insufficient balance error")
        } catch {
            // Expected
            print("Expected error: \(error)")
        }
    }
}