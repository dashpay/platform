import Foundation
import DashSDKFFI

// This struct is not mapping all fields of FFITransactionRecord
// for the lack of wrappers
public struct NotOwnedTransactionRecord {
    let txid: Data
    let net_amount: Int64
    let context: TransactionContext
    let fee: UInt64
    let tx_data: Data
    let label: String?

    public init(handle: UnsafePointer<FFITransactionRecord>) {
      let p = handle.pointee

      self.txid = withUnsafeBytes(of: p.txid) { Data($0) }
      self.net_amount = p.net_amount
      self.fee = p.fee
      self.tx_data = p.tx_data != nil
          ? Data(bytes: p.tx_data, count: p.tx_len)
          : Data()
      self.label = p.label != nil
          ? String(cString: p.label)
          : nil
      self.context = TransactionContext(ffi: p.context)
    }
}
