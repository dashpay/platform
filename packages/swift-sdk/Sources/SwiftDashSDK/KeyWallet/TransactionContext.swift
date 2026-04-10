import Foundation
import DashSDKFFI

public enum TransactionContextType: UInt32 {
    case mempool = 0
    case instantSend = 1
    case inBlock = 2
    case inChainLockedBlock = 3

    var ffiValue: FFITransactionContextType {
        FFITransactionContextType(rawValue: self.rawValue)
    }

    init(ffiContext: FFITransactionContextType) {
        self = TransactionContextType(rawValue: ffiContext.rawValue) ?? .mempool
    }
}

public class BlockInfo {
    let height: UInt32
    let block_hash: Data
    let timestamp: UInt32

    init(ffi: FFIBlockInfo) {
        self.height = ffi.height
        self.block_hash = withUnsafeBytes(of: ffi.block_hash) { Data($0) }
        self.timestamp = ffi.timestamp
    }
}

public class TransactionContext {
    let context_type: TransactionContextType
    let block_info: BlockInfo
    let islock_data: Data

    init(ffi: FFITransactionContext) {
        self.context_type = TransactionContextType(ffiContext: ffi.context_type)
        self.block_info = BlockInfo(ffi: ffi.block_info)
        if let islockPtr = ffi.islock_data, ffi.islock_len > 0 {
            self.islock_data = Data(bytes: islockPtr, count: ffi.islock_len)
        } else {
            self.islock_data = Data()
        }
    }
}