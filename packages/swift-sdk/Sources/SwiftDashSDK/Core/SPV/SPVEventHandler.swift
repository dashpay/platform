import Foundation
import DashSDKFFI

/// swift callbacks that are triggered by the SPVClient
/// 
/// Important limitations:
/// - Only didUpdateSyncProgress and didCompleteSync work with the 
///   current SPVClient implementation, how the other events are triggered
///   is subject to change in the SPVClient sync rewrite 

internal protocol SPVEventHandler: AnyObject {
    func spvClient(didUpdateSyncProgress progress: SPVSyncProgress)
    func spvClient(didReceiveBlock block: SPVBlockEvent)
    func spvClient(didReceiveTransaction transaction: SPVTransactionEvent)
    func spvClient(didCompleteSync success: Bool, error: String?)
    func spvClient(didChangeConnectionStatus connected: Bool, peers: Int)
    func spvClient(didUpdateBlocksHit count: Int)
    
    func intoFFIEventCallbacks() -> FFIEventCallbacks
}

internal extension SPVEventHandler {
    /// Binds the swift callbacks into the FFIEventCallbacks struct by using 
    /// C-compatible function pointers and returns struct ready to be used in
    /// the SPVClient init process
    func intoFFIEventCallbacks() -> FFIEventCallbacks {
        var callbacks = FFIEventCallbacks()

        callbacks.user_data = Unmanaged.passUnretained(self).toOpaque()
        
        callbacks.on_block = onBlockCallbackC
        callbacks.on_transaction = onTransactionCallbackC
        callbacks.on_compact_filter_matched = onCompactFilterMatchedCallbackC
        callbacks.on_mempool_transaction_added = onMempoolTransactionAddedCallbackC
        callbacks.on_mempool_transaction_confirmed = onMempoolTransactionConfirmedCallbackC
        callbacks.on_mempool_transaction_removed = onMempoolTransactionRemovedCallbackC
        callbacks.on_wallet_transaction = onWalletTransactionCallbackC
        
        return callbacks
    }
}

private class DummySPVEventHandler: SPVEventHandler {
    func spvClient(didUpdateSyncProgress progress: SPVSyncProgress) {}
    func spvClient(didReceiveBlock block: SPVBlockEvent) {}
    func spvClient(didReceiveTransaction transaction: SPVTransactionEvent) {}
    func spvClient(didCompleteSync success: Bool, error: String?) {}
    func spvClient(didChangeConnectionStatus connected: Bool, peers: Int) {}
    func spvClient(didUpdateBlocksHit count: Int) {}
}

// MARK: - C Callback Functions

// Swift compatible type with C const uint8_t (*foo)[32]
private typealias Byte32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

func onSpvProgressCallbackC(
    progressPtr: UnsafePointer<FFIDetailedSyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let spvSyncProgress = ffiSyncProgressPtrIntoSpvSyncProgress(progressPtr)
    
    spvEventHandler.spvClient(didUpdateSyncProgress: spvSyncProgress)
}

func onSpvCompletionCallbackC(
    success: Bool,
    errorMsg: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let errorString: String? = errorMsg.map { String(cString: $0) }
 
    spvEventHandler.spvClient(didCompleteSync: success, error: errorString)
}

private func onBlockCallbackC(
    _ height: UInt32,
    _ hashPtr: UnsafePointer<Byte32>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let hash = byte32PtrIntoData(hashPtr)

    let block = SPVBlockEvent(
        height: height,
        hash: hash,
        timestamp: Date()
    )
    
    spvEventHandler.spvClient(didReceiveBlock: block)
}

private func onTransactionCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)
    
    let txid = byte32PtrIntoData(txidPtr)
    
    let addresses = addressesPtrIntoString(addressesPtr)
    
    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: confirmed,
        amount: amount,
        addresses: addresses,
        blockHeight: blockHeight
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onCompactFilterMatchedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ scripts: UnsafePointer<CChar>?,
    _ wallet: UnsafePointer<CChar>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    spvEventHandler.spvClient(didUpdateBlocksHit: 1)
}

private func onMempoolTransactionAddedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ amount: Int64,
    _ addressPtr: UnsafePointer<CChar>?,
    _ isInstantSend: Bool,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)

    let addresses = addressesPtrIntoString(addressPtr)

    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: false,
        amount: amount,
        addresses: addresses,
        blockHeight: 0
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onMempoolTransactionConfirmedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ blockHeight: UInt32,
    _ blockHashPtr: UnsafePointer<Byte32>?,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)

    // Amount and addresses are not provided here; emit a confirmation-only update
    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: true,
        amount: 0,
        addresses: [],
        blockHeight: blockHeight
    )
    
    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func onMempoolTransactionRemovedCallbackC(
    _ txidPtr: UnsafePointer<Byte32>?,
    _ reason: UInt8,
    _ userData: UnsafeMutableRawPointer?
) { 
    // Intentionally no-op; could surface to UI in future if needed
}

private func onWalletTransactionCallbackC(
    _ walletId: UnsafePointer<CChar>?,
    _ accountIndex: UInt32,
    _ txidPtr: UnsafePointer<Byte32>?,
    _ confirmed: Bool,
    _ amount: Int64,
    _ addressesPtr: UnsafePointer<CChar>?,
    _ blockHeight: UInt32,
    _ isOurs: Bool,
    _ userData: UnsafeMutableRawPointer?
) {
    let spvEventHandler = rawPtrIntoSpvEventHandler(userData)

    let txid = byte32PtrIntoData(txidPtr)
    
    let addresses = addressesPtrIntoString(addressesPtr)

    let transaction = SPVTransactionEvent(
        txid: txid,
        confirmed: confirmed,
        amount: amount,
        addresses: addresses,
        blockHeight: blockHeight
    )

    spvEventHandler.spvClient(didReceiveTransaction: transaction) 
}

private func byte32PtrIntoData(_ ptr: UnsafePointer<Byte32>?) -> Data {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Byte32 pointer is nil!")
        return Data()
    }
    
    return Data(bytes: ptr, count: 32)
}

private func addressesPtrIntoString(_ ptr: UnsafePointer<CChar>?) -> [String] {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Addresses pointer is nil!")
        return [""]
    }
    
    let str = String(cString: ptr)
    return str.components(separatedBy: ",")
}

private func rawPtrIntoSpvEventHandler(_ ptr: UnsafeMutableRawPointer?) -> any SPVEventHandler {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "SPVEventHandler pointer is nil!")
        return DummySPVEventHandler()
    }

    return Unmanaged<AnyObject>.fromOpaque(ptr).takeUnretainedValue() as! any SPVEventHandler
}

public func ffiSyncProgressPtrIntoSpvSyncProgress(_ ptr: UnsafePointer<FFIDetailedSyncProgress>?) -> SPVSyncProgress {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Progress pointer is nil!")
        return SPVSyncProgress.default()
    }

    return SPVSyncProgress.from(ptr.pointee)
}

// MARK: - SPV Event Types

public struct SPVBlockEvent {
    public let height: UInt32
    public let hash: Data
    public let timestamp: Date
}

public struct SPVTransactionEvent {
    public let txid: Data
    public let confirmed: Bool
    public let amount: Int64
    public let addresses: [String]
    public let blockHeight: UInt32?
}