import DashSDKFFI

/// Swift names for the `direction` wire values of `PersistentTransaction`
/// and `PersistentTransactionAccounting`, bound to the constants the Rust
/// FFI exports (`PLATFORM_WALLET_TRANSACTION_DIRECTION_*`), never re-declared.
public enum CoreDirectionCode {
    public static let incoming = UInt32(PLATFORM_WALLET_TRANSACTION_DIRECTION_INCOMING)
    public static let outgoing = UInt32(PLATFORM_WALLET_TRANSACTION_DIRECTION_OUTGOING)
    public static let internalTransfer = UInt32(PLATFORM_WALLET_TRANSACTION_DIRECTION_INTERNAL)
    public static let coinJoin = UInt32(PLATFORM_WALLET_TRANSACTION_DIRECTION_COIN_JOIN)
}
