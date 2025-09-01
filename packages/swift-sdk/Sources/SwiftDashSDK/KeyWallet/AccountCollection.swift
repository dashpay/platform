import Foundation
import DashSDKFFI

/// Swift wrapper for a collection of accounts
public class AccountCollection {
    private let handle: OpaquePointer
    private weak var wallet: Wallet?
    
    internal init(handle: OpaquePointer, wallet: Wallet) {
        self.handle = handle
        self.wallet = wallet
    }
    
    deinit {
        account_collection_free(handle)
    }
    
    // MARK: - Provider Accounts (BLS)
    
    /// Get the provider operator keys account (BLS)
    public func getProviderOperatorKeys() -> BLSAccount? {
        guard let rawPointer = account_collection_get_provider_operator_keys(handle) else {
            return nil
        }
        let accountHandle = OpaquePointer(rawPointer)
        return BLSAccount(handle: accountHandle, wallet: wallet)
    }
    
    // MARK: - Provider Accounts (EdDSA)
    
    /// Get the provider platform keys account (EdDSA)
    public func getProviderPlatformKeys() -> EdDSAAccount? {
        guard let rawPointer = account_collection_get_provider_platform_keys(handle) else {
            return nil
        }
        let accountHandle = OpaquePointer(rawPointer)
        return EdDSAAccount(handle: accountHandle, wallet: wallet)
    }
    
    // MARK: - Summary
    
    /// Get a summary of all accounts in this collection
    public func getSummary() -> AccountCollectionSummary? {
        guard let summaryPtr = account_collection_summary_data(handle) else {
            return nil
        }
        
        defer {
            account_collection_summary_free(summaryPtr)
        }
        
        return AccountCollectionSummary(ffiSummary: summaryPtr.pointee)
    }
}