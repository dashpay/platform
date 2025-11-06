import Foundation
import Darwin
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
        // Convert outputs to FFI format with stable C strings
        var cStrings: [UnsafeMutablePointer<CChar>] = []
        cStrings.reserveCapacity(outputs.count)
        var ffiOutputs: [FFITxOutput] = []
        ffiOutputs.reserveCapacity(outputs.count)

        for output in outputs {
            let cstr = output.address.withCString { strdup($0) }
            guard let cstr else {
                // Free any previously allocated strings before throwing
                for ptr in cStrings { free(ptr) }
                throw KeyWalletError.invalidInput("Failed to allocate C string for address")
            }
            cStrings.append(cstr)
            ffiOutputs.append(FFITxOutput(address: UnsafePointer(cstr), amount: output.amount))
        }

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
            // Free allocated C strings
            for ptr in cStrings { free(ptr) }
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
        
        let success = transactionData.withUnsafeBytes { (txPtr: UnsafePointer<UInt8>) in
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

    /// Build and sign a transaction in one step using managed wallet
    /// - Parameters:
    ///   - managedWallet: The managed wallet with UTXO information
    ///   - wallet: The wallet with private keys for signing
    ///   - accountIndex: The account index to use
    ///   - outputs: The transaction outputs
    ///   - feePerKB: Fee per kilobyte in satoshis
    ///   - currentHeight: Current blockchain height for UTXO selection
    /// - Returns: The signed transaction bytes ready for broadcast
    public static func buildAndSign(managedWallet: ManagedWallet,
                                   wallet: Wallet,
                                   accountIndex: UInt32 = 0,
                                   outputs: [Output],
                                   feePerKB: UInt64,
                                   currentHeight: UInt32) throws -> Data {
        guard !outputs.isEmpty else {
            throw KeyWalletError.invalidInput("Transaction must have at least one output")
        }

        guard !wallet.isWatchOnly else {
            throw KeyWalletError.invalidState("Cannot sign with watch-only wallet")
        }

        var error = FFIError()
        var txBytesPtr: UnsafeMutablePointer<UInt8>?
        var txLen: size_t = 0

        // Get managed wallet handle
        guard let managedHandle = managedWallet.getInfoHandle() else {
            throw KeyWalletError.invalidState("Failed to get managed wallet handle")
        }

        // Convert outputs to FFI format with stable C strings
        var cStrings: [UnsafeMutablePointer<CChar>] = []
        cStrings.reserveCapacity(outputs.count)
        var ffiOutputs: [FFITxOutput] = []
        ffiOutputs.reserveCapacity(outputs.count)

        for output in outputs {
            let cstr = output.address.withCString { strdup($0) }
            guard let cstr else {
                for ptr in cStrings { free(ptr) }
                throw KeyWalletError.invalidInput("Failed to allocate C string for address")
            }
            cStrings.append(cstr)
            ffiOutputs.append(FFITxOutput(address: UnsafePointer(cstr), amount: output.amount))
        }

        let success = ffiOutputs.withUnsafeBufferPointer { outputsPtr in
            wallet_build_and_sign_transaction(
                managedHandle,
                wallet.ffiHandle,
                wallet.network.ffiValue,
                accountIndex,
                outputsPtr.baseAddress,
                outputs.count,
                feePerKB,
                currentHeight,
                &txBytesPtr,
                &txLen,
                &error)
        }

        defer {
            for ptr in cStrings { free(ptr) }
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

    /// Extract TXID from raw transaction bytes
    /// - Parameter transactionData: The transaction bytes
    /// - Returns: The transaction ID as a hex string
    public static func getTxid(from transactionData: Data) throws -> String {
        var error = FFIError()
        var txidPtr: UnsafeMutablePointer<CChar>?

        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress
            txidPtr = transaction_get_txid_from_bytes(txPtr, transactionData.count, &error)
            return txidPtr != nil
        }

        defer {
            if error.message != nil {
                error_message_free(error.message)
            }
            if let ptr = txidPtr {
                string_free(ptr)
            }
        }

        guard success, let ptr = txidPtr else {
            throw KeyWalletError(ffiError: error)
        }

        return String(cString: ptr)
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
        
        let success = transactionData.withUnsafeBytes { (txPtr: UnsafePointer<UInt8>) in
            
            if let hash = blockHash {
                return hash.withUnsafeBytes { (hashPtr: UnsafePointer<UInt8>) in
                    
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
        
        let classificationPtr = transactionData.withUnsafeBytes { (txPtr: UnsafePointer<UInt8>) in
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
