import Foundation
import DashSDKFFI

/// Swift wrapper for a collection of managed accounts
public class ManagedAccountCollection {
    private let handle: UnsafeMutablePointer<FFIManagedAccountCollection>
    private let manager: WalletManager
    
    internal init(handle: UnsafeMutablePointer<FFIManagedAccountCollection>, manager: WalletManager) {
        self.handle = handle
        self.manager = manager
    }
    
    deinit {
        managed_account_collection_free(handle)
    }
    
    // MARK: - BIP44 Accounts
    
    /// Get a BIP44 account by index
    /// - Parameter index: The account index
    /// - Returns: The managed account if it exists
    public func getBIP44Account(at index: UInt32) -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_bip44_account(handle, index) else {
            return nil
        }
        
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Get all BIP44 account indices
    public func getBIP44Indices() -> [UInt32] {
        var indices: UnsafeMutablePointer<UInt32>?
        var count: Int = 0
        
        let success = managed_account_collection_get_bip44_indices(handle, &indices, &count)
        
        guard success, let indicesPtr = indices, count > 0 else {
            return []
        }
        
        defer {
            indicesPtr.deallocate()
        }
        
        return Array(UnsafeBufferPointer(start: indicesPtr, count: count))
    }
    
    // MARK: - BIP32 Accounts
    
    /// Get a BIP32 account by index
    /// - Parameter index: The account index
    /// - Returns: The managed account if it exists
    public func getBIP32Account(at index: UInt32) -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_bip32_account(handle, index) else {
            return nil
        }
        
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Get all BIP32 account indices
    public func getBIP32Indices() -> [UInt32] {
        var indices: UnsafeMutablePointer<UInt32>?
        var count: Int = 0
        
        let success = managed_account_collection_get_bip32_indices(handle, &indices, &count)
        
        guard success, let indicesPtr = indices, count > 0 else {
            return []
        }
        
        defer {
            indicesPtr.deallocate()
        }
        
        return Array(UnsafeBufferPointer(start: indicesPtr, count: count))
    }
    
    // MARK: - CoinJoin Accounts
    
    /// Get a CoinJoin account by index
    /// - Parameter index: The account index
    /// - Returns: The managed account if it exists
    public func getCoinJoinAccount(at index: UInt32) -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_coinjoin_account(handle, index) else {
            return nil
        }
        
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Get all CoinJoin account indices
    public func getCoinJoinIndices() -> [UInt32] {
        var indices: UnsafeMutablePointer<UInt32>?
        var count: Int = 0
        
        let success = managed_account_collection_get_coinjoin_indices(handle, &indices, &count)
        
        guard success, let indicesPtr = indices, count > 0 else {
            return []
        }
        
        defer {
            indicesPtr.deallocate()
        }
        
        return Array(UnsafeBufferPointer(start: indicesPtr, count: count))
    }
    
    // MARK: - Identity Accounts
    
    /// Get the identity registration account
    public func getIdentityRegistrationAccount() -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_identity_registration(handle) else {
            return nil
        }
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if identity registration account exists
    public var hasIdentityRegistration: Bool {
        return managed_account_collection_has_identity_registration(handle)
    }
    
    /// Get an identity top-up account by registration index
    /// - Parameter registrationIndex: The registration index
    /// - Returns: The managed account if it exists
    public func getIdentityTopUpAccount(registrationIndex: UInt32) -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_identity_topup(handle, registrationIndex) else {
            return nil
        }
        
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Get all identity top-up account indices
    public func getIdentityTopUpIndices() -> [UInt32] {
        var indices: UnsafeMutablePointer<UInt32>?
        var count: Int = 0
        
        let success = managed_account_collection_get_identity_topup_indices(handle, &indices, &count)
        
        guard success, let indicesPtr = indices, count > 0 else {
            return []
        }
        
        defer {
            indicesPtr.deallocate()
        }
        
        return Array(UnsafeBufferPointer(start: indicesPtr, count: count))
    }
    
    /// Get the identity top-up not bound account
    public func getIdentityTopUpNotBoundAccount() -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_identity_topup_not_bound(handle) else {
            return nil
        }
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if identity top-up not bound account exists
    public var hasIdentityTopUpNotBound: Bool {
        return managed_account_collection_has_identity_topup_not_bound(handle)
    }
    
    /// Get the identity invitation account
    public func getIdentityInvitationAccount() -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_identity_invitation(handle) else {
            return nil
        }
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if identity invitation account exists
    public var hasIdentityInvitation: Bool {
        return managed_account_collection_has_identity_invitation(handle)
    }
    
    // MARK: - Provider Accounts
    
    /// Get the provider voting keys account
    public func getProviderVotingKeysAccount() -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_provider_voting_keys(handle) else {
            return nil
        }
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if provider voting keys account exists
    public var hasProviderVotingKeys: Bool {
        return managed_account_collection_has_provider_voting_keys(handle)
    }
    
    /// Get the provider owner keys account
    public func getProviderOwnerKeysAccount() -> ManagedAccount? {
        guard let accountHandle = managed_account_collection_get_provider_owner_keys(handle) else {
            return nil
        }
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if provider owner keys account exists
    public var hasProviderOwnerKeys: Bool {
        return managed_account_collection_has_provider_owner_keys(handle)
    }
    
    /// Get the provider operator keys account
    public func getProviderOperatorKeysAccount() -> ManagedAccount? {
        guard let rawPointer = managed_account_collection_get_provider_operator_keys(handle) else {
            return nil
        }
        let accountHandle = rawPointer.assumingMemoryBound(to: FFIManagedAccount.self)
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if provider operator keys account exists
    public var hasProviderOperatorKeys: Bool {
        return managed_account_collection_has_provider_operator_keys(handle)
    }
    
    /// Get the provider platform keys account
    public func getProviderPlatformKeysAccount() -> ManagedAccount? {
        guard let rawPointer = managed_account_collection_get_provider_platform_keys(handle) else {
            return nil
        }
        let accountHandle = rawPointer.assumingMemoryBound(to: FFIManagedAccount.self)
        return ManagedAccount(handle: accountHandle, manager: manager)
    }
    
    /// Check if provider platform keys account exists
    public var hasProviderPlatformKeys: Bool {
        return managed_account_collection_has_provider_platform_keys(handle)
    }
    
    // MARK: - Summary
    
    /// Get a summary of all accounts in this collection
    public func getSummary() -> ManagedAccountCollectionSummary? {
        guard let summaryPtr = managed_account_collection_summary_data(handle) else {
            return nil
        }
        
        defer {
            managed_account_collection_summary_free(summaryPtr)
        }
        
        return ManagedAccountCollectionSummary(ffiSummary: summaryPtr.pointee)
    }
}
