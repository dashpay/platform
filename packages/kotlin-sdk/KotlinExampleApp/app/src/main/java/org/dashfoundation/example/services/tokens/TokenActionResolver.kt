package org.dashfoundation.example.services.tokens

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson

/**
 * "Is this identity allowed to perform this action on this token?" —
 * port of `TokenActionPermission` / `TokenActionKind` /
 * `TokenActionEvaluator` / `TokenActionResolver` from
 * `TokenActionPermissionsView.swift`.
 */

/** Three states per action row (allowed / denied-with-reason / omitted). */
sealed interface TokenActionPermission {
    data object Allowed : TokenActionPermission
    data class Denied(val reason: String) : TokenActionPermission
    data object Hidden : TokenActionPermission

    val isAllowed: Boolean get() = this is Allowed
    val isHidden: Boolean get() = this is Hidden
    val deniedReason: String? get() = (this as? Denied)?.reason
}

/**
 * Every token action the permissions screen renders, in declaration
 * order (mirrors `TokenActionKind`). [id] doubles as the nav-arg
 * discriminator for the `TokenAction` route.
 */
enum class TokenActionKind(
    val id: String,
    val title: String,
    val subtitle: String,
) {
    TRANSFER("transfer", "Transfer", "Move tokens to another identity"),
    MINT("mint", "Mint", "Issue new tokens"),
    BURN("burn", "Burn", "Destroy tokens you hold"),
    FREEZE("freeze", "Freeze", "Freeze a holder's balance"),
    UNFREEZE("unfreeze", "Unfreeze", "Unfreeze a holder's balance"),
    DESTROY_FROZEN_FUNDS(
        "destroyFrozenFunds", "Destroy Frozen Funds", "Permanently destroy frozen funds",
    ),
    PAUSE("pause", "Pause", "Halt all token operations"),
    RESUME("resume", "Resume", "Resume operations after a pause"),
    CLAIM("claim", "Claim", "Claim distribution rewards"),
    DIRECT_PURCHASE("directPurchase", "Direct Purchase", "Buy tokens at the listed price"),
    SET_DIRECT_PURCHASE_PRICE(
        "setDirectPurchasePrice", "Set Direct Purchase Price",
        "Configure or disable the price for direct purchase",
    ),
    SET_CONVENTIONS(
        "setConventions", "Set / Change Conventions", "Update token name / localizations",
    ),
    UPDATE_MAX_SUPPLY("updateMaxSupply", "Update Max Supply", "Change the maximum supply"),
    CHANGE_DISTRIBUTION(
        "changeDistribution", "Change Distribution Rules",
        "Change perpetual / pre-programmed distribution",
    ),
    ;

    companion object {
        fun fromId(id: String): TokenActionKind? = entries.firstOrNull { it.id == id }
    }
}

/** One computed row, ready to render. */
data class ResolvedTokenAction(
    val kind: TokenActionKind,
    val permission: TokenActionPermission,
)

/**
 * Group definitions decoded from the contract's `groups` JSON
 * (`{"<position>": {"members": {"<base58>": power}, "requiredPower": n}}`)
 * — the shape `DataContractParser` stores and
 * `TokenActionEvaluator.resolveGroup` / `CoSignProposalView` read.
 */
class ContractGroups private constructor(private val groups: JsonObject) {

    fun group(position: Int): JsonObject? = groups[position.toString()] as? JsonObject

    fun members(position: Int): Map<String, Int> {
        val members = group(position)?.get("members") as? JsonObject ?: return emptyMap()
        return members.entries.mapNotNull { (id, power) ->
            (power as? JsonPrimitive)?.content?.toIntOrNull()?.let { id to it }
        }.toMap()
    }

    fun requiredPower(position: Int): Int =
        (group(position)?.get("requiredPower") as? JsonPrimitive)
            ?.content?.toIntOrNull()?.takeIf { it >= 0 } ?: 0

    fun isMember(position: Int, identityId: ByteArray): Boolean =
        members(position).containsKey(Base58.encode(identityId))

