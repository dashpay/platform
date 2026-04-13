import Foundation
import DashSDKFFI

public class TxOutput {
    let address: UnsafeMutablePointer<CChar>
    let amount: UInt64

    public init(address: String, amount: UInt64) {
        let address = address.trimmingCharacters(in: .whitespacesAndNewlines)
        self.address = strdup(address)
        self.amount = amount
    }

    deinit {
        free(address)
    }

    func toFFI() -> FFITxOutput {
        return FFITxOutput(address: address, amount: amount)
    }
}