import Foundation

// MARK: - Detailed Balance

/// Balance with additional metadata about the wallet
public struct DetailedBalance: Equatable, Sendable {
    /// The balance amounts
    public let balance: Balance

    /// Number of addresses in the wallet
    public let addressCount: Int

    /// Number of unspent transaction outputs
    public let utxoCount: Int

    /// When this balance was last updated
    public let lastUpdated: Date

    public init(
        balance: Balance,
        addressCount: Int = 0,
        utxoCount: Int = 0,
        lastUpdated: Date = Date()
    ) {
        self.balance = balance
        self.addressCount = addressCount
        self.utxoCount = utxoCount
        self.lastUpdated = lastUpdated
    }
}

// MARK: - UTXO Selection

/// Result of UTXO selection for transaction building
public struct UTXOSelection: Sendable {
    /// UTXOs selected for the transaction
    public let selectedUTXOs: [UTXO]

    /// Target amount to send (excluding fee)
    public let totalAmount: UInt64

    /// Estimated transaction fee
    public let fee: UInt64

    /// Change amount to return to sender
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

    /// Total amount available from selected UTXOs
    public var inputAmount: UInt64 {
        selectedUTXOs.reduce(0) { $0 + $1.amount }
    }

    /// Whether the selection covers the target amount plus fee
    public var isValid: Bool {
        inputAmount >= totalAmount + fee
    }
}

// MARK: - UTXO Selector

/// Utility for selecting optimal UTXOs for transaction building
public struct UTXOSelector {

    /// Select UTXOs to cover a target amount plus fees
    /// - Parameters:
    ///   - available: Available UTXOs to choose from
    ///   - targetAmount: Amount to send (in duffs)
    ///   - feePerByte: Fee rate in duffs per byte
    /// - Returns: UTXO selection if sufficient funds available, nil otherwise
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
