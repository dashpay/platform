import Foundation
import DashSDKFFI

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
            return address.withCString { addressCStr in
                FFITxOutput(address: addressCStr, amount: amount)
            }
        }
    }
    
    /// Build a transaction
    /// - Parameters:
    ///   - wallet: The wallet to build from
    ///   - accountIndex: The account index to use
    ///   - outputs: The transaction outputs
    ///   - feePerKB: Fee per kilobyte in satoshis
    /// - Returns: The unsigned transaction bytes
    public static func build(wallet: Wallet,
                            accountIndex: UInt32 = 0,
                            outputs: [Output],
                            feePerKB: UInt64) throws -> Data {
        guard !outputs.isEmpty else {
            throw KeyWalletError.invalidInput("Transaction must have at least one output")
        }
        
        var error = FFIError()
        var txBytesPtr: UnsafeMutablePointer<UInt8>?
        var txLen: size_t = 0
        
        // Convert outputs to FFI format
        let ffiOutputs = outputs.map { $0.toFFI() }
        
        let success = ffiOutputs.withUnsafeBufferPointer { outputsPtr in
            wallet_build_transaction(
                wallet.ffiHandle,
                NetworkSet(wallet.network).ffiNetworks,
                accountIndex,
                outputsPtr.baseAddress,
                outputs.count,
                feePerKB,
                &txBytesPtr,
                &txLen,
                &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = txBytesPtr {
                transaction_bytes_free(ptr)
            }
        }
        
        guard success, let ptr = txBytesPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        // Copy the transaction data before freeing
        let txData = Data(bytes: ptr, count: txLen)
        
        return txData
    }
    
    /// Sign a transaction
    /// - Parameters:
    ///   - wallet: The wallet to sign with
    ///   - transactionData: The unsigned transaction bytes
    /// - Returns: The signed transaction bytes
    public static func sign(wallet: Wallet, transactionData: Data) throws -> Data {
        guard !wallet.isWatchOnly else {
            throw KeyWalletError.invalidState("Cannot sign with watch-only wallet")
        }
        
        var error = FFIError()
        var signedTxPtr: UnsafeMutablePointer<UInt8>?
        var signedLen: size_t = 0
        
        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            return wallet_sign_transaction(
                wallet.ffiHandle,
                NetworkSet(wallet.network).ffiNetworks,
                txPtr, transactionData.count,
                &signedTxPtr, &signedLen, &error)
        }
        
        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = signedTxPtr {
                transaction_bytes_free(ptr)
            }
        }
        
        guard success, let ptr = signedTxPtr else {
            throw KeyWalletError(ffiError: error)
        }
        
        // Copy the signed transaction data before freeing
        let signedData = Data(bytes: ptr, count: signedLen)
        
        return signedData
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
                        wallet.network.ffiValue,
                        txPtr, transactionData.count,
                        context.ffiValue, blockHeight, hashPtr,
                        timestamp, updateState, &result, &error)
                }
            } else {
                return wallet_check_transaction(
                    wallet.ffiHandle,
                    wallet.network.ffiValue,
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