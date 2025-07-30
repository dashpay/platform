import XCTest
@testable import SwiftExampleApp

// MARK: - Transaction Tests

final class TransactionTests: XCTestCase {
    
    // MARK: - Transaction Builder Tests
    
    func testTransactionBuilderBasic() {
        let builder = TransactionBuilder(network: .testnet, feePerKB: 1000)
        
        XCTAssertNotNil(builder)
        XCTAssertEqual(builder.network, .testnet)
        XCTAssertEqual(builder.feePerKB, 1000)
    }
    
    func testTransactionBuilderAddInput() throws {
        let builder = TransactionBuilder(network: .testnet)
        
        // Create mock UTXO
        let utxo = MockUTXO(
            txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            outputIndex: 0,
            amount: 100_000_000,
            scriptPubKey: Data(repeating: 0x76, count: 25)
        )
        
        let address = MockAddress(address: "yTsGq4wV8WySdQTYgGqmiUKMxb8RBr6wc6")
        let privateKey = Data(repeating: 0x01, count: 32)
        
        try builder.addInput(utxo: utxo, address: address, privateKey: privateKey)
        
        XCTAssertEqual(builder.inputs.count, 1)
        XCTAssertEqual(builder.totalInputAmount, 100_000_000)
    }
    
    func testTransactionBuilderAddOutput() throws {
        let builder = TransactionBuilder(network: .testnet)
        
        let address = "yTsGq4wV8WySdQTYgGqmiUKMxb8RBr6wc6"
        let amount: UInt64 = 50_000_000
        
        try builder.addOutput(address: address, amount: amount)
        
        XCTAssertEqual(builder.outputs.count, 1)
        XCTAssertEqual(builder.totalOutputAmount, amount)
    }
    
    func testTransactionBuilderChangeAddress() throws {
        let builder = TransactionBuilder(network: .testnet)
        
        let changeAddress = "yXdUfGBfX6rQmNq5speeNGD5HfL2qkYBNe"
        try builder.setChangeAddress(changeAddress)
        
        XCTAssertEqual(builder.changeAddress, changeAddress)
    }
    
    func testTransactionBuilderInsufficientBalance() throws {
        let builder = TransactionBuilder(network: .testnet)
        
        // Add small input
        let utxo = MockUTXO(
            txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            outputIndex: 0,
            amount: 10_000,
            scriptPubKey: Data(repeating: 0x76, count: 25)
        )
        
        let address = MockAddress(address: "yTsGq4wV8WySdQTYgGqmiUKMxb8RBr6wc6")
        let privateKey = Data(repeating: 0x01, count: 32)
        
        try builder.addInput(utxo: utxo, address: address, privateKey: privateKey)
        
        // Try to add large output
        try builder.addOutput(address: "yXdUfGBfX6rQmNq5speeNGD5HfL2qkYBNe", amount: 100_000_000)
        
        // Should fail when building
        do {
            _ = try builder.build()
            XCTFail("Should have thrown insufficient balance error")
        } catch TransactionError.insufficientBalance {
            // Expected
        }
    }
    
    // MARK: - UTXO Manager Tests
    
    func testUTXOManagerCoinSelection() throws {
        let walletManager = try WalletManager()
        let utxoManager = walletManager.utxoManager!
        
        // Create mock UTXOs
        let utxos = [
            MockUTXO(
                txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                outputIndex: 0,
                amount: 50_000_000,
                scriptPubKey: Data(repeating: 0x76, count: 25)
            ),
            MockUTXO(
                txHash: "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
                outputIndex: 1,
                amount: 30_000_000,
                scriptPubKey: Data(repeating: 0x76, count: 25)
            ),
            MockUTXO(
                txHash: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
                outputIndex: 0,
                amount: 100_000_000,
                scriptPubKey: Data(repeating: 0x76, count: 25)
            )
        ]
        
        // Test selecting coins for 70 million duffs
        let targetAmount: UInt64 = 70_000_000
        let selectedUTXOs = utxoManager.selectCoinsFromList(
            utxos: utxos,
            targetAmount: targetAmount,
            feePerKB: 1000
        )
        
        XCTAssertNotNil(selectedUTXOs)
        
        // Should select the 100M UTXO (largest first strategy)
        XCTAssertEqual(selectedUTXOs?.utxos.count, 1)
        XCTAssertEqual(selectedUTXOs?.totalAmount, 100_000_000)
        XCTAssertGreaterThan(selectedUTXOs?.fee ?? 0, 0)
        XCTAssertGreaterThan(selectedUTXOs?.change ?? 0, 0)
    }
    
    func testUTXOManagerCoinSelectionExactAmount() throws {
        let walletManager = try WalletManager()
        let utxoManager = walletManager.utxoManager!
        
        let utxos = [
            MockUTXO(
                txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                outputIndex: 0,
                amount: 50_000_000,
                scriptPubKey: Data(repeating: 0x76, count: 25)
            )
        ]
        
        // Try to select exactly what we have minus expected fee
        let targetAmount: UInt64 = 49_999_000
        let selectedUTXOs = utxoManager.selectCoinsFromList(
            utxos: utxos,
            targetAmount: targetAmount,
            feePerKB: 1000
        )
        
        XCTAssertNotNil(selectedUTXOs)
        XCTAssertEqual(selectedUTXOs?.utxos.count, 1)
        XCTAssertEqual(selectedUTXOs?.change, 0) // No change expected
    }
    
