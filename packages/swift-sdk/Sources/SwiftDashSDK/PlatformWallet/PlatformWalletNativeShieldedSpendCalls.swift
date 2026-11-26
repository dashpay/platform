import Foundation
import DashSDKFFI

/// Native entry points behind the admitted shielded spend wrappers on
/// [`PlatformWalletManager`] (pool spends with and without a plan, the spend
/// plan, shields and the identity-debit abandonment). Same design as [`PlatformWalletNativeTeardownCalls`]:
/// injecting the functions keeps each wrapper's real body, its admission and
/// its result mapping under test without calling Rust. Every field mirrors
/// its C export's imported Swift type, so `.live` is the exports themselves.
/// The unchecked conformance is intentional: the closures are immutable
/// C-entry wrappers, and the whole value is copied into the wrapper's
/// admitted detached task.
///
/// The asset-lock pool fundings and the pool seeding are not here: they are
/// deliberately not admitted until `shutdown()` can cancel them (#6003).
///
/// Tests with a fake manager handle must inject this table: handles come from
/// a process-global registry, so an arbitrary non-zero test value is not
/// guaranteed to miss a live Rust entry owned by another test.
struct PlatformWalletNativeShieldedSpendCalls: @unchecked Sendable {
    typealias WalletId = UnsafePointer<UInt8>?
    typealias Resolver = UnsafeMutablePointer<MnemonicResolverHandle>?
    /// A plan's 32-byte digest (`ShieldedSpendPlan.digest`).
    typealias Digest = UnsafePointer<UInt8>?

    /// `platform_wallet_manager_shielded_plan_spend`.
    typealias PlanSpend = @Sendable (
        Handle, WalletId, UInt32, UInt8, UInt64, Bool, UInt32,
        UnsafeMutablePointer<ShieldedSpendPlanFFI>?
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_transfer_with_plan`.
    typealias TransferWithPlan = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<UInt8>?, UInt64,
        UnsafePointer<CChar>?, Digest
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_unshield_with_plan`.
    typealias UnshieldWithPlan = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<CChar>?, UInt64, Digest
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_identity_top_up_from_pool_with_plan`.
    typealias IdentityTopUpFromPoolWithPlan = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<UInt8>?, UInt64, Digest
    ) -> PlatformWalletFFIResult
    /// `platform_wallet_manager_shielded_withdraw_with_plan`.
    typealias WithdrawWithPlan = @Sendable (
        Handle, WalletId, Resolver, UInt32, UnsafePointer<CChar>?, UInt64, UInt32, Digest
    ) -> PlatformWalletFFIResult

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
    /// `platform_wallet_manager_abandon_shielded_identity_debit`.
    typealias AbandonIdentityDebit = @Sendable (
        Handle, WalletId, UInt32, UnsafePointer<UInt8>?, Bool
    ) -> PlatformWalletFFIResult

    let planSpend: PlanSpend
    let transfer: Transfer
    let transferWithPlan: TransferWithPlan
    let shield: Shield
    let shieldFromIdentity: ShieldFromIdentity
    let shieldToRecipient: ShieldToRecipient
    let unshield: Unshield
    let unshieldWithPlan: UnshieldWithPlan
    let identityTopUpFromPool: IdentityTopUpFromPool
    let identityTopUpFromPoolWithPlan: IdentityTopUpFromPoolWithPlan
    let tokenPurchaseFromPool: TokenPurchaseFromPool
    let withdraw: Withdraw
    let withdrawWithPlan: WithdrawWithPlan
    let identityCreateFromPool: IdentityCreateFromPool
    let abandonIdentityDebit: AbandonIdentityDebit

    static let live = PlatformWalletNativeShieldedSpendCalls(
        planSpend: platform_wallet_manager_shielded_plan_spend,
        transfer: platform_wallet_manager_shielded_transfer,
        transferWithPlan: platform_wallet_manager_shielded_transfer_with_plan,
        shield: platform_wallet_manager_shielded_shield,
        shieldFromIdentity: platform_wallet_manager_shielded_shield_from_identity,
        shieldToRecipient: platform_wallet_manager_shielded_shield_to_recipient,
        unshield: platform_wallet_manager_shielded_unshield,
        unshieldWithPlan: platform_wallet_manager_shielded_unshield_with_plan,
        identityTopUpFromPool: platform_wallet_manager_shielded_identity_top_up_from_pool,
        identityTopUpFromPoolWithPlan:
            platform_wallet_manager_shielded_identity_top_up_from_pool_with_plan,
        tokenPurchaseFromPool: platform_wallet_manager_shielded_token_purchase_from_pool,
        withdraw: platform_wallet_manager_shielded_withdraw,
        withdrawWithPlan: platform_wallet_manager_shielded_withdraw_with_plan,
        identityCreateFromPool: platform_wallet_manager_shielded_identity_create_from_pool,
        abandonIdentityDebit: platform_wallet_manager_abandon_shielded_identity_debit
    )
}
