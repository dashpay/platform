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
}