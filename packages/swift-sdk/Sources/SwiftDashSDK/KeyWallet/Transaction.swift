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
            // TODO: This memory is not being freed, FFI must free FFITxOutput
            // or expose a method to do it
            let cString = strdup(address)

            return FFITxOutput(address: cString, amount: amount)
        }
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