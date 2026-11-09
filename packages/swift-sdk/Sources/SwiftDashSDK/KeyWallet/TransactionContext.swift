import Foundation

public enum TransactionContextType: UInt32 {
    case mempool = 0
    case instantSend = 1
    case inBlock = 2
    case inChainLockedBlock = 3
}