    companion object {
        /**
         * Decode from [DataContractEntity.groupsData], falling back to the
         * `groups` key of the serialized contract JSON.
         */
        fun from(contract: DataContractEntity?): ContractGroups? {
            contract ?: return null
            val obj = contract.groupsData?.let { parseObject(it.decodeToString()) }
                ?: parseObject(contract.serializedContract.decodeToString())
                    ?.get("groups") as? JsonObject
            return obj?.let { ContractGroups(it) }
        }

        private fun parseObject(json: String): JsonObject? = try {
            LenientJson.parseToJsonElement(json).jsonObject
        } catch (_: Exception) {
            null
        }
    }
}

/**
 * Resolves an `authorizedToMakeChange` rule string against a candidate
 * identity — port of `TokenActionEvaluator`.
 */
object TokenActionEvaluator {

    fun evaluate(
        rulesJson: String?,
        feature: String,
        identity: IdentityEntity,
        contract: DataContractEntity?,
        token: TokenEntity,
    ): TokenActionPermission {
        val rule = ChangeControlRules.parse(rulesJson)
            ?: return TokenActionPermission.Denied("$feature is not configured for this token")
        return resolveTaker(rule.authorizedToMakeChange, feature, identity, contract, token)
    }

    fun resolveTaker(
        authorized: String,
        feature: String,
        identity: IdentityEntity,
        contract: DataContractEntity?,
        token: TokenEntity,
    ): TokenActionPermission {
        if (authorized == AuthorizedActionTakers.NO_ONE) {
            return TokenActionPermission.Denied("$feature: no one is authorized")
        }

        if (authorized == AuthorizedActionTakers.CONTRACT_OWNER) {
            val ownerId = contract?.ownerId
                ?: return TokenActionPermission.Denied("$feature: contract owner unknown locally")
            return if (ownerId.contentEquals(identity.identityId)) {
                TokenActionPermission.Allowed
            } else {
                TokenActionPermission.Denied("$feature: only the contract owner")
            }
        }

        if (authorized == AuthorizedActionTakers.MAIN_GROUP) {
            val position = token.mainControlGroupPosition
                ?: return TokenActionPermission.Denied("$feature: main control group not configured")
            return resolveGroup(position, feature, identity, contract)
        }

        if (authorized.startsWith(AuthorizedActionTakers.IDENTITY_PREFIX)) {
            val idBase58 = authorized.removePrefix(AuthorizedActionTakers.IDENTITY_PREFIX)
            val target = Base58.decodeIdentifier(idBase58)
                ?: return TokenActionPermission.Denied("$feature: authorized identity unparseable")
            return if (target.contentEquals(identity.identityId)) {
                TokenActionPermission.Allowed
            } else {
                TokenActionPermission.Denied("$feature: only identity ${short(idBase58)}")
            }
        }

        if (authorized.startsWith(AuthorizedActionTakers.GROUP_PREFIX)) {
            val position = authorized
                .removePrefix(AuthorizedActionTakers.GROUP_PREFIX)
                .toIntOrNull()
            if (position != null) {
                return resolveGroup(position, feature, identity, contract)
            }
        }

        // Nested groups / admin-action-takers overrides — surface the gap
        // rather than silently green-lighting (the iOS resolver leaves this
        // case open the same way).
        return TokenActionPermission.Denied(
            "$feature: authorization rule '$authorized' not yet evaluated",
        )
    }

    private fun resolveGroup(
        position: Int,
        feature: String,
        identity: IdentityEntity,
        contract: DataContractEntity?,
    ): TokenActionPermission {
        contract ?: return TokenActionPermission.Denied("$feature: contract not loaded")
        val groups = ContractGroups.from(contract)
            ?: return TokenActionPermission.Denied("$feature: group data unavailable")
        groups.group(position)
            ?: return TokenActionPermission.Denied("$feature: group $position not found")
        val members = groups.members(position)
        if (members.isEmpty()) {
            return TokenActionPermission.Denied("$feature: group $position has no members")
        }
        return if (members.containsKey(Base58.encode(identity.identityId))) {
            TokenActionPermission.Allowed
        } else {
            TokenActionPermission.Denied("$feature: not in group $position")
        }
    }

    internal fun short(s: String): String =
        if (s.length <= 12) s else "${s.take(6)}..${s.takeLast(4)}"
}

/**
 * Pulls together the full action list for a `(token, identity)` pair, in
 * display order — port of `TokenActionResolver.resolve`.
 */
object TokenActionResolver {

