import Foundation
import DashSDKFFI

/// Fee rate for transaction building
public enum FeeRate: UInt32, Sendable {
    case economy = 0
    case normal = 1
    case priority = 2

    var ffiValue: FFIFeeRate {
        FFIFeeRate(rawValue: self.rawValue)
    }
}

/// Result of building and signing a transaction
public struct BuildAndSignResult: Sendable {
    /// The signed transaction bytes
    public let transactionData: Data
    /// The fee paid in duffs
    public let fee: UInt64
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
            return address.withCString { addressCStr in
                FFITxOutput(address: addressCStr, amount: amount)
            }
        }
    }

    /// Build and sign a transaction in a single operation
    ///
    /// This uses the wallet manager's coin selection and key derivation to build
    /// a fully signed transaction ready for broadcast.
    ///
    /// - Parameters:
    ///   - manager: The wallet manager that manages the wallet
    ///   - wallet: The wallet to build and sign from
    ///   - accountIndex: The BIP44 account index to use
    ///   - outputs: The transaction outputs
    ///   - feeRate: The fee rate level (economy, normal, or priority)
    /// - Returns: A result containing the signed transaction bytes and the fee paid
    public static func buildAndSign(manager: WalletManager,
                                    wallet: Wallet,
                                    accountIndex: UInt32 = 0,
                                    outputs: [Output],
                                    feeRate: FeeRate = .normal) throws -> BuildAndSignResult {
        guard !outputs.isEmpty else {
            throw KeyWalletError.invalidInput("Transaction must have at least one output")
        }

        guard !wallet.isWatchOnly else {
            throw KeyWalletError.invalidState("Cannot build and sign with watch-only wallet")
        }

        var error = FFIError()
        var txBytesPtr: UnsafeMutablePointer<UInt8>?
        var txLen: size_t = 0
        var feeOut: UInt64 = 0

        // Convert outputs to FFI format
        let ffiOutputs = outputs.map { $0.toFFI() }

        let success = ffiOutputs.withUnsafeBufferPointer { outputsPtr in
            wallet_build_and_sign_transaction(
                manager.ffiHandle,
                wallet.ffiHandle,
                accountIndex,
                outputsPtr.baseAddress,
                outputs.count,
                feeRate.ffiValue,
                &feeOut,
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

        return BuildAndSignResult(transactionData: txData, fee: feeOut)
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
