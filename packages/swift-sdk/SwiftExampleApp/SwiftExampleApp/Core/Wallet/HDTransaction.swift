import Foundation
import SwiftData

// MARK: - HD Transaction

@Model
public final class HDTransaction {
    @Attribute(.unique) public var id: UUID
    @Attribute(.unique) public var txHash: String
    public var rawTransaction: Data?
    public var blockHeight: Int?
    public var blockHash: String?
    public var timestamp: Date
    public var confirmations: Int
    public var size: Int
    public var fee: UInt64
    public var type: String  // "sent", "received", "self"
    
    // Inputs and outputs
    public var inputsData: Data?  // Serialized TransactionInput array
    public var outputsData: Data? // Serialized TransactionOutput array
    
    // Relationships
    @Relationship public var addresses: [HDAddress] = []
    @Relationship public var wallet: HDWallet?
    
    // Computed amount (positive for received, negative for sent)
    public var amount: Int64
    
    // Transaction status
    public var isPending: Bool
    public var isInstantSend: Bool
    public var isChainLocked: Bool
    
    public init(txHash: String, timestamp: Date = Date()) {
        self.id = UUID()
        self.txHash = txHash
        self.timestamp = timestamp
        self.confirmations = 0
        self.size = 0
        self.fee = 0
        self.type = "received"
        self.amount = 0
        self.isPending = true
        self.isInstantSend = false
        self.isChainLocked = false
    }
    
    public var transactionType: TransactionType {
        return TransactionType(rawValue: type) ?? .received
    }
}

public enum TransactionType: String {
    case sent = "sent"
    case received = "received"
    case `self` = "self"
}

// MARK: - Transaction Components

public struct TransactionInput: Codable {
    public let txHash: String
    public let outputIndex: UInt32
    public let script: Data
    public let sequence: UInt32
    public let amount: UInt64?
    public let address: String?
    
    public init(txHash: String, outputIndex: UInt32, script: Data, sequence: UInt32 = 0xFFFFFFFF, amount: UInt64? = nil, address: String? = nil) {
        self.txHash = txHash
        self.outputIndex = outputIndex
        self.script = script
        self.sequence = sequence
        self.amount = amount
        self.address = address
    }
}

public struct TransactionOutput: Codable {
    public let amount: UInt64
    public let script: Data
    public let address: String?
    public let isChange: Bool
    
    public init(amount: UInt64, script: Data, address: String? = nil, isChange: Bool = false) {
        self.amount = amount
        self.script = script
        self.address = address
        self.isChange = isChange
    }
}

// TransactionBuilder is now defined in TransactionBuilder.swift