    fun resolve(
        token: TokenEntity,
        identity: IdentityEntity,
        contract: DataContractEntity?,
    ): List<ResolvedTokenAction> {
        val rows = ArrayList<ResolvedTokenAction>(TokenActionKind.entries.size)

        // Transfer — always visible; only the paused gate is modelled.
        rows += ResolvedTokenAction(
            TokenActionKind.TRANSFER,
            if (token.isPaused) {
                TokenActionPermission.Denied("Token is paused")
            } else {
                TokenActionPermission.Allowed
            },
        )

        // Mint — plus the "no destination configured" guard.
        rows += ResolvedTokenAction(
            TokenActionKind.MINT,
            if (token.canManuallyMint) {
                val perm = TokenActionEvaluator.evaluate(
                    token.manualMintingRules, "Manual minting", identity, contract, token,
                )
                val pausedGuarded = pausedGuard(perm, token)
                if (pausedGuarded.isAllowed &&
                    !token.mintingAllowChoosingDestination &&
                    token.newTokensDestinationIdentity == null
                ) {
                    TokenActionPermission.Denied(
                        "Mint: no destination configured (contract forbids choosing one " +
                            "and didn't pin a recipient)",
                    )
                } else {
                    pausedGuarded
                }
            } else {
                TokenActionPermission.Denied("Manual minting not enabled for this token")
            },
        )

        // Burn.
        rows += ResolvedTokenAction(
            TokenActionKind.BURN,
            if (token.canManuallyBurn) {
                pausedGuard(
                    TokenActionEvaluator.evaluate(
                        token.manualBurningRules, "Manual burning", identity, contract, token,
                    ),
                    token,
                )
            } else {
                TokenActionPermission.Denied("Manual burning not enabled for this token")
            },
        )

        // Freeze / Unfreeze / Destroy frozen funds.
        rows += ResolvedTokenAction(
            TokenActionKind.FREEZE,
            if (token.canFreeze) {
                TokenActionEvaluator.evaluate(
                    token.freezeRules, "Freezing", identity, contract, token,
                )
            } else {
                TokenActionPermission.Denied("Freezing not enabled for this token")
            },
        )
        rows += ResolvedTokenAction(
            TokenActionKind.UNFREEZE,
            if (token.canUnfreeze) {
                TokenActionEvaluator.evaluate(
                    token.unfreezeRules, "Unfreezing", identity, contract, token,
                )
            } else {
                TokenActionPermission.Denied("Unfreezing not enabled for this token")
            },
        )
        rows += ResolvedTokenAction(
            TokenActionKind.DESTROY_FROZEN_FUNDS,
            if (token.canDestroyFrozenFunds) {
                TokenActionEvaluator.evaluate(
                    token.destroyFrozenFundsRules, "Destroy frozen funds",
                    identity, contract, token,
                )
            } else {
                TokenActionPermission.Denied("Destroying frozen funds not enabled")
            },
        )

        // Emergency action → Pause + Resume.
        if (token.hasEmergencyActions) {
            val base = TokenActionEvaluator.evaluate(
                token.emergencyActionRules, "Emergency action", identity, contract, token,
            )
            rows += ResolvedTokenAction(
                TokenActionKind.PAUSE,
                if (base.isAllowed && token.isPaused) {
                    TokenActionPermission.Denied("Token is already paused")
                } else {
                    base
                },
            )
            rows += ResolvedTokenAction(
                TokenActionKind.RESUME,
                if (base.isAllowed && !token.isPaused) {
                    TokenActionPermission.Denied("Token is not paused")
                } else {
                    base
                },
            )
        } else {
            rows += ResolvedTokenAction(
                TokenActionKind.PAUSE,
                TokenActionPermission.Denied("Emergency actions not enabled"),
            )
            rows += ResolvedTokenAction(
                TokenActionKind.RESUME,
                TokenActionPermission.Denied("Emergency actions not enabled"),
            )
        }

        // Claim.
        rows += ResolvedTokenAction(TokenActionKind.CLAIM, resolveClaim(token, identity))

        // Direct purchase — allowed whenever pricing rules exist and the
        // token isn't paused. `PurchaseForm` fetches the configured price on
        // open (`directPurchasePrices`) and computes the total cost
        // client-side — including the "no price configured" case — so the
        // price no longer needs to be pre-resolved here to gate the row.
        val distributionChange = TokenDistributionChangeRules.parse(token.distributionChangeRules)
        val pricingRule = distributionChange?.changeDirectPurchasePricingRules
        rows += ResolvedTokenAction(
            TokenActionKind.DIRECT_PURCHASE,
            when {
                pricingRule == null -> TokenActionPermission.Hidden
                token.isPaused -> TokenActionPermission.Denied("Token is paused")
                else -> TokenActionPermission.Allowed
            },
        )

        // Set direct-purchase price — hidden without a pricing rule.
        rows += ResolvedTokenAction(
            TokenActionKind.SET_DIRECT_PURCHASE_PRICE,
            if (pricingRule == null) {
                TokenActionPermission.Hidden
            } else {
                TokenActionEvaluator.resolveTaker(
                    pricingRule.authorizedToMakeChange, "Direct purchase pricing",
                    identity, contract, token,
                )
            },
        )

        // Conventions / max supply.
        rows += ResolvedTokenAction(
            TokenActionKind.SET_CONVENTIONS,
            TokenActionEvaluator.evaluate(
                token.conventionsChangeRules, "Conventions change", identity, contract, token,
            ),
        )
        rows += ResolvedTokenAction(
            TokenActionKind.UPDATE_MAX_SUPPLY,
            TokenActionEvaluator.evaluate(
                token.maxSupplyChangeRules, "Max supply change", identity, contract, token,
            ),
        )

        // Change distribution — headline rule fallback chain.
        val distributionRule = distributionChange?.perpetualDistributionRules
            ?: distributionChange?.newTokensDestinationIdentityRules
            ?: distributionChange?.mintingAllowChoosingDestinationRules
        rows += ResolvedTokenAction(
            TokenActionKind.CHANGE_DISTRIBUTION,
            if (distributionRule == null) {
                TokenActionPermission.Denied("Distribution change is not configured for this token")
            } else {
                TokenActionEvaluator.resolveTaker(
                    distributionRule.authorizedToMakeChange, "Distribution change",
                    identity, contract, token,
                )
            },
        )

        return rows
    }

