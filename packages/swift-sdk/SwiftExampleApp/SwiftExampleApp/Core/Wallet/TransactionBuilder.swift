import Foundation
import SwiftData
import DashSDKFFI

// MARK: - Transaction Builder

public class TransactionBuilder {
    private let ffi = WalletFFIBridge.shared
    private var transaction: UnsafeMutablePointer<FFITransaction>?
    private var inputs: [(utxo: HDUTXO, address: HDAddress, privateKey: Data)] = []
    private var outputs: [(address: String, amount: UInt64)] = []
    private var changeAddress: String?
    private let network: Network
    private let feePerKB: UInt64
    
    public init(network: Network, feePerKB: UInt64 = 1000) {
        self.network = network
        self.feePerKB = feePerKB
        self.transaction = ffi.createTransaction()
    }
    
    deinit {
        if let tx = transaction {
            ffi.destroyTransaction(tx)
        }
    }
    
    // MARK: - Building Transaction
    
    public func addInput(utxo: HDUTXO, address: HDAddress, privateKey: Data) throws {
        guard let tx = transaction else {
            throw TransactionError.invalidState
        }
        
        guard let txidData = Data(hex: utxo.txHash) else {
            throw TransactionError.invalidInput("Invalid transaction hash")
        }
        
        // Add to internal tracking
        inputs.append((utxo, address, privateKey))
        
        // Add to transaction
        guard ffi.addInput(
            to: tx,
            txid: txidData,
            vout: utxo.outputIndex,
            scriptSig: Data(), // Will be filled during signing
            sequence: 0xFFFFFFFF
        ) else {
            throw TransactionError.invalidInput("Failed to add input")
        }
    }
    
    public func addOutput(address: String, amount: UInt64) throws {
        guard let tx = transaction else {
            throw TransactionError.invalidState
        }
        
        guard ffi.validateAddress(address, network: network) else {
            throw TransactionError.invalidAddress
        }
        
        outputs.append((address, amount))
        
        guard ffi.addOutput(to: tx, address: address, amount: amount, network: network) else {
            throw TransactionError.invalidOutput("Failed to add output")
        }
    }
    
    public func setChangeAddress(_ address: String) throws {
        guard ffi.validateAddress(address, network: network) else {
            throw TransactionError.invalidAddress
        }
        changeAddress = address
    }
    
    // MARK: - Fee Calculation
    
    public func calculateFee() -> UInt64 {
        // Estimate transaction size
        let baseSize = 10 // Version (4) + locktime (4) + marker (2)
        let inputSize = inputs.count * 148 // Approximate size per input with signature
        let outputSize = (outputs.count + (changeAddress != nil ? 1 : 0)) * 34 // Approximate size per output
        let estimatedSize = baseSize + inputSize + outputSize
        
        // Calculate fee based on size
        let fee = UInt64(estimatedSize) * feePerKB / 1000
        return max(fee, 1000) // Minimum fee of 1000 duffs
    }
    
    // MARK: - Building and Signing
    
    public func build() throws -> BuiltTransaction {
        guard let tx = transaction else {
            throw TransactionError.invalidState
        }
        
        guard !inputs.isEmpty else {
            throw TransactionError.noInputs
        }
        
        guard !outputs.isEmpty else {
            throw TransactionError.noOutputs
        }
        
        // Calculate total input and output amounts
        let totalInput = inputs.reduce(0) { $0 + $1.utxo.amount }
        let totalOutput = outputs.reduce(0) { $0 + $1.amount }
        let fee = calculateFee()
        
        guard totalInput >= totalOutput + fee else {
            throw TransactionError.insufficientFunds
        }
        
        // Add change output if needed
        let change = totalInput - totalOutput - fee
        if change > 546 { // Dust threshold
            guard let changeAddr = changeAddress else {
                throw TransactionError.noChangeAddress
            }
            
            guard ffi.addOutput(to: tx, address: changeAddr, amount: change, network: network) else {
                throw TransactionError.invalidOutput("Failed to add change output")
            }
        }
        
        // Sign all inputs
        for (index, input) in inputs.enumerated() {
            // Get the script pubkey for the UTXO
            let scriptPubkey = input.utxo.scriptPubKey
            
            guard ffi.signInput(
                tx: tx,
                inputIndex: UInt32(index),
                privateKey: input.privateKey,
                scriptPubkey: scriptPubkey,
                sighashType: 1 // SIGHASH_ALL
            ) else {
                throw TransactionError.signingFailed
            }
        }
        
        // Get transaction ID and serialized data
        guard let txid = ffi.getTransactionId(tx) else {
            throw TransactionError.serializationFailed
        }
        
        guard let rawTx = ffi.serializeTransaction(tx) else {
            throw TransactionError.serializationFailed
        }
        
        return BuiltTransaction(
            txid: txid.hexString,
            rawTransaction: rawTx,
            fee: fee,
            inputs: inputs.map { $0.utxo },
            changeAmount: change > 546 ? change : 0
        )
    }
}

// MARK: - Built Transaction

public struct BuiltTransaction {
    public let txid: String
    public let rawTransaction: Data
    public let fee: UInt64
    public let inputs: [HDUTXO]
    public let changeAmount: UInt64
}

// MARK: - Transaction Errors

public enum TransactionError: LocalizedError {
    case invalidState
    case noInputs
    case noOutputs
    case insufficientFunds
    case invalidAddress
    case invalidInput(String)
    case invalidOutput(String)
    case noChangeAddress
    case signingFailed
    case serializationFailed
    case broadcastFailed(String)
    
    public var errorDescription: String? {
        switch self {
        case .invalidState:
            return "Transaction in invalid state"
        case .noInputs:
            return "Transaction has no inputs"
        case .noOutputs:
            return "Transaction has no outputs"
        case .insufficientFunds:
            return "Insufficient funds for transaction"
        case .invalidAddress:
            return "Invalid recipient address"
        case .invalidInput(let message):
            return "Invalid input: \(message)"
        case .invalidOutput(let message):
            return "Invalid output: \(message)"
        case .noChangeAddress:
            return "No change address specified"
        case .signingFailed:
            return "Failed to sign transaction"
        case .serializationFailed:
            return "Failed to serialize transaction"
        case .broadcastFailed(let message):
            return "Failed to broadcast: \(message)"
        }
    }
}

// MARK: - Data Extension

extension Data {
    init?(hex: String) {
        let hex = hex.replacingOccurrences(of: " ", with: "")
        guard hex.count % 2 == 0 else { return nil }
        
        var data = Data()
        var index = hex.startIndex
        
        while index < hex.endIndex {
            let byteString = String(hex[index..<hex.index(index, offsetBy: 2)])
            guard let byte = UInt8(byteString, radix: 16) else { return nil }
            data.append(byte)
            index = hex.index(index, offsetBy: 2)
        }
        
        self = data
    }
}