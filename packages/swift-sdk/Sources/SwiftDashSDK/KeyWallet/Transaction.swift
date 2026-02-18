import Foundation
import DashSDKFFI

public enum FeeRate {
    case economy
    case normal
    case priority
    
    func intoFFI() -> FFIFeeRate {
        switch self {
        case .economy: return FFIFeeRate(0)
        case .normal: return FFIFeeRate(1)
        case .priority: return FFIFeeRate(2)
        }
    }
}

/// Transaction utilities for wallet operations
public class Transaction {
    
    /// Transaction output for building transactions
    public struct Output {
        public let address: String
        public let amount: UInt64
        
        public init(address: String, amount: UInt64) {
            self.address = address
            self.amount = amount
        }
        
        func toFFI() -> FFITxOutput {
            let cString = strdup(address)
            return FFITxOutput(address: cString, amount: amount)
        }
    }
    
    /// Check if a transaction belongs to a wallet
    /// - Parameters:
    ///   - wallet: The wallet to check against
    ///   - transactionData: The transaction bytes
    ///   - context: The transaction context
    ///   - blockHeight: The block height (0 for mempool)
    ///   - blockHash: The block hash (nil for mempool)
    ///   - timestamp: The timestamp
    ///   - updateState: Whether to update wallet state if transaction is relevant
    /// - Returns: Transaction check result
    public static func check(wallet: Wallet,
                            transactionData: Data,
                            context: TransactionContext = .mempool,
                            blockHeight: UInt32 = 0,
                            blockHash: Data? = nil,
                            timestamp: UInt64 = 0,
                            updateState: Bool = true) throws -> TransactionCheckResult {
        var error = FFIError()
        var result = FFITransactionCheckResult()
        
        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            
            if let hash = blockHash {
                return hash.withUnsafeBytes { hashBytes in
                    let hashPtr = hashBytes.bindMemory(to: UInt8.self).baseAddress
                    
                    return wallet_check_transaction(
                        wallet.ffiHandle,
                        txPtr, transactionData.count,
                        context.ffiValue, blockHeight, hashPtr,
                        timestamp, updateState, &result, &error)
                }
            } else {
                return wallet_check_transaction(
                    wallet.ffiHandle,
                    txPtr, transactionData.count,
                    context.ffiValue, blockHeight, nil,
                    timestamp, updateState, &result, &error)
            }
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            transaction_check_result_free(&result)
        }
        
        guard success else {
            throw KeyWalletError(ffiError: error)
        }
        
        return TransactionCheckResult(ffiResult: result)
    }
    
    /// Classify a transaction for routing
    /// - Parameter transactionData: The transaction bytes
    /// - Returns: A string describing the transaction type
    public static func classify(_ transactionData: Data) throws -> String {
        var error = FFIError()
        
        let classificationPtr = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            return transaction_classify(txPtr, transactionData.count, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
        }
        
        guard let ptr = classificationPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        let classification = String(cString: ptr)
        string_free(ptr)
        
        return classification
    }
}
