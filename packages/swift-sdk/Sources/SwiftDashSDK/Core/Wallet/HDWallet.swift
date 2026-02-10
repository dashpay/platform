import Foundation
import SwiftData

// MARK: - HD Wallet

@Model
public final class HDWallet {
    @Attribute(.unique) public var id: UUID
    public var label: String
    public var network: AppNetwork
    public var createdAt: Date
    public var isWatchOnly: Bool
    public var isImported: Bool
    
    // FFI Wallet ID (32 bytes) - links to the rust-dashcore wallet
    @Attribute(.unique) public var walletId: Data
    
    // Serialized wallet bytes from FFI - used to restore wallet on app restart
    @Attribute(.unique) public var serializedWalletBytes: Data
    
    init(walletId: Data, serializedWalletBytes: Data, label: String, network: AppNetwork, isWatchOnly: Bool = false, isImported: Bool = false) {
        self.id = UUID()
        self.label = label
        self.network = network
        self.createdAt = Date()
        self.isWatchOnly = isWatchOnly
        self.isImported = isImported
        self.walletId = walletId
        self.serializedWalletBytes = serializedWalletBytes
    }
}
