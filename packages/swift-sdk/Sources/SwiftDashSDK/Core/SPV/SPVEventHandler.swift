import Foundation
import DashSDKFFI

internal enum SPVSyncManager: UInt32, Sendable {
    case headers = 0
    case filterHeaders = 1
    case filters = 2
    case blocks = 3
    case masternodes = 4
    case chainLocks = 5
    case instantSend = 6
    case unknown = 999
}

// Swift compatible type with C const uint8_t (*foo)[32]
private typealias Byte32 = (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)

// MARK: - Progress callback

internal protocol SPVProgressUpdateEventHandler: AnyObject {
  func onProgressUpdate(_ progress: SPVSyncProgress)
  
  func intoFFIProgressCallback() -> FFIProgressCallback
}

extension SPVProgressUpdateEventHandler {
  func intoFFIProgressCallback() -> FFIProgressCallback {
      return FFIProgressCallback(
          on_progress: onSpvProgressUpdateCallbackC,
          user_data: Unmanaged.passUnretained(self).toOpaque()
      )
  }
}

private func onSpvProgressUpdateCallbackC(
    progressPtr: UnsafePointer<FFISyncProgress>?,
    userData: UnsafeMutableRawPointer?
) {
    let spvProgressUpdateEventHandler = rawPtrIntoSpvProgressUpdateEventHandler(userData)

    let spvSyncProgress = ffiSyncProgressPtrIntoSpvSyncProgress(progressPtr)

    spvProgressUpdateEventHandler.onProgressUpdate(spvSyncProgress)
}

private func rawPtrIntoSpvProgressUpdateEventHandler(
    _ ptr: UnsafeMutableRawPointer?
) -> any SPVProgressUpdateEventHandler {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "SPVProgressUpdateEventHandler pointer is nil!")
        return DummySPVProgressUpdateEventHandler()
    }

    return Unmanaged<AnyObject>.fromOpaque(ptr).takeUnretainedValue() as! any SPVProgressUpdateEventHandler
}

private final class DummySPVProgressUpdateEventHandler: SPVProgressUpdateEventHandler {
    func onProgressUpdate(_ progress: SPVSyncProgress) {}

    func intoFFIProgressCallback() -> FFIProgressCallback {
        .init()
    }
}

// MARK: - Sync event callbacks

internal protocol SPVSyncEventsHandler: AnyObject {
  func onStart(_ manager: SPVSyncManager)
  func onComplete(_ headerTip: UInt32)
  func onBlockHeadersStored(_ tipHeight: UInt32)
  func onBlockHeadersSyncCompleted(_ tipHeight: UInt32)
  func onFilterHeadersStored(_ startHeight: UInt32, _ endHeight: UInt32, _ tipHeight: UInt32)
  func onFilterHeadersSyncCompleted(_ tipHeight: UInt32)
  func onFilterStored(_ startHeight: UInt32, _ endHeight: UInt32)
  func onFilterSyncCompleted(_ tipHeight: UInt32)
  func onBlocksNeeded(_ height: UInt32, _ hash: Data, _ count: UInt32)
  func onBlocksProcessed(_ height: UInt32, _ hash: Data, _ newAddressCount: UInt32)
  func onMasternodeStateUpdated(_ height: UInt32)
  func onChainLockReceived(_ height: UInt32, _ hash: Data, _ validated: Bool)
  func onInstantLockReceived(_ txid: Data, _ validated: Bool)
  func onSyncManagerError(_ manager: SPVSyncManager, _ errorMsg: String)
  
  func intoFFISyncEventCallbacks() -> FFISyncEventCallbacks
}

