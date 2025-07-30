import Foundation

// Simple address manager for HD wallet addresses
public class AddressManager {
    
    public init() {}
    
    // Generate next address for an account
    public func generateNextAddress(account: HDAccount, type: AddressType, network: DashNetwork) -> HDAddress? {
        let existingAddresses = account.addresses.filter { $0.type == type }
        let nextIndex = UInt32(existingAddresses.count)
        
        // Use FFI to derive address
        let path = DerivationPath.dashBIP44(
            account: account.accountNumber,
            change: type == .internal ? 1 : 0,
            index: nextIndex,
            testnet: network == .testnet
        )
        
        // Generate address using placeholder for now
        let address = "y\(type == .internal ? "Internal" : "Address")\(account.accountNumber)\(nextIndex)"
        
        let hdAddress = HDAddress(
            address: address,
            index: nextIndex,
            derivationPath: path.stringRepresentation,
            addressType: type,
            account: account
        )
        
        return hdAddress
    }
    
    // Find unused addresses
    public func findUnusedAddresses(account: HDAccount, type: AddressType, count: Int = 20) -> [HDAddress] {
        return account.addresses
            .filter { $0.type == type && !$0.isUsed }
            .sorted { $0.index < $1.index }
            .prefix(count)
            .map { $0 }
    }
    
    // Check if address belongs to wallet
    public func isOurAddress(_ address: String, wallet: HDWallet) -> Bool {
        return wallet.accounts.flatMap { $0.addresses }.contains { $0.address == address }
    }
}