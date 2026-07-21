package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.TokenActionPermissions
import org.dashfoundation.example.services.tokens.ChangeControlRules
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.services.tokens.TokenDistributionChangeRules
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.formatDate
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle

/**
 * One token's full configuration — port of `TokenDetailsView.swift`:
 * actions entry point (→ [TokenActionPermissions]), basic info,
 * localizations, supply, feature flags (via the `hasAuthorizedTakers`
 * truth-in-UI predicate), history-keeping flags, control rules,
 * distribution, and trade mode — plus the identity balances and history
 * events the Room token family carries.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TokenDetailsScreen(tokenIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val tokenId = remember(tokenIdHex) { tokenIdHex.hexToBytes() }

    val tokenFlow = remember(tokenIdHex) {
        container.database.tokenDao().observeTokenById(tokenId)
    }
    val token by tokenFlow.collectAsStateWithLifecycle(initialValue = null)

    val historyFlow = remember(tokenIdHex) {
        container.database.tokenDao().observeHistoryByToken(tokenId)
    }
    val history by historyFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(token?.name ?: "Token") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val current = token ?: return@Scaffold

        // Balance rows are keyed by the canonical on-chain token id
        // (`TokenBalanceEntity.tokenId`), not the synthetic row id — match
        // through the relationship key instead, like the Swift views do.
        val allBalancesFlow = remember(tokenIdHex) {
            container.database.tokenDao().observeNonZeroBalances()
        }
        val allBalances by allBalancesFlow.collectAsStateWithLifecycle(initialValue = emptyList())
        val balances = allBalances.filter { it.tokenRef?.contentEquals(current.id) == true }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // Actions entry (← actionsEntrySection).
            FormSection {
                ListItem(
                    headlineContent = { Text("View Actions") },
                    supportingContent = { Text("See what you can do with this token") },
                    modifier = Modifier
                        .clickable {
                            navController.navigate(TokenActionPermissions(tokenIdHex))
                        }
                        .testTag("tokenDetail.viewActions"),
                )
            }

            FormSection(title = "Basic Information") {
                LabeledContent("Name", current.name)
                LabeledContent("Description", current.tokenDescription ?: "No description")
                LabeledContent("Contract", truncateMiddle(Base58.encode(current.contractId), 10, 6))
                LabeledContent("Position", "${current.position}")
                LabeledContent("Decimals", "${current.decimals}")
            }

            current.localizations?.let { LocalizationsSection(it) }

            val maxSupplyRules = ChangeControlRules.parse(current.maxSupplyChangeRules)
            FormSection(title = "Supply Information") {
                LabeledContent(
                    "Base Supply",
                    TokenAmounts.format(current.baseSupply, current.decimals),
                )
                LabeledContent(
                    "Max Supply",
                    current.maxSupply
                        ?.let { TokenAmounts.format(it, current.decimals) }
                        ?: "Unlimited",
                )
                LabeledContent(
                    "Max Supply Changeable",
                    if (maxSupplyRules?.hasAuthorizedTakers == true) "Yes" else "No",
                )
            }

            FormSection(title = "Token Features") {
                FeatureRow("Can be minted", current.rulesAllow { it.manualMintingRules })
                FeatureRow("Can be burned", current.rulesAllow { it.manualBurningRules })
                FeatureRow("Can be frozen", current.rulesAllow { it.freezeRules })
                FeatureRow("Can be unfrozen", current.rulesAllow { it.unfreezeRules })
                FeatureRow(
                    "Can destroy frozen funds",
                    current.rulesAllow { it.destroyFrozenFundsRules },
                )
                FeatureRow("Transfer to frozen allowed", current.allowTransferToFrozenBalance)
                FeatureRow(
                    "Emergency action available",
                    current.rulesAllow { it.emergencyActionRules },
                )
                FeatureRow("Currently paused", current.isPaused)
            }

            FormSection(title = "History Keeping") {
                FeatureRow("Transfer history", current.keepsTransferHistory)
                FeatureRow("Freezing history", current.keepsFreezingHistory)
                FeatureRow("Minting history", current.keepsMintingHistory)
                FeatureRow("Burning history", current.keepsBurningHistory)
                FeatureRow("Direct pricing history", current.keepsDirectPricingHistory)
                FeatureRow("Direct purchase history", current.keepsDirectPurchaseHistory)
            }

            FormSection(title = "Control Rules") {
                ControlRuleRows("Conventions", current.conventionsChangeRules)
                ControlRuleRows("Max Supply", current.maxSupplyChangeRules)
                ControlRuleRows("Manual Minting", current.manualMintingRules)
                ControlRuleRows("Manual Burning", current.manualBurningRules)
                ControlRuleRows("Freeze", current.freezeRules)
                ControlRuleRows("Unfreeze", current.unfreezeRules)
                ControlRuleRows("Destroy Frozen Funds", current.destroyFrozenFundsRules)
                ControlRuleRows("Emergency Action", current.emergencyActionRules)
                TokenDistributionChangeRules.parse(current.distributionChangeRules)?.let { bundle ->
                    bundle.changeDirectPurchasePricingRules?.let {
                        ControlRuleRows("Direct Purchase Pricing", it.toJson())
                    }
                    bundle.perpetualDistributionRules?.let {
                        ControlRuleRows("Perpetual Distribution", it.toJson())
                    }
                }
            }

            if (current.perpetualDistribution != null ||
                current.preProgrammedDistribution != null ||
                current.newTokensDestinationIdentity != null
            ) {
                FormSection(title = "Distribution") {
                    LabeledContent(
                        "Perpetual",
                        if (current.perpetualDistribution != null) "Configured" else "None",
                    )
                    LabeledContent(
                        "Pre-programmed",
                        if (current.preProgrammedDistribution != null) "Configured" else "None",
                    )
                    current.newTokensDestinationIdentity?.let {
                        LabeledContent(
                            "Destination Identity",
                            truncateMiddle(Base58.encode(it), 10, 6),
                        )
                    }
                    LabeledContent(
                        "Allow choosing destination",
                        if (current.mintingAllowChoosingDestination) "Yes" else "No",
                    )
                }
            }

            FormSection(title = "Trade Mode") {
                LabeledContent(
                    "Trade Mode",
                    if (current.tradeMode == "NotTradeable") "Not Tradeable" else current.tradeMode,
                )
                current.tradeModeChangeRules?.let { ControlRuleRows("Trade Mode Change", it) }
            }

            // ── Room-backed balance + history rows ─────────────────────
            FormSection(title = "Balances (${balances.size})") {
                if (balances.isEmpty()) {
                    Text(
                        "No non-zero balances stored locally.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    balances.forEach { balance ->
                        LabeledContent(
                            truncateMiddle(Base58.encode(balance.identityId), 8, 6),
                            TokenAmounts.format(
                                balance.balance.value,
                                balance.tokenDecimals ?: current.decimals,
                            ) + if (balance.frozen) " (frozen)" else "",
                        )
                    }
                }
            }

            FormSection(title = "History (${history.size})") {
                if (history.isEmpty()) {
                    Text(
                        "No history events stored locally.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    history.forEach { event ->
                        Column(modifier = Modifier.padding(vertical = 6.dp)) {
                            Row(modifier = Modifier.fillMaxWidth()) {
                                Text(
                                    event.eventType,
                                    style = MaterialTheme.typography.bodyMedium,
                                    modifier = Modifier.weight(1f),
                                )
                                Text(
                                    formatDate(event.eventTimestamp),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            val detail = buildList {
                                event.amount?.let {
                                    add("amount ${TokenAmounts.format(it, current.decimals)}")
                                }
                                add("by ${truncateMiddle(Base58.encode(event.performedByIdentity), 6, 4)}")
                                event.eventDescription?.let { add(it) }
                            }.joinToString(" · ")
                            Text(
                                detail,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        HorizontalDivider()
                    }
                }
            }
        }
    }
}

@Composable
private fun FeatureRow(label: String, enabled: Boolean) {
    LabeledContent(label, if (enabled) "Yes" else "No")
}

/** Authorized / admin taker rows for one rule (← `ControlRuleView`). */
@Composable
private fun ControlRuleRows(title: String, rulesJson: String?) {
    val rule = ChangeControlRules.parse(rulesJson) ?: return
    Column(modifier = Modifier.padding(vertical = 4.dp)) {
        Text(title, style = MaterialTheme.typography.bodyMedium)
        Text(
            "Authorized: ${rule.authorizedToMakeChange}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Admin: ${rule.adminActionTakers}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** True when the rule column exists AND names an authorized taker. */
private inline fun org.dashfoundation.dashsdk.persistence.entities.TokenEntity.rulesAllow(
    selector: (org.dashfoundation.dashsdk.persistence.entities.TokenEntity) -> String?,
): Boolean = ChangeControlRules.parse(selector(this))?.hasAuthorizedTakers == true

/** Localization rows from the stored `{lang: TokenLocalization}` JSON map. */
@Composable
private fun LocalizationsSection(localizationsJson: String) {
    val entries = remember(localizationsJson) {
        try {
            LenientJson.parseToJsonElement(localizationsJson).jsonObject
                .entries
                .mapNotNull { (lang, value) ->
                    val obj = value as? JsonObject ?: return@mapNotNull null
                    val singular = (obj["singularForm"] as? JsonPrimitive)?.content
                        ?: (obj["singular"] as? JsonPrimitive)?.content ?: ""
                    val plural = (obj["pluralForm"] as? JsonPrimitive)?.content
                        ?: (obj["plural"] as? JsonPrimitive)?.content ?: ""
                    Triple(lang, singular, plural)
                }
                .sortedBy { it.first }
        } catch (_: Exception) {
            emptyList()
        }
    }
    if (entries.isEmpty()) return
    FormSection(title = "Localizations") {
        entries.forEach { (lang, singular, plural) ->
            LabeledContent(lang.uppercase(), "Singular: $singular · Plural: $plural")
        }
    }
}