extension SPVSyncEventsHandler {
  func intoFFISyncEventCallbacks() -> FFISyncEventCallbacks {
    FFISyncEventCallbacks(
      on_sync_start: onSpvSyncStartCallbackC,
      on_block_headers_stored: onSpvBlockHeadersStoredCallbackC,
      on_block_header_sync_complete: onSpvBlockHeaderSyncCompletedCallbackC,
      on_filter_headers_stored: onSpvFilterHeadersStoredCallbackC,
      on_filter_headers_sync_complete: onSpvFilterHeadersSyncCompletedCallbackC,
      on_filters_stored: onSpvFiltersStoredCallbackC,
      on_filters_sync_complete: onSpvFiltersSyncCompletedCallbackC,
      on_blocks_needed: onSpvBlocksNeededCallbackC,
      on_block_processed: onSpvBlockProcessedCallbackC,
      on_masternode_state_updated: onSpvMasternodeStateUpdatedCallbackC,
      on_chainlock_received: onSpvChainLockReceivedCallbackC,
      on_instantlock_received: onSpvInstantLockReceivedCallbackC,
      on_manager_error: onSpvSyncManagerErrorCallbackC,
      on_sync_complete: onSpvSyncCompleteCallbackC,
      
      user_data: Unmanaged.passUnretained(self).toOpaque()
    )
  }
}

private func onSpvSyncStartCallbackC(
    manager: FFIManagerId,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvSyncEventsHandler(userData)

    let spvSyncManager = SPVSyncManager(rawValue: manager.rawValue) ?? .unknown

    handler.onStart(spvSyncManager)
}

private func onSpvSyncCompleteCallbackC(
    headerTip: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData).onComplete(headerTip)
}

private func onSpvBlockHeadersStoredCallbackC(
    tipHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onBlockHeadersStored(tipHeight)
}

private func onSpvBlockHeaderSyncCompletedCallbackC(
    tipHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onBlockHeadersSyncCompleted(tipHeight)
}

private func onSpvFilterHeadersStoredCallbackC(
    startHeight: UInt32,
    endHeight: UInt32,
    tipHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onFilterHeadersStored(startHeight, endHeight, tipHeight)
}

private func onSpvFilterHeadersSyncCompletedCallbackC(
    tipHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onFilterHeadersSyncCompleted(tipHeight)
}

private func onSpvFiltersStoredCallbackC(
    startHeight: UInt32,
    endHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onFilterStored(startHeight, endHeight)
}

private func onSpvFiltersSyncCompletedCallbackC(
    tipHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onFilterSyncCompleted(tipHeight)
}

private func onSpvBlocksNeededCallbackC(
    block: UnsafePointer<FFIBlockNeeded>?,
    count: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    guard let block else {
        assert(false, "FFIBlockNeeded pointer is nil!")
        return
    }
    
    let blockHeight = block.pointee.height;
    let blockHash = withUnsafeBytes(of: block.pointee.hash) { Data($0) }
  
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onBlocksNeeded(blockHeight, blockHash, count)
}

private func onSpvBlockProcessedCallbackC(
    height: UInt32,
    hashPtr: UnsafePointer<Byte32>?,
    newAddressCount: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    let hash = byte32PtrIntoData(hashPtr)
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onBlocksProcessed(height, hash, newAddressCount)
}

private func onSpvMasternodeStateUpdatedCallbackC(
    height: UInt32,
    userData: UnsafeMutableRawPointer?
) {
    rawPtrIntoSpvSyncEventsHandler(userData)
        .onMasternodeStateUpdated(height)
}

private func onSpvChainLockReceivedCallbackC(
    height: UInt32,
    hashPtr: UnsafePointer<Byte32>?,
    validated: Bool,
    userData: UnsafeMutableRawPointer?
) {
    let hash = byte32PtrIntoData(hashPtr)

    rawPtrIntoSpvSyncEventsHandler(userData)
        .onChainLockReceived(height, hash, validated)
}

private func onSpvInstantLockReceivedCallbackC(
    txidPtr: UnsafePointer<Byte32>?,
    validated: Bool,
    userData: UnsafeMutableRawPointer?
) {
    let txid = byte32PtrIntoData(txidPtr)

    rawPtrIntoSpvSyncEventsHandler(userData)
        .onInstantLockReceived(txid, validated)
}

private func onSpvSyncManagerErrorCallbackC(
    manager: FFIManagerId,
    errorPtr: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvSyncEventsHandler(userData)

    let syncManager = SPVSyncManager(rawValue: manager.rawValue) ?? .unknown
    let errorMsg = errorPtr.map { String(cString: $0) } ?? "Unknown error"

    handler.onSyncManagerError(syncManager, errorMsg)
}

