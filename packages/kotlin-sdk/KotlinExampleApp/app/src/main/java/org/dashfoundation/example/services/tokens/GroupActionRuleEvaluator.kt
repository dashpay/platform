package org.dashfoundation.example.services.tokens

import org.dashfoundation.dashsdk.persistence.entities.TokenEntity

/**
 * Group-gating inference for token action forms — the Kotlin port of
 * `inferredGroupPosition` in `TokenMintActionView.swift:191-207` (and its
 * clones across the burn / freeze / unfreeze / destroy / pause / resume /
 * set-price / update-max-supply forms), parameterized by the action's
 * rules JSON column instead of a concrete view.
 *
 * Also hosts the multi-rule position sweep from
 * `TokenGroupRuleResolver.relevantGroupPositions`
 * (PendingGroupActionsView.swift:239-300), which drives the
 * pending-group-actions discovery loop.
 */
object GroupActionRuleEvaluator {

    /** u16 range guard — group positions cross the FFI as u16. */
    private const val MAX_GROUP_POSITION = 0xFFFF

    /**
     * Banner state for a group-gatable action form: either the action is
     * single-signer ([None]) or submitting will propose a group action at
     * [Propose.groupPosition] (rendered as the iOS "Group action" banner).
     */
    sealed interface BannerState {
        data object None : BannerState
        data class Propose(val groupPosition: Int) : BannerState
    }

    /**
     * If the action governed by [rulesJson] is gated on a group, return
     * the group position the caller should propose under; otherwise null
     * (single-signer). Line-for-line port of
     * `TokenMintActionView.inferredGroupPosition`:
     *  - no rule → null
     *  - `MainGroup` → [mainControlGroupPosition] when set and u16-ranged
     *  - `Group:<n>` → n when u16-ranged
     *  - anything else (NoOne / ContractOwner / Identity:<id>) → null
     */
    fun inferredGroupPosition(rulesJson: String?, mainControlGroupPosition: Int?): Int? {
        val rule = ChangeControlRules.parse(rulesJson) ?: return null
        return inferredGroupPositionForTaker(rule.authorizedToMakeChange, mainControlGroupPosition)
    }

    /** Same inference from an already-decoded `authorizedToMakeChange` string. */
    fun inferredGroupPositionForTaker(authorized: String, mainControlGroupPosition: Int?): Int? {
        if (authorized == AuthorizedActionTakers.MAIN_GROUP) {
            val main = mainControlGroupPosition ?: return null
            return main.takeIf { it in 0..MAX_GROUP_POSITION }
        }
        if (authorized.startsWith(AuthorizedActionTakers.GROUP_PREFIX)) {
            val pos = authorized
                .removePrefix(AuthorizedActionTakers.GROUP_PREFIX)
                .toIntOrNull() ?: return null
            return pos.takeIf { it in 0..MAX_GROUP_POSITION }
        }
        return null
    }

    /** [BannerState] for one action's rules column. */
    fun bannerState(rulesJson: String?, mainControlGroupPosition: Int?): BannerState =
        inferredGroupPosition(rulesJson, mainControlGroupPosition)
            ?.let { BannerState.Propose(it) }
            ?: BannerState.None

    /**
     * Every group position referenced by the token's group-capable rules,
     * deduped in first-hit order — port of
     * `TokenGroupRuleResolver.relevantGroupPositions(for:)`. Drives both
     * the "Pending Group Actions" section visibility and the per-position
     * discovery loop.
     */
    fun relevantGroupPositions(token: TokenEntity): List<Int> {
        val ordered = LinkedHashSet<Int>()
        for (rulesJson in groupCapableRules(token)) {
            inferredGroupPosition(rulesJson, token.mainControlGroupPosition)
                ?.let(ordered::add)
        }
        return ordered.toList()
    }

    /**
     * The rules whose actions ship with `GroupAction.Propose` — mirror of
     * `TokenGroupRuleResolver.groupCapableRules`. The direct-purchase
     * pricing rule rides inside the distribution-change-rules JSON blob.
     */
    private fun groupCapableRules(token: TokenEntity): List<String?> = listOf(
        token.manualMintingRules,
        token.manualBurningRules,
        token.freezeRules,
        token.unfreezeRules,
        token.destroyFrozenFundsRules,
        token.emergencyActionRules,
        token.maxSupplyChangeRules,
        TokenDistributionChangeRules.parse(token.distributionChangeRules)
            ?.changeDirectPurchasePricingRules?.toJson(),
    )
}
