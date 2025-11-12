import Foundation

public struct SDKBuiltTransaction {
    public let txid: String
    public let rawTransaction: Data
    public let fee: UInt64
}

public enum SDKTxError: LocalizedError {
    case notImplemented(String)
    case invalidInput(String)
    case invalidState(String)

    public var errorDescription: String? {
        switch self {
        case .notImplemented(let msg): return msg
        case .invalidInput(let msg): return msg
        case .invalidState(let msg): return msg
        }
    }
}