private func rawPtrIntoSpvSyncEventsHandler(_ ptr: UnsafeMutableRawPointer?) -> any SPVSyncEventsHandler {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "SPVSyncEventsHandler pointer is nil!")
        return DummySPVSyncEventsHandler()
    }

    return Unmanaged<AnyObject>.fromOpaque(ptr).takeUnretainedValue() as! any SPVSyncEventsHandler
}

private final class DummySPVSyncEventsHandler: SPVSyncEventsHandler {
    func onStart(_ manager: SPVSyncManager) {}
    func onComplete(_ headerTip: UInt32) {}
    func onBlockHeadersStored(_ tipHeight: UInt32) {}
    func onBlockHeadersSyncCompleted(_ tipHeight: UInt32) {}
    func onFilterHeadersStored(_ startHeight: UInt32, _ endHeight: UInt32, _ tipHeight: UInt32) {}
    func onFilterHeadersSyncCompleted(_ tipHeight: UInt32) {}
    func onFilterStored(_ startHeight: UInt32, _ endHeight: UInt32) {}
    func onFilterSyncCompleted(_ tipHeight: UInt32) {}
    func onBlocksNeeded(_ height: UInt32, _ hash: Data, _ count: UInt32) {}
    func onBlocksProcessed(_ height: UInt32, _ hash: Data, _ newAddressCount: UInt32) {}
    func onMasternodeStateUpdated(_ height: UInt32) {}
    func onChainLockReceived(_ height: UInt32, _ hash: Data, _ validated: Bool) {}
    func onInstantLockReceived(_ txid: Data, _ validated: Bool) {}
    func onSyncManagerError(_ manager: SPVSyncManager, _ errorMsg: String) {}

    func intoFFISyncEventCallbacks() -> FFISyncEventCallbacks {
        .init()
    }
}

// MARK: - Network event callbacks

internal protocol SPVNetworkEventsHandler: AnyObject {
    func onPeerConnected(_ address: String)
    func onPeerDisconnected(_ address: String)
    func onPeersUpdated(_ connectedCount: UInt32, _ bestHeight: UInt32)

    func intoFFINetworkEventCallbacks() -> FFINetworkEventCallbacks
}

extension SPVNetworkEventsHandler {
    func intoFFINetworkEventCallbacks() -> FFINetworkEventCallbacks {
        FFINetworkEventCallbacks(
            on_peer_connected: onSpvPeerConnectedCallbackC,
            on_peer_disconnected: onSpvPeerDisconnectedCallbackC,
            on_peers_updated: onSpvPeersUpdatedCallbackC,
            user_data: Unmanaged.passUnretained(self).toOpaque()
        )
    }
}

private func onSpvPeerConnectedCallbackC(
    addressPtr: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvNetworkEventsHandler(userData)

    guard let addressPtr else {
        assert(false, "PeerConnected address pointer is nil")
        return
    }

    let address = String(cString: addressPtr)
    handler.onPeerConnected(address)
}

private func onSpvPeerDisconnectedCallbackC(
    addressPtr: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvNetworkEventsHandler(userData)

    guard let addressPtr else {
        assert(false, "PeerDisconnected address pointer is nil")
        return
    }

    let address = String(cString: addressPtr)
    handler.onPeerDisconnected(address)
}

private func onSpvPeersUpdatedCallbackC(
    connectedCount: UInt32,
    bestHeight: UInt32,
    userData: UnsafeMutableRawPointer?
) {
      rawPtrIntoSpvNetworkEventsHandler(userData)
          .onPeersUpdated(
            connectedCount,
            bestHeight
          )
}

private func rawPtrIntoSpvNetworkEventsHandler(
    _ ptr: UnsafeMutableRawPointer?
) -> any SPVNetworkEventsHandler {
    guard let ptr else {
        assert(false, "SPVNetworkEventsHandler pointer is nil!")
        return DummySPVNetworkEventsHandler()
    }

    return Unmanaged<AnyObject>
        .fromOpaque(ptr)
        .takeUnretainedValue() as! any SPVNetworkEventsHandler
}