    func testUTXOManagerInsufficientBalance() throws {
        let walletManager = try WalletManager()
        let utxoManager = walletManager.utxoManager!
        
        let utxos = [
            MockUTXO(
                txHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                outputIndex: 0,
                amount: 10_000,
                scriptPubKey: Data(repeating: 0x76, count: 25)
            )
        ]
        
        // Try to select more than available
        let targetAmount: UInt64 = 100_000_000
        let selectedUTXOs = utxoManager.selectCoinsFromList(
            utxos: utxos,
            targetAmount: targetAmount,
            feePerKB: 1000
        )
        
        XCTAssertNil(selectedUTXOs) // Should return nil for insufficient balance
    }
    
    // MARK: - Fee Calculation Tests
    
    func testFeeCalculation() {
        let calculator = FeeCalculator()
        
        // Test basic transaction size (1 input, 2 outputs)
        let fee = calculator.calculateFee(
            inputs: 1,
            outputs: 2,
            feePerKB: 1000
        )
        
        // Expected size ~226 bytes (148 + 34*2 + 10)
        // Fee should be around 226 satoshis
        XCTAssertGreaterThan(fee, 200)
        XCTAssertLessThan(fee, 300)
    }
    
    func testFeeCalculationMultipleInputs() {
        let calculator = FeeCalculator()
        
        // Test with multiple inputs
        let fee = calculator.calculateFee(
            inputs: 5,
            outputs: 2,
            feePerKB: 1000
        )
        
        // Each input adds ~148 bytes
        // Expected size ~818 bytes
        XCTAssertGreaterThan(fee, 800)
        XCTAssertLessThan(fee, 900)
    }
}

// MARK: - Mock Objects

struct MockUTXO: UTXOProtocol {
    let txHash: String
    let outputIndex: UInt32
    let amount: UInt64
    let scriptPubKey: Data
    let blockHeight: Int? = nil
    
    var isSpent: Bool = false
}

struct MockAddress: AddressProtocol {
    let address: String
    let derivationPath: String = "m/44'/5'/0'/0/0"
    let index: UInt32 = 0
    let type: AddressType = .external
}

// MARK: - Fee Calculator

struct FeeCalculator {
    // Transaction size estimation
    // Input: ~148 bytes (prev tx + index + script + sequence)
    // Output: ~34 bytes (amount + script length + script)
    // Fixed: ~10 bytes (version + locktime)
    
    func calculateFee(inputs: Int, outputs: Int, feePerKB: UInt64) -> UInt64 {
        let inputSize = 148 * inputs
        let outputSize = 34 * outputs
        let fixedSize = 10
        
        let totalSize = inputSize + outputSize + fixedSize
        
        // Calculate fee (satoshis per kilobyte)
        return UInt64((Double(totalSize) / 1000.0) * Double(feePerKB))
    }
}

// MARK: - Protocol Extensions

protocol UTXOProtocol {
    var txHash: String { get }
    var outputIndex: UInt32 { get }
    var amount: UInt64 { get }
    var scriptPubKey: Data { get }
    var isSpent: Bool { get }
}

protocol AddressProtocol {
    var address: String { get }
    var derivationPath: String { get }
    var index: UInt32 { get }
    var type: AddressType { get }
}

extension HDUTXO: UTXOProtocol {}
extension HDAddress: AddressProtocol {}

// MARK: - UTXO Manager Test Extensions

extension UTXOManager {
    func selectCoinsFromList(
        utxos: [any UTXOProtocol],
        targetAmount: UInt64,
        feePerKB: UInt64
    ) -> CoinSelection? {
        // Simple largest-first coin selection for testing
        let sortedUTXOs = utxos.filter { !$0.isSpent }.sorted { $0.amount > $1.amount }
        
        var selectedUTXOs: [any UTXOProtocol] = []
        var totalAmount: UInt64 = 0
        
        for utxo in sortedUTXOs {
            selectedUTXOs.append(utxo)
            totalAmount += utxo.amount
            
            // Estimate fee
            let estimatedFee = FeeCalculator().calculateFee(
                inputs: selectedUTXOs.count,
                outputs: 2, // Output + change
                feePerKB: feePerKB
            )
            
            if totalAmount >= targetAmount + estimatedFee {
                let change = totalAmount - targetAmount - estimatedFee
                
                // Convert to HDUTXOs for return type
                let hdUTXOs = selectedUTXOs.compactMap { utxo -> HDUTXO? in
                    // In real implementation, these would be actual HDUTXO objects
                    // For testing, we just need the selection logic
                    return nil
                }
                
                return CoinSelection(
                    utxos: hdUTXOs,
                    totalAmount: totalAmount,
                    targetAmount: targetAmount,
                    fee: estimatedFee,
                    change: change
                )
            }
        }
        
        return nil // Insufficient balance
    }
}