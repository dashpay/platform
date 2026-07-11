import Foundation
import DashSDKFFI

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
                            context: TransactionContextType = .mempool,
                            blockHeight: UInt32 = 0,
                            blockHash: Data? = nil,
                            timestamp: UInt64 = 0,
                            updateState: Bool = true) throws -> TransactionCheckResult {
        var error = FFIError()
        var result = FFITransactionCheckResult()

        // Build FFIBlockInfo
        var blockInfo = FFIBlockInfo()
        blockInfo.height = blockHeight
        blockInfo.timestamp = UInt32(timestamp)
        if let hash = blockHash, hash.count == 32 {
            hash.withUnsafeBytes { buf in
                withUnsafeMutableBytes(of: &blockInfo.block_hash) { dst in
                    dst.copyBytes(from: buf.prefix(32))
                }
            }
        }

        let contextType = FFITransactionContextType(rawValue: context.rawValue)

        let success = transactionData.withUnsafeBytes { txBytes in
            let txPtr = txBytes.bindMemory(to: UInt8.self).baseAddress

            return wallet_check_transaction(
                wallet.ffiHandle,
                txPtr, transactionData.count,
                contextType, blockInfo,
                nil, 0, // islock_data, islock_len
                updateState, &result, &error)
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
