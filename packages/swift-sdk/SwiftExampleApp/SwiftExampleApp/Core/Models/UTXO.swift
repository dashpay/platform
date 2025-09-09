import Foundation

public struct UTXO: Identifiable, Equatable {
    public var id: String {
        "\(txid):\(outputIndex)"
    }
    
    public let txid: String
    public let outputIndex: UInt32
    public let amount: UInt64
    public let address: String
    public let scriptPubKey: Data
    public let blockHeight: Int64?
    public let confirmations: Int
    
    public var isConfirmed: Bool {
        confirmations >= 6
    }
    
    public var isSpendable: Bool {
        isConfirmed
    }
    
    public init(
        txid: String,
        outputIndex: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data,
        blockHeight: Int64? = nil,
        confirmations: Int = 0
    ) {
        self.txid = txid
        self.outputIndex = outputIndex
        self.amount = amount
        self.address = address
        self.scriptPubKey = scriptPubKey
        self.blockHeight = blockHeight
        self.confirmations = confirmations
    }
}

// UTXO selection for transaction building
public struct UTXOSelection {
    public let selectedUTXOs: [UTXO]
    public let totalAmount: UInt64
    public let fee: UInt64
    public let change: UInt64
    
    public init(
        selectedUTXOs: [UTXO],
        totalAmount: UInt64,
        fee: UInt64,
        change: UInt64
    ) {
        self.selectedUTXOs = selectedUTXOs
        self.totalAmount = totalAmount
        self.fee = fee
        self.change = change
    }
    
    public var inputAmount: UInt64 {
        selectedUTXOs.reduce(0) { $0 + $1.amount }
    }
    
    public var isValid: Bool {
        inputAmount >= totalAmount + fee
    }
}

// UTXO selector for optimal coin selection
public struct UTXOSelector {
    public static func selectUTXOs(
        from available: [UTXO],
        targetAmount: UInt64,
        feePerByte: UInt64 = 1
    ) -> UTXOSelection? {
        // Filter to only confirmed UTXOs
        let spendable = available.filter { $0.isSpendable }
        
        // Sort by amount (largest first for now - could implement better algorithms)
        let sorted = spendable.sorted { $0.amount > $1.amount }
        
        var selected: [UTXO] = []
        var totalSelected: UInt64 = 0
        
        // Simple selection - take UTXOs until we have enough
        for utxo in sorted {
            selected.append(utxo)
            totalSelected += utxo.amount
            
            // Estimate fee (simplified - real implementation would be more complex)
            let estimatedSize = (selected.count * 148) + (2 * 34) + 10 // inputs + outputs + overhead
            let estimatedFee = UInt64(estimatedSize) * feePerByte
            
            if totalSelected >= targetAmount + estimatedFee {
                let change = totalSelected - targetAmount - estimatedFee
                return UTXOSelection(
                    selectedUTXOs: selected,
                    totalAmount: targetAmount,
                    fee: estimatedFee,
                    change: change
                )
            }
        }
        
        // Not enough funds
        return nil
    }
}