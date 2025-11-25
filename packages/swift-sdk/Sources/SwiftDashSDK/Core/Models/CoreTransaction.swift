import Foundation

public struct CoreTransaction: Identifiable, Equatable {
    public let id: String // txid
    public let amount: Int64 // positive for received, negative for sent
    public let fee: UInt64
    public let timestamp: Date
    public let blockHeight: Int64?
    public let confirmations: Int
    public let type: String // TransactionType is defined in HDTransaction.swift
    public let memo: String?
    public let inputs: [CoreTransactionInput]
    public let outputs: [CoreTransactionOutput]
    public let isInstantSend: Bool
    public let isAssetLock: Bool
    public let rawData: Data?
    
    public var isConfirmed: Bool {
        confirmations >= 6
    }
    
    public var isPending: Bool {
        confirmations == 0
    }
    
    public var formattedAmount: String {
        let dash = Double(abs(amount)) / 100_000_000.0
        let sign = amount < 0 ? "-" : "+"
        return "\(sign)\(String(format: "%.8f", dash)) DASH"
    }
    
    public var formattedFee: String {
        let dash = Double(fee) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
    
    public init(
        id: String,
        amount: Int64,
        fee: UInt64,
        timestamp: Date,
        blockHeight: Int64? = nil,
        confirmations: Int = 0,
        type: String,
        memo: String? = nil,
        inputs: [CoreTransactionInput] = [],
        outputs: [CoreTransactionOutput] = [],
        isInstantSend: Bool = false,
        isAssetLock: Bool = false,
        rawData: Data? = nil
    ) {
        self.id = id
        self.amount = amount
        self.fee = fee
        self.timestamp = timestamp
        self.blockHeight = blockHeight
        self.confirmations = confirmations
        self.type = type
        self.memo = memo
        self.inputs = inputs
        self.outputs = outputs
        self.isInstantSend = isInstantSend
        self.isAssetLock = isAssetLock
        self.rawData = rawData
    }
}

public struct CoreTransactionInput: Equatable {
    public let previousTxid: String
    public let previousOutputIndex: UInt32
    public let address: String?
    public let amount: UInt64?
    public let scriptSignature: Data
    
    public init(
        previousTxid: String,
        previousOutputIndex: UInt32,
        address: String? = nil,
        amount: UInt64? = nil,
        scriptSignature: Data
    ) {
        self.previousTxid = previousTxid
        self.previousOutputIndex = previousOutputIndex
        self.address = address
        self.amount = amount
        self.scriptSignature = scriptSignature
    }
}

public struct CoreTransactionOutput: Equatable {
    public let index: UInt32
    public let address: String
    public let amount: UInt64
    public let scriptPubKey: Data
    public let isChange: Bool
    
    public init(
        index: UInt32,
        address: String,
        amount: UInt64,
        scriptPubKey: Data,
        isChange: Bool = false
    ) {
        self.index = index
        self.address = address
        self.amount = amount
        self.scriptPubKey = scriptPubKey
        self.isChange = isChange
    }
}

// Transaction builder for creating new transactions
public struct CoreTransactionBuilder {
    public var inputs: [CoreTransactionInput] = []
    public var outputs: [CoreTransactionOutput] = []
    public var fee: UInt64 = 0
    public var isInstantSend: Bool = false
    public var isAssetLock: Bool = false
    public var memo: String?
    
    public init() {}
    
    public mutating func addInput(_ input: CoreTransactionInput) {
        inputs.append(input)
    }
    
    public mutating func addOutput(to address: String, amount: UInt64, isChange: Bool = false) {
        let output = CoreTransactionOutput(
            index: UInt32(outputs.count),
            address: address,
            amount: amount,
            scriptPubKey: Data(), // Will be filled by SDK
            isChange: isChange
        )
        outputs.append(output)
    }
    
    public var totalInputAmount: UInt64 {
        inputs.compactMap { $0.amount }.reduce(0, +)
    }
    
    public var totalOutputAmount: UInt64 {
        outputs.reduce(0) { $0 + $1.amount }
    }
    
    public var calculatedFee: UInt64 {
        guard totalInputAmount >= totalOutputAmount else { return 0 }
        return totalInputAmount - totalOutputAmount
    }
}