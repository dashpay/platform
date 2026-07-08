package org.dashfoundation.dashsdk.tokens

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TokensNative

/**
 * Token state-transition surface — the Android analog of the Swift
 * `ManagedPlatformWallet` token actions
 * (`packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/Tokens/TokenActions.swift`).
 *
 * Each of the 12 actions maps 1:1 onto a `platform_wallet_token_*` FFI entry
 * point (via [TokensNative]). They are `suspend` on [Dispatchers.IO] because
 * the native call blocks a worker thread while Rust broadcasts the state
 * transition and verifies the response proof on its own runtime.
 *
 * All actions require:
 * - [walletHandle]: a live `PlatformWallet` handle from
 *   `WalletManagerNative.getWallet` (the wallet that owns the acting identity).
 * - [signerHandle]: a native `SignerHandle` from `SignerNative.createSigner`.
 * - [signingKeyId]: the identity public key id to sign with.
 *
 * Group-gatable actions (mint, burn, freeze, unfreeze, destroyFrozenFunds,
 * pause, resume, setPrice, updateConfig) take a [GroupAction] describing
 * single-signer / propose / co-sign authorization; purchase, transfer and
 * claim are always single-signer.
 *
 * Return values are the raw JSON strings the FFI produces (balances maps for
 * mint/burn/transfer); the caller decodes them, mirroring how the Swift
 * wrappers hand JSON up to their callers.
 */
class Tokens internal constructor(private val walletHandle: Long) {

    /**
     * Mint [amount]; [issuedToIdentityId] null mints to [identityId].
     * Returns the post-mint balances JSON (keyed by the recipient), or null.
     */
    suspend fun mint(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: Long,
        issuedToIdentityId: ByteArray? = null,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): String? = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        require(amount > 0) { "amount must be positive, got $amount" }
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenMint(
                walletHandle, identityId, tokenContractId, tokenPosition,
                issuedToIdentityId, amount, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Burn [amount] from [identityId]. Returns the post-burn balances JSON, or null. */
    suspend fun burn(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: Long,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): String? = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        require(amount > 0) { "amount must be positive, got $amount" }
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenBurn(
                walletHandle, identityId, tokenContractId, tokenPosition,
                amount, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Transfer [amount] to [recipientId]. Returns the post-transfer balances JSON, or null. */
    suspend fun transfer(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        recipientId: ByteArray,
        amount: Long,
        publicNote: String? = null,
        signingKeyId: Int,
        signerHandle: Long,
    ): String? = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        require(amount > 0) { "amount must be positive, got $amount" }
        mapNativeErrors {
            TokensNative.tokenTransfer(
                walletHandle, identityId, tokenContractId, tokenPosition,
                recipientId, amount, publicNote, signingKeyId, signerHandle,
            )
        }
    }

    /** Freeze [frozenIdentityId]'s entire balance. */
    suspend fun freeze(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenFreeze(
                walletHandle, identityId, tokenContractId, tokenPosition,
                frozenIdentityId, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Unfreeze [frozenIdentityId]'s frozen balance. */
    suspend fun unfreeze(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenUnfreeze(
                walletHandle, identityId, tokenContractId, tokenPosition,
                frozenIdentityId, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Destroy [frozenIdentityId]'s frozen balance. */
    suspend fun destroyFrozenFunds(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        frozenIdentityId: ByteArray,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenDestroyFrozenFunds(
                walletHandle, identityId, tokenContractId, tokenPosition,
                frozenIdentityId, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Pause all operations for the token. */
    suspend fun pause(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenPause(
                walletHandle, identityId, tokenContractId, tokenPosition, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Resume all operations for the token. */
    suspend fun resume(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenResume(
                walletHandle, identityId, tokenContractId, tokenPosition, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Set the direct-purchase price. [pricePerToken] == 0 disables direct purchase. */
    suspend fun setPrice(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        pricePerToken: Long,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        require(pricePerToken >= 0) {
            "pricePerToken must be non-negative (0 disables direct purchase), got $pricePerToken"
        }
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenSetPrice(
                walletHandle, identityId, tokenContractId, tokenPosition,
                pricePerToken, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /** Purchase [amount] at the set price for [expectedTotalCost] credits. */
    suspend fun purchase(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        amount: Long,
        expectedTotalCost: Long,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        require(amount > 0) { "amount must be positive, got $amount" }
        require(expectedTotalCost > 0) {
            "expectedTotalCost must be positive, got $expectedTotalCost"
        }
        mapNativeErrors {
            TokensNative.tokenPurchase(
                walletHandle, identityId, tokenContractId, tokenPosition,
                amount, expectedTotalCost, signingKeyId, signerHandle,
            )
        }
    }

    /** Claim a distribution payout of [distributionType] (pre-programmed / perpetual). */
    suspend fun claim(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        distributionType: TokenDistributionType,
        publicNote: String? = null,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        mapNativeErrors {
            TokensNative.tokenClaim(
                walletHandle, identityId, tokenContractId, tokenPosition,
                distributionType.value, publicNote, signingKeyId, signerHandle,
            )
        }
    }

    /**
     * Update the token configuration. [change] renders the FFI change-item
     * tag + JSON payload (only [TokenConfigChange.MaxSupply] is wired in this
     * release, matching the FFI).
     */
    suspend fun updateConfig(
        identityId: ByteArray,
        tokenContractId: ByteArray,
        tokenPosition: Int,
        change: TokenConfigChange,
        publicNote: String? = null,
        groupAction: GroupAction = GroupAction.None,
        signingKeyId: Int,
        signerHandle: Long,
    ): Unit = withContext(Dispatchers.IO) {
        validateSelectors(tokenPosition, signingKeyId)
        val g = groupAction.flatten()
        mapNativeErrors {
            TokensNative.tokenUpdateConfig(
                walletHandle, identityId, tokenContractId, tokenPosition,
                change.tag, change.payloadJson, publicNote,
                g.kind, g.position, g.actionId, g.actionIsProposer,
                signingKeyId, signerHandle,
            )
        }
    }

    /**
     * Validate the selectors every token action carries before they cross
     * JNI: [tokenPosition] is a u16 contract position and [signingKeyId] an
     * identity key id (u32) — both signed ints Kotlin-side.
     */
    private fun validateSelectors(tokenPosition: Int, signingKeyId: Int) {
        require(tokenPosition in 0..0xFFFF) {
            "tokenPosition must be in 0..65535, got $tokenPosition"
        }
        require(signingKeyId >= 0) { "signingKeyId must be non-negative, got $signingKeyId" }
    }
}

/** Distribution type for [Tokens.claim] — mirrors the FFI discriminant. */
enum class TokenDistributionType(val value: Int) {
    PRE_PROGRAMMED(0),
    PERPETUAL(1),
}

/**
 * A token-configuration change for [Tokens.updateConfig], rendered to the
 * flat `(tag, payloadJson)` the FFI decodes
 * (`rs-platform-wallet-ffi/src/tokens/update_config.rs`). Only MaxSupply is
 * wired by the FFI in this release; other change items are a documented
 * future follow-up on the Rust side.
 */
sealed class TokenConfigChange {

    /** The FFI change-item tag. */
    abstract val tag: Int

    /** The per-tag JSON payload string. */
    abstract val payloadJson: String

    /**
     * Set (or, with [newMaxSupply] null, remove) the token's max supply.
     * Renders `{"newMaxSupply":"<u64>"}` or `{"newMaxSupply":null}`; the
     * value is a decimal string so the full u64 range survives (Long is
     * signed, so a value is passed as its unsigned decimal rendering).
     */
    data class MaxSupply(val newMaxSupply: Long?) : TokenConfigChange() {
        override val tag: Int = 0
        override val payloadJson: String =
            if (newMaxSupply == null) {
                "{\"newMaxSupply\":null}"
            } else {
                "{\"newMaxSupply\":\"${newMaxSupply.toULong()}\"}"
            }
    }
}