    private fun pausedGuard(
        permission: TokenActionPermission,
        token: TokenEntity,
    ): TokenActionPermission =
        if (permission.isAllowed && token.isPaused) {
            TokenActionPermission.Denied("Token is paused")
        } else {
            permission
        }

    private fun resolveClaim(
        token: TokenEntity,
        identity: IdentityEntity,
    ): TokenActionPermission {
        val hasPerpetual = token.perpetualDistribution != null
        val hasPreProgrammed = token.preProgrammedDistribution != null
        if (!hasPerpetual && !hasPreProgrammed) {
            return TokenActionPermission.Denied("Token has no distribution schedule")
        }

        if (token.newTokensDestinationIdentity?.contentEquals(identity.identityId) == true) {
            return TokenActionPermission.Allowed
        }
        // Pre-programmed releases name their recipients in the contract;
        // maturity and already-claimed are enforced on-chain at submit time.
        if (hasPreProgrammed && isPreProgrammedRecipient(token, identity)) {
            return TokenActionPermission.Allowed
        }

        if (!hasPerpetual) {
            return TokenActionPermission.Denied("Not a recipient of any pre-programmed release")
        }
        if (!token.mintingAllowChoosingDestination) {
            return TokenActionPermission.Denied("Not the designated distribution recipient")
        }
        return TokenActionPermission.Denied("Distribution eligibility not yet evaluated")
    }

    /**
     * True when [identity] is named as a recipient in ANY release of the
     * token's pre-programmed distribution JSON
     * (`{"distributions":{"<timestampMs>":{"<recipientBase58>":amount,…},…}}`,
     * optionally V0-wrapped).
     */
    private fun isPreProgrammedRecipient(
        token: TokenEntity,
        identity: IdentityEntity,
    ): Boolean {
        val json = token.preProgrammedDistribution ?: return false
        val recipient = Base58.encode(identity.identityId)
        return try {
            val root = LenientJson.parseToJsonElement(json).jsonObject
            val body = (root["V0"] as? JsonObject) ?: root
            val distributions = body["distributions"] as? JsonObject ?: return false
            distributions.values.any { release ->
                (release as? JsonObject)?.containsKey(recipient) == true
            }
        } catch (_: Exception) {
            false
        }
    }
}