/*
public class TransactionBuilder {
    private var inputs: [TransactionInput] = []
    private var outputs: [TransactionOutput] = []
    private let network: DashNetwork
    private let feePerKB: UInt64
    
    public init(network: DashNetwork, feePerKB: UInt64 = 1000) {
        self.network = network
        self.feePerKB = feePerKB
    }
    
    // MARK: - Building Transaction
    
    public func addInput(utxo: HDUTXO, address: HDAddress) {
        let input = TransactionInput(
            txHash: utxo.txHash,
            outputIndex: utxo.outputIndex,
            script: Data(), // Will be filled during signing
            amount: utxo.amount,
            address: address.address
        )
        inputs.append(input)
    }
    
    public func addOutput(address: String, amount: UInt64) throws {
        guard CoreSDKWrapper.shared.validateAddress(address, network: network) else {
            throw TransactionError.invalidAddress
        }
        
        let scriptPubKey = try createScriptPubKey(for: address)
        let output = TransactionOutput(
            amount: amount,
            script: scriptPubKey,
            address: address,
            isChange: false
        )
        outputs.append(output)
    }
    
    public func addChangeOutput(address: String, amount: UInt64) throws {
        guard CoreSDKWrapper.shared.validateAddress(address, network: network) else {
            throw TransactionError.invalidAddress
        }
        
        let scriptPubKey = try createScriptPubKey(for: address)
        let output = TransactionOutput(
            amount: amount,
            script: scriptPubKey,
            address: address,
            isChange: true
        )
        outputs.append(output)
    }
    
    public func calculateFee() -> UInt64 {
        // Estimate transaction size
        let baseSize = 10 // Version (4) + locktime (4) + marker (2)
        let inputSize = inputs.count * 148 // Approximate size per input with signature
        let outputSize = outputs.count * 34 // Approximate size per output
        let estimatedSize = baseSize + inputSize + outputSize
        
        // Calculate fee based on size
        let fee = UInt64(estimatedSize) * feePerKB / 1000
        return max(fee, 1000) // Minimum fee of 1000 duffs
    }
    
    public func build() throws -> RawTransaction {
        guard !inputs.isEmpty else {
            throw TransactionError.noInputs
        }
        
        guard !outputs.isEmpty else {
            throw TransactionError.noOutputs
        }
        
        // Calculate total input and output amounts
        let totalInput = inputs.compactMap { $0.amount }.reduce(0, +)
        let totalOutput = outputs.reduce(0) { $0 + $1.amount }
        let fee = calculateFee()
        
        guard totalInput >= totalOutput + fee else {
            throw TransactionError.insufficientFunds
        }
        
        // Create raw transaction
        return RawTransaction(
            version: 2,
            inputs: inputs,
            outputs: outputs,
            lockTime: 0
        )
    }
    
    // MARK: - Signing
    
    public func sign(transaction: RawTransaction, with privateKeys: [String: Data]) throws -> Data {
        // This should use actual transaction signing logic
        // For now, return mock signed transaction
        var signedInputs: [TransactionInput] = []
        
        for (index, input) in transaction.inputs.enumerated() {
            guard let address = input.address,
                  let privateKey = privateKeys[address] else {
                throw TransactionError.missingPrivateKey
            }
            
            // Create signature script
            let signatureScript = try createSignatureScript(
                for: transaction,
                inputIndex: index,
                privateKey: privateKey
            )
            
            let signedInput = TransactionInput(
                txHash: input.txHash,
                outputIndex: input.outputIndex,
                script: signatureScript,
                sequence: input.sequence,
                amount: input.amount,
                address: input.address
            )
            signedInputs.append(signedInput)
        }
        
        // Serialize signed transaction
        let signedTx = RawTransaction(
            version: transaction.version,
            inputs: signedInputs,
            outputs: transaction.outputs,
            lockTime: transaction.lockTime
        )
        
        return try signedTx.serialize()
    }
    
    // MARK: - Private Methods
    
    private func createScriptPubKey(for address: String) throws -> Data {
        // This should create actual P2PKH script
        // For now, return mock script
        var script = Data()
        script.append(0x76) // OP_DUP
        script.append(0xa9) // OP_HASH160
        script.append(0x14) // Push 20 bytes
        script.append(Data(repeating: 0, count: 20)) // Mock pubkey hash
        script.append(0x88) // OP_EQUALVERIFY
        script.append(0xac) // OP_CHECKSIG
        return script
    }
    
    private func createSignatureScript(for transaction: RawTransaction, inputIndex: Int, privateKey: Data) throws -> Data {
        // This should create actual signature script
        // For now, return mock script
        let signature = CoreSDKWrapper.shared.signTransaction(Data(), with: privateKey) ?? Data()
        let publicKey = CoreSDKWrapper.shared.derivePublicKey(from: privateKey) ?? Data()
        
        var script = Data()
        script.append(UInt8(signature.count + 1)) // Signature length + hash type
        script.append(signature)
        script.append(0x01) // SIGHASH_ALL
        script.append(UInt8(publicKey.count)) // Public key length
        script.append(publicKey)
        
        return script
    }
}
*/

// MARK: - Raw Transaction

public struct RawTransaction {
    public let version: UInt32
    public let inputs: [TransactionInput]
    public let outputs: [TransactionOutput]
    public let lockTime: UInt32
    
    public func serialize() throws -> Data {
        var data = Data()
        
        // Version
        var versionLE = version.littleEndian
        data.append(Data(bytes: &versionLE, count: 4))
        
        // Input count (compact size)
        data.append(compactSize(UInt64(inputs.count)))
        
        // Inputs
        for input in inputs {
            // Previous output
            if let txHashData = Data(hex: input.txHash) {
                data.append(contentsOf: txHashData.reversed()) // Little endian
            }
            var outputIndexLE = input.outputIndex.littleEndian
            data.append(Data(bytes: &outputIndexLE, count: 4))
            
            // Script
            data.append(compactSize(UInt64(input.script.count)))
            data.append(input.script)
            
            // Sequence
            var sequenceLE = input.sequence.littleEndian
            data.append(Data(bytes: &sequenceLE, count: 4))
        }
        
        // Output count
        data.append(compactSize(UInt64(outputs.count)))
        
        // Outputs
        for output in outputs {
            // Amount
            var amountLE = output.amount.littleEndian
            data.append(Data(bytes: &amountLE, count: 8))
            
            // Script
            data.append(compactSize(UInt64(output.script.count)))
            data.append(output.script)
        }
        
        // Lock time
        var lockTimeLE = lockTime.littleEndian
        data.append(Data(bytes: &lockTimeLE, count: 4))
        
        return data
    }
    
    private func compactSize(_ value: UInt64) -> Data {
        if value < 0xfd {
            return Data([UInt8(value)])
        } else if value <= 0xffff {
            var data = Data([0xfd])
            var valueLE = UInt16(value).littleEndian
            data.append(Data(bytes: &valueLE, count: 2))
            return data
        } else if value <= 0xffffffff {
            var data = Data([0xfe])
            var valueLE = UInt32(value).littleEndian
            data.append(Data(bytes: &valueLE, count: 4))
            return data
        } else {
            var data = Data([0xff])
            var valueLE = value.littleEndian
            data.append(Data(bytes: &valueLE, count: 8))
            return data
        }
    }
}

// TransactionError is now defined in TransactionBuilder.swift

// Data hex extension is now defined in TransactionBuilder.swift