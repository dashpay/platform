import Foundation

public enum TransactionError: LocalizedError {
    case invalidState
    case noInputs
    case noOutputs
    case insufficientFunds
    case invalidAddress
    case invalidInput(String)
    case invalidOutput(String)
    case noChangeAddress
    case signingFailed
    case serializationFailed
    case broadcastFailed(String)
    case notSupported(String)

    public var errorDescription: String? {
        switch self {
        case .invalidState:
            return "Transaction in invalid state"
        case .noInputs:
            return "Transaction has no inputs"
        case .noOutputs:
            return "Transaction has no outputs"
        case .insufficientFunds:
            return "Insufficient funds for transaction"
        case .invalidAddress:
            return "Invalid recipient address"
        case .invalidInput(let message):
            return "Invalid input: \(message)"
        case .invalidOutput(let message):
            return "Invalid output: \(message)"
        case .noChangeAddress:
            return "No change address specified"
        case .signingFailed:
            return "Failed to sign transaction"
        case .serializationFailed:
            return "Failed to serialize transaction"
        case .broadcastFailed(let message):
            return "Failed to broadcast: \(message)"
        case .notSupported(let msg):
            return msg
        }
    }
}

// Transaction object used by the example app
public struct BuiltTransaction {
    public let txid: String
    public let rawTransaction: Data
    public let fee: UInt64
    public let inputs: [HDUTXO]
    public let changeAmount: UInt64
}

// Common hex initializer used by transaction code
extension Data {
    init?(hex: String) {
        let hex = hex.replacingOccurrences(of: " ", with: "")
        guard hex.count % 2 == 0 else { return nil }
        var data = Data(capacity: hex.count / 2)
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            guard next <= hex.endIndex else { return nil }
            let byteString = String(hex[index..<next])
            guard let num = UInt8(byteString, radix: 16) else { return nil }
            data.append(num)
            index = next
        }
        self = data
    }
}