private final class DummySPVNetworkEventsHandler: SPVNetworkEventsHandler {
    func onPeerConnected(_ address: String) {}
    func onPeerDisconnected(_ address: String) {}
    func onPeersUpdated(_ connectedCount: UInt32, _ bestHeight: UInt32) {}

    func intoFFINetworkEventCallbacks() -> FFINetworkEventCallbacks {
        .init()
    }
}

// MARK: - Wallet event callbacks

internal protocol SPVWalletEventsHandler: AnyObject {
    func onTransactionReceived(
        _ walletId: String,
        _ accountIndex: UInt32,
        _ txid: Data,
        _ amount: Int64,
        _ addresses: [String]
    )

    func onBalanceUpdated(
        _ walletId: String,
        _ spendable: UInt64,
        _ unconfirmed: UInt64,
        _ immature: UInt64,
        _ locked: UInt64
    )

    func intoFFIWalletEventCallbacks() -> FFIWalletEventCallbacks
}

extension SPVWalletEventsHandler {
    func intoFFIWalletEventCallbacks() -> FFIWalletEventCallbacks {
        FFIWalletEventCallbacks(
            on_transaction_received: onSpvTransactionReceivedCallbackC,
            on_balance_updated: onSpvBalanceUpdatedCallbackC,
            user_data: Unmanaged.passUnretained(self).toOpaque()
        )
    }
}

private func onSpvTransactionReceivedCallbackC(
    walletIdPtr: UnsafePointer<CChar>?,
    accountIndex: UInt32,
    txidPtr: UnsafePointer<Byte32>?,
    amount: Int64,
    addressesPtr: UnsafePointer<CChar>?,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvWalletEventsHandler(userData)

    guard let walletIdPtr else {
        assert(false, "TransactionReceived walletId pointer is nil")
        return
    }

    let walletId = String(cString: walletIdPtr)
    let txid = byte32PtrIntoData(txidPtr)
    let addresses = addressesPtrIntoString(addressesPtr)

    handler.onTransactionReceived(
        walletId,
        accountIndex,
        txid,
        amount,
        addresses
    )
}

private func onSpvBalanceUpdatedCallbackC(
    walletIdPtr: UnsafePointer<CChar>?,
    spendable: UInt64,
    unconfirmed: UInt64,
    immature: UInt64,
    locked: UInt64,
    userData: UnsafeMutableRawPointer?
) {
    let handler = rawPtrIntoSpvWalletEventsHandler(userData)

    guard let walletIdPtr else {
        assert(false, "BalanceUpdated walletId pointer is nil")
        return
    }

    let walletId = String(cString: walletIdPtr)

    handler.onBalanceUpdated(
        walletId,
        spendable,
        unconfirmed,
        immature,
        locked
    )
}

private func rawPtrIntoSpvWalletEventsHandler(
    _ ptr: UnsafeMutableRawPointer?
) -> any SPVWalletEventsHandler {
    guard let ptr else {
        assert(false, "SPVWalletEventsHandler pointer is nil!")
        return DummySPVWalletEventsHandler()
    }

    return Unmanaged<AnyObject>
        .fromOpaque(ptr)
        .takeUnretainedValue() as! any SPVWalletEventsHandler
}

private final class DummySPVWalletEventsHandler: SPVWalletEventsHandler {
    func onTransactionReceived(
        _ walletId: String,
        _ accountIndex: UInt32,
        _ txid: Data,
        _ amount: Int64,
        _ addresses: [String]
    ) {}

    func onBalanceUpdated(
        _ walletId: String,
        _ spendable: UInt64,
        _ unconfirmed: UInt64,
        _ immature: UInt64,
        _ locked: UInt64
    ) {}

    func intoFFIWalletEventCallbacks() -> FFIWalletEventCallbacks {
        .init()
    }
}

// MARK: - Helpers

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

public func ffiSyncProgressPtrIntoSpvSyncProgress(_ ptr: UnsafePointer<FFISyncProgress>?) -> SPVSyncProgress {
    guard let ptr else {
        // If the pointer in nil, a bug in the dash-spv library has occurred
        assert(false, "Progress pointer is nil!")
        return SPVSyncProgress.default()
    }

    return SPVSyncProgress(ptr.pointee)
}
