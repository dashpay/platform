import Foundation
import DashSDKFFI

/// The C progress callback `platform_wallet_manager_shielded_seed_pool_notes`
/// takes, as Swift imports it.
typealias SeedShieldedPoolProgressFn = @convention(c) (
    UnsafeMutableRawPointer?, UInt64, UInt64, UInt64, UInt64
) -> Void

/// Native entry points behind the admitted shielded spend wrappers on
/// [`PlatformWalletManager`] (pool spends, shields, pool funding and the
/// identity-debit abandonment). Same design as
/// [`PlatformWalletNativeTeardownCalls`]: injecting the functions keeps each
/// wrapper's real body, its admission and its result mapping under test
/// without calling Rust. Every field mirrors its C export's imported Swift
/// type, so `.live` is the exports themselves. The unchecked conformance is
/// intentional: the closures are immutable C-entry wrappers, and the whole
/// value is copied into the wrapper's detached task.
///
/// Tests with a fake manager handle must inject this table: handles come from
/// a process-global registry, so an arbitrary non-zero test value is not
/// guaranteed to miss a live Rust entry owned by another test.
struct PlatformWalletNativeShieldedSpendCalls: @unchecked Sendable {
    typealias WalletId = UnsafePointer<UInt8>?
    typealias Resolver = UnsafeMutablePointer<MnemonicResolverHandle>?

    /// `platform_wallet_manager_shielded_transfer`.
    typealias Transfer = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<UInt8>?, UInt64,
        UnsafePointer<CChar>?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_shield`.
    typealias Shield = @Sendable (
        Handle, WalletId, UInt32, UInt32, UInt64, OpaquePointer?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_shield_from_identity`.
    typealias ShieldFromIdentity = @Sendable (
        Handle, WalletId, UInt32, UnsafePointer<UInt8>?, UInt64, OpaquePointer?,
        UnsafeMutablePointer<UInt64>?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_shield_to_recipient`.
    typealias ShieldToRecipient = @Sendable (
        Handle, WalletId, UInt32, UInt32, UnsafePointer<UInt8>?, UInt64,
        UnsafePointer<CChar>?, OpaquePointer?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_unshield`.
    typealias Unshield = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<CChar>?, UInt64
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_identity_top_up_from_pool`.
    typealias IdentityTopUpFromPool = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<UInt8>?, UInt64
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_token_purchase_from_pool`.
    typealias TokenPurchaseFromPool = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<UInt8>?, UInt16, UInt64,
        UInt64
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_withdraw`.
    typealias Withdraw = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<CChar>?, UInt64, UInt32
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_identity_create_from_pool`.
    typealias IdentityCreateFromPool = @Sendable (
        Handle, WalletId, Resolver, UInt32, UInt32, UnsafePointer<IdentityPubkeyFFI>?,
        UInt, UInt64, UnsafePointer<UInt8>?, OpaquePointer?,
        UnsafeMutablePointer<FFIByteTuple32>?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_fund_from_asset_lock`.
    typealias FundFromAssetLock = @Sendable (
        Handle, WalletId, UInt32, UInt64, UnsafePointer<UInt8>?, UnsafePointer<UInt8>?,
        UInt, Resolver, AssetLockBuiltFn?, UnsafeMutableRawPointer?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_fund_from_asset_lock_coinjoin_drain`.
    typealias FundFromCoinJoinDrain = @Sendable (
        Handle, WalletId, UInt32, UnsafePointer<UInt8>?, Resolver, AssetLockBuiltFn?,
        UnsafeMutableRawPointer?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_seed_pool_notes`.
    typealias SeedPoolNotes = @Sendable (
        Handle, WalletId, UInt32, UInt64, UInt32, Resolver, SeedShieldedPoolProgressFn?,
        UnsafeMutableRawPointer?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_abandon_shielded_identity_debit`.
    typealias AbandonIdentityDebit = @Sendable (
        Handle, WalletId, UInt32, UnsafePointer<UInt8>?, Bool
    ) -> PlatformWalletFFIResult

    let transfer: Transfer
    let shield: Shield
    let shieldFromIdentity: ShieldFromIdentity
    let shieldToRecipient: ShieldToRecipient
    let unshield: Unshield
    let identityTopUpFromPool: IdentityTopUpFromPool
    let tokenPurchaseFromPool: TokenPurchaseFromPool
    let withdraw: Withdraw
    let identityCreateFromPool: IdentityCreateFromPool
    let fundFromAssetLock: FundFromAssetLock
    let fundFromCoinJoinDrain: FundFromCoinJoinDrain
    let seedPoolNotes: SeedPoolNotes
    let abandonIdentityDebit: AbandonIdentityDebit

    static let live = PlatformWalletNativeShieldedSpendCalls(
        transfer: platform_wallet_manager_shielded_transfer,
        shield: platform_wallet_manager_shielded_shield,
        shieldFromIdentity: platform_wallet_manager_shielded_shield_from_identity,
        shieldToRecipient: platform_wallet_manager_shielded_shield_to_recipient,
        unshield: platform_wallet_manager_shielded_unshield,
        identityTopUpFromPool: platform_wallet_manager_shielded_identity_top_up_from_pool,
        tokenPurchaseFromPool: platform_wallet_manager_shielded_token_purchase_from_pool,
        withdraw: platform_wallet_manager_shielded_withdraw,
        identityCreateFromPool: platform_wallet_manager_shielded_identity_create_from_pool,
        fundFromAssetLock: platform_wallet_manager_shielded_fund_from_asset_lock,
        fundFromCoinJoinDrain:
            platform_wallet_manager_shielded_fund_from_asset_lock_coinjoin_drain,
        seedPoolNotes: platform_wallet_manager_shielded_seed_pool_notes,
        abandonIdentityDebit: platform_wallet_manager_abandon_shielded_identity_debit
    )
}
