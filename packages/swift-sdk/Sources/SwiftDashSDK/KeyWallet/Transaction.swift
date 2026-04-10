import Foundation
import DashSDKFFI

/// Transaction utilities for wallet operations
public class Transaction {

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