// Swift-side mirrors of the repr(C) structs in
// `rs-platform-wallet-ffi/src/core_wallet_types.rs`.
//
// These structs are never constructed on the Swift side — the Rust
// persister allocates them, fires the `on_persist_wallet_changeset_fn`
// callback with a pointer, and the Swift handler casts the pointer to
// walk the data. Rust owns all heap allocations and frees them after
// the callback returns.

import Foundation

// 32-byte tuple type used for txids / block hashes.
typealias FFIHash32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

struct OutPointFFI {
    var txid: FFIHash32
    var vout: UInt32
}

struct ChainChangeSetFFI {
    var has_synced_height: Bool
    var synced_height: UInt32
    var has_block_hash: Bool
    var block_hash: FFIHash32
}

struct BalanceChangeSetFFI {
    var confirmed_delta: Int64
    var unconfirmed_delta: Int64
    var immature_delta: Int64
    var locked_delta: Int64
}

struct UtxoEntryFFI {
    var outpoint: OutPointFFI
    var amount: UInt64
    var address: UnsafeMutablePointer<CChar>?
    var script_pubkey: UnsafeMutablePointer<UInt8>?
    var script_pubkey_len: Int
    var height: UInt32
    var is_coinbase: Bool
    var is_confirmed: Bool
    var is_instantlocked: Bool
    var is_locked: Bool
}

struct TransactionRecordFFI {
    var txid: FFIHash32
    var tx_data: UnsafeMutablePointer<UInt8>?
    var tx_data_len: Int
    var context: UInt32
    var block_height: UInt32
    var block_hash: FFIHash32
    var block_timestamp: UInt32
    var direction: UInt32
    var transaction_type: UnsafeMutablePointer<CChar>?
    var net_amount: Int64
    var fee: UInt64
    var has_fee: Bool
    var label: UnsafeMutablePointer<CChar>?
    var first_seen: UInt64
}

struct AccountChangeSetFFI {
    var account_type_name: UnsafeMutablePointer<CChar>?
    var account_index: UInt32
    var utxos_added: UnsafeMutablePointer<UtxoEntryFFI>?
    var utxos_added_count: Int
    var utxos_spent: UnsafeMutablePointer<OutPointFFI>?
    var utxos_spent_count: Int
    var utxos_instant_locked: UnsafeMutablePointer<OutPointFFI>?
    var utxos_instant_locked_count: Int
    var transactions: UnsafeMutablePointer<TransactionRecordFFI>?
    var transactions_count: Int
    var external_highest_used: Int32
    var has_external_highest_used: Bool
    var internal_highest_used: Int32
    var has_internal_highest_used: Bool
}

struct WalletChangeSetFFI {
    var has_chain: Bool
    var chain: ChainChangeSetFFI
    var has_balance: Bool
    var balance: BalanceChangeSetFFI
    var accounts: UnsafeMutablePointer<AccountChangeSetFFI>?
    var accounts_count: Int
}

// MARK: - Helpers

/// Convert a 32-byte FFI tuple into `Data` for SwiftData persistence.
func hashData(_ hash: FFIHash32) -> Data {
    withUnsafeBytes(of: hash) { Data($0) }
}

/// Hex-encode a 32-byte FFI tuple.
func hashHex(_ hash: FFIHash32) -> String {
    withUnsafeBytes(of: hash) { buf in
        buf.map { String(format: "%02x", $0) }.joined()
    }
}
