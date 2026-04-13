import Foundation
import DashSDKFFI

public enum TransactionType {
    case standard
    case coinJoin
    case providerRegistration
    case providerUpdateRegister
    case providerUpdateService
    case providerUpdateRevocation
    case assetLock
    case assetUnlock
    case coinbase
    case ignored

    init(ffi: FFITransactionType) {
        switch ffi {
        case FFI_TRANSACTION_TYPE_STANDARD: self = .standard
        case FFI_TRANSACTION_TYPE_COIN_JOIN: self = .coinJoin
        case FFI_TRANSACTION_TYPE_PROVIDER_REGISTRATION: self = .providerRegistration
        case FFI_TRANSACTION_TYPE_PROVIDER_UPDATE_REGISTRAR: self = .providerUpdateRegister
        case FFI_TRANSACTION_TYPE_PROVIDER_UPDATE_SERVICE: self = .providerUpdateService
        case FFI_TRANSACTION_TYPE_PROVIDER_UPDATE_REVOCATION: self = .providerUpdateRevocation
        case FFI_TRANSACTION_TYPE_ASSET_LOCK: self = .assetLock
        case FFI_TRANSACTION_TYPE_ASSET_UNLOCK: self = .assetUnlock
        case FFI_TRANSACTION_TYPE_COINBASE: self = .coinbase
        case FFI_TRANSACTION_TYPE_IGNORED: self = .ignored
        default: fatalError("Unknown FFITransactionType value: \(ffi)")
        }
    }
}

public enum TransactionDirection {
    case incoming
    case outgoing
    case internalDir
    case coinjoin

    init(ffi: FFITransactionDirection) {
        switch ffi {
        case FFI_TRANSACTION_DIRECTION_INCOMING: self = .incoming
        case FFI_TRANSACTION_DIRECTION_OUTGOING: self = .outgoing
        case FFI_TRANSACTION_DIRECTION_INTERNAL: self = .internalDir
        case FFI_TRANSACTION_DIRECTION_COIN_JOIN: self = .coinjoin
        default: fatalError("Unknown FFITransactionDirection value: \(ffi)")
        }
    }
}

public struct InputDetail {
    public let index: UInt32
    public let value: UInt64
    public let address: String

    public init(ffi: FFIInputDetail) {
        self.index = ffi.index
        self.value = ffi.value
        self.address = ffi.address != nil
            ? String(cString: ffi.address)
            : ""
    }
}

public enum OutputRole {
    case received
    case change
    case sent
    case unspendable

    init(ffi: FFIOutputRole) {
        switch ffi {
        case FFI_OUTPUT_ROLE_RECEIVED: self = .received
        case FFI_OUTPUT_ROLE_CHANGE: self = .change
        case FFI_OUTPUT_ROLE_SENT: self = .sent
        case FFI_OUTPUT_ROLE_UNSPENDABLE: self = .unspendable
        default: fatalError("Unknown FFIOutputRole value: \(ffi)")
        }
    }
}

public struct OutputDetail {
    public let index: UInt32
    public let role: OutputRole

    public init(ffi: FFIOutputDetail) {
        self.index = ffi.index
        self.role = OutputRole(ffi: ffi.role)
    }
}

public struct NotOwnedTransactionRecord {
    public let txid: Data
    public let netAmount: Int64
    public let context: TransactionContext
    public let transactionType: TransactionType
    public let direction: TransactionDirection
    public let fee: UInt64
    public let inputDetails: [InputDetail]
    public let outputDetails: [OutputDetail]
    public let txData: Data
    public let label: String?

    public init(handle: UnsafePointer<FFITransactionRecord>) {
      let p = handle.pointee

      self.txid = withUnsafeBytes(of: p.txid) { Data($0) }
      self.netAmount = p.net_amount
      self.fee = p.fee
      self.txData = p.tx_data != nil
          ? Data(bytes: p.tx_data, count: p.tx_len)
          : Data()
      self.label = p.label != nil
          ? String(cString: p.label)
          : nil

      self.context = TransactionContext(ffi: p.context)
      self.transactionType = TransactionType(ffi: p.transaction_type)
      self.direction = TransactionDirection(ffi: p.direction)

      self.inputDetails = p.input_details != nil
          ? (0..<p.input_details_count).map { i in
              InputDetail(ffi: p.input_details[i])
          }
          : []

      self.outputDetails = p.output_details != nil
          ? (0..<p.output_details_count).map { i in
              OutputDetail(ffi: p.output_details[i])
          }
          : []
    }
}
