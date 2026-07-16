package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.tokens.GroupAction
import org.dashfoundation.dashsdk.tokens.TokenConfigChange
import org.dashfoundation.dashsdk.tokens.TokenDistributionType
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.TokenAction
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator.BannerState
import org.dashfoundation.example.services.tokens.ProvenBalances
import org.dashfoundation.example.services.tokens.TokenActionKind
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.services.tokens.TokenDirectPurchasePricing
import org.dashfoundation.example.services.tokens.TokenDistributionChangeRules
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.RecipientPicker
import org.dashfoundation.example.ui.components.RecipientSelection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import java.math.BigInteger

/**
 * The 12 token-action forms — thin composables over [TokenActionScaffold],
 * each a port of the matching `TokenActions/Token<Action>ActionView.swift`.
 * Every form declares its sections + builds the `Tokens.<action>` call;
 * the shared submit/progress/error/banner machinery lives in the scaffold.
 */
@Composable
fun TokenActionScreen(route: TokenAction, navController: NavHostController) {
    val context = rememberTokenActionContext(route.tokenIdHex, route.identityIdHex)
    if (context == null) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        return
    }
    val viewModel: TokenActionViewModel = viewModel()

    when (TokenActionKind.fromId(route.actionId)) {
        TokenActionKind.TRANSFER -> TransferForm(context, viewModel, navController)
        TokenActionKind.MINT -> MintForm(context, viewModel, navController)
        TokenActionKind.BURN -> BurnForm(context, viewModel, navController)
        TokenActionKind.FREEZE -> FreezeFamilyForm(
            context, viewModel, navController, FreezeVariant.FREEZE,
        )
        TokenActionKind.UNFREEZE -> FreezeFamilyForm(
            context, viewModel, navController, FreezeVariant.UNFREEZE,
        )
        TokenActionKind.DESTROY_FROZEN_FUNDS -> FreezeFamilyForm(
            context, viewModel, navController, FreezeVariant.DESTROY_FROZEN_FUNDS,
        )
        TokenActionKind.PAUSE -> PauseResumeForm(context, viewModel, navController, isPause = true)
        TokenActionKind.RESUME -> PauseResumeForm(context, viewModel, navController, isPause = false)
        TokenActionKind.CLAIM -> ClaimForm(context, viewModel, navController)
        TokenActionKind.DIRECT_PURCHASE -> PurchaseForm(context, viewModel, navController)
        TokenActionKind.SET_DIRECT_PURCHASE_PRICE -> SetPriceForm(context, viewModel, navController)
        TokenActionKind.UPDATE_MAX_SUPPLY -> UpdateMaxSupplyForm(context, viewModel, navController)
        // Conventions / distribution editors are deferred on iOS too
        // (TokenActionPermissionsView.swift `deferredToFutureRelease`).
        TokenActionKind.SET_CONVENTIONS, TokenActionKind.CHANGE_DISTRIBUTION, null ->
            DeferredActionScreen(route.actionId, navController)
    }
}

// ── Shared bits ────────────────────────────────────────────────────────

private val RecipientSaver = listSaver<RecipientSelection?, String>(
    save = { it?.let { s -> listOf(s.identityIdHex, s.label) } ?: emptyList() },
    restore = { if (it.size == 2) RecipientSelection(it[0], it[1]) else null },
)

/** The identity's held balance of this token (matched on the token relationship key). */
@Composable
private fun rememberHeldBalance(context: TokenActionContext): ULong {
    val container = LocalAppContainer.current
    val flow = remember(context.token.id, context.identity.identityId) {
        container.database.tokenDao().observeBalancesByIdentity(context.identity.identityId)
    }
    val balances by flow.collectAsStateWithLifecycle(initialValue = emptyList())
    val row = balances.firstOrNull { it.tokenRef?.contentEquals(context.token.id) == true }
    return row?.balance?.value ?: 0uL
}

@Composable
private fun AmountSection(
    value: String,
    onValueChange: (String) -> Unit,
    errorText: String?,
    label: String = "Amount",
) {
    FormSection(title = label) {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            label = { Text(label) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            isError = errorText != null,
            supportingText = { if (errorText != null) Text(errorText) },
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp)
                .testTag("tokenAction.amountField"),
        )
    }
}

private fun bigOrNull(raw: String?): BigInteger? = raw?.let {
    try {
        BigInteger(it)
    } catch (_: NumberFormatException) {
        null
    }
}

// ── Transfer (← TokenTransferActionView.swift) ─────────────────────────

@Composable
private fun TransferForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    var recipient by rememberSaveable(stateSaver = RecipientSaver) { mutableStateOf(null) }
    var amountText by rememberSaveable { mutableStateOf("") }
    var note by rememberSaveable { mutableStateOf("") }
    val container = LocalAppContainer.current
    val sdk by LocalAppState.current.sdk.collectAsStateWithLifecycle()

    val balance = rememberHeldBalance(context)
    val amountRaw = TokenAmounts.parse(amountText, context.token.decimals)
    val exceedsBalance = amountRaw != null && amountRaw > balance
    val canSubmit = recipient != null && amountRaw != null && amountRaw > 0u && !exceedsBalance
    val networkRaw = context.identity.networkRaw

    TokenActionScaffold(
        title = "Transfer",
        context = context,
        banner = BannerState.None, // transfer is never group-gated
        bannerActionLabel = "transfer",
        submitLabel = "Transfer",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        tokenSummaryRows = {
            LabeledContent("Balance", TokenAmounts.format(balance, context.token.decimals))
        },
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val recipientId = recipient?.identityIdHex?.hexToBytes() ?: return@TokenActionScaffold
            val amount = amountRaw ?: return@TokenActionScaffold
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                val balances = wallet.tokens.transfer(
                    identityId = context.identity.identityId,
                    tokenContractId = context.token.contractId,
                    tokenPosition = context.token.position,
                    recipientId = recipientId,
                    amount = amount,
                    publicNote = noteOrNull,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
                ProvenBalances.persist(
                    balances, context.token, container.database.tokenDao(), sdk, networkRaw,
                )
            }
        },
    ) {
        FormSection(title = "Recipient") {
            RecipientPicker(
                selection = recipient,
                onSelectionChange = { recipient = it },
                networkRaw = networkRaw,
                excludeIdentityIdHex = context.identity.identityId.let {
                    it.joinToString("") { b -> "%02x".format(b) }
                },
            )
        }
        AmountSection(
            value = amountText,
            onValueChange = { amountText = it },
            errorText = if (exceedsBalance) "Amount exceeds your balance." else null,
        )
        PublicNoteField(note) { note = it }
    }
}

// ── Mint (← TokenMintActionView.swift) ─────────────────────────────────

@Composable
private fun MintForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    var mintToSelf by rememberSaveable { mutableStateOf(true) }
    var recipient by rememberSaveable(stateSaver = RecipientSaver) { mutableStateOf(null) }
    var amountText by rememberSaveable { mutableStateOf("") }
    var note by rememberSaveable { mutableStateOf("") }
    val container = LocalAppContainer.current
    val sdk by LocalAppState.current.sdk.collectAsStateWithLifecycle()

    val banner = GroupActionRuleEvaluator.bannerState(
        token.manualMintingRules, token.mainControlGroupPosition,
    )
    val amountRaw = TokenAmounts.parse(amountText, token.decimals)
    val maxSupply = bigOrNull(token.maxSupply)
    val exceedsMaxSupply = amountRaw != null && maxSupply != null &&
        BigInteger(amountRaw.toString()) > maxSupply
    val canSubmit = amountRaw != null && amountRaw > 0u && !exceedsMaxSupply &&
        (mintToSelf || recipient != null)

    TokenActionScaffold(
        title = "Mint",
        context = context,
        banner = banner,
        bannerActionLabel = "mint",
        submitLabel = "Mint",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        tokenSummaryRows = {
            token.maxSupply?.let {
                LabeledContent("Max supply", TokenAmounts.format(it, token.decimals))
            }
        },
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val amount = amountRaw ?: return@TokenActionScaffold
            // Destination resolution — TokenMintActionView.swift:221-243:
            // self + choosable → explicit own id; self + not choosable →
            // null (defer to the configured destination); else the picked
            // recipient.
            val issuedTo: ByteArray? = if (mintToSelf) {
                if (token.mintingAllowChoosingDestination) context.identity.identityId else null
            } else {
                recipient?.identityIdHex?.hexToBytes() ?: return@TokenActionScaffold
            }
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                val balances = wallet.tokens.mint(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    amount = amount,
                    issuedToIdentityId = issuedTo,
                    publicNote = noteOrNull,
                    groupAction = groupAction,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
                ProvenBalances.persist(
                    balances, token, container.database.tokenDao(),
                    sdk, context.identity.networkRaw,
                )
            }
        },
    ) {
        if (!token.mintingAllowChoosingDestination) {
            FormSection {
                Text(
                    "This token does not allow choosing a mint destination — tokens are " +
                        "issued to the configured recipient.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        FormSection(title = "Recipient") {
            androidx.compose.foundation.layout.Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    if (token.mintingAllowChoosingDestination) {
                        "Mint to self"
                    } else {
                        "Use configured destination"
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f),
                )
                Switch(
                    checked = mintToSelf,
                    onCheckedChange = { mintToSelf = it },
                    enabled = token.mintingAllowChoosingDestination,
                    modifier = Modifier.testTag("tokenAction.mintToSelfToggle"),
                )
            }
            if (!mintToSelf) {
                RecipientPicker(
                    selection = recipient,
                    onSelectionChange = { recipient = it },
                    networkRaw = context.identity.networkRaw,
                )
            }
        }
        AmountSection(
            value = amountText,
            onValueChange = { amountText = it },
            errorText = if (exceedsMaxSupply) {
                "Amount exceeds the configured max supply."
            } else {
                null
            },
        )
        PublicNoteField(note) { note = it }
    }
}

// ── Burn (← TokenBurnActionView.swift) ─────────────────────────────────

@Composable
private fun BurnForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    var amountText by rememberSaveable { mutableStateOf("") }
    var note by rememberSaveable { mutableStateOf("") }
    val container = LocalAppContainer.current
    val sdk by LocalAppState.current.sdk.collectAsStateWithLifecycle()

    val banner = GroupActionRuleEvaluator.bannerState(
        token.manualBurningRules, token.mainControlGroupPosition,
    )
    val balance = rememberHeldBalance(context)
    val amountRaw = TokenAmounts.parse(amountText, token.decimals)
    val exceedsBalance = amountRaw != null && amountRaw > balance
    val canSubmit = amountRaw != null && amountRaw > 0u && !exceedsBalance

    TokenActionScaffold(
        title = "Burn",
        context = context,
        banner = banner,
        bannerActionLabel = "burn",
        submitLabel = "Burn",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        tokenSummaryRows = {
            LabeledContent("Balance", TokenAmounts.format(balance, token.decimals))
        },
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val amount = amountRaw ?: return@TokenActionScaffold
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                val balances = wallet.tokens.burn(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    amount = amount,
                    publicNote = noteOrNull,
                    groupAction = groupAction,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
                ProvenBalances.persist(
                    balances, token, container.database.tokenDao(),
                    sdk, context.identity.networkRaw,
                )
            }
        },
    ) {
        AmountSection(
            value = amountText,
            onValueChange = { amountText = it },
            errorText = if (exceedsBalance) "Amount exceeds your balance." else null,
        )
        PublicNoteField(note) { note = it }
    }
}

// ── Freeze / Unfreeze / Destroy Frozen Funds ───────────────────────────
// (← TokenFreezeActionView / TokenUnfreezeActionView /
//    TokenDestroyFrozenFundsActionView — identical shape: a target
//    identity, a note, and a group-gatable rule.)

private enum class FreezeVariant(
    val title: String,
    val actionLabel: String,
    val targetLabel: String,
) {
    FREEZE("Freeze", "freeze", "Identity to freeze"),
    UNFREEZE("Unfreeze", "unfreeze", "Identity to unfreeze"),
    DESTROY_FROZEN_FUNDS("Destroy Frozen Funds", "destroy", "Frozen identity"),
}

@Composable
private fun FreezeFamilyForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
    variant: FreezeVariant,
) {
    val token = context.token
    var target by rememberSaveable(stateSaver = RecipientSaver) { mutableStateOf(null) }
    var note by rememberSaveable { mutableStateOf("") }

    val rulesJson = when (variant) {
        FreezeVariant.FREEZE -> token.freezeRules
        FreezeVariant.UNFREEZE -> token.unfreezeRules
        FreezeVariant.DESTROY_FROZEN_FUNDS -> token.destroyFrozenFundsRules
    }
    val banner = GroupActionRuleEvaluator.bannerState(rulesJson, token.mainControlGroupPosition)

    TokenActionScaffold(
        title = variant.title,
        context = context,
        banner = banner,
        bannerActionLabel = variant.actionLabel,
        submitLabel = variant.title,
        canSubmit = target != null,
        viewModel = viewModel,
        navController = navController,
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val frozenId = target?.identityIdHex?.hexToBytes() ?: return@TokenActionScaffold
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                when (variant) {
                    FreezeVariant.FREEZE -> wallet.tokens.freeze(
                        identityId = context.identity.identityId,
                        tokenContractId = token.contractId,
                        tokenPosition = token.position,
                        frozenIdentityId = frozenId,
                        publicNote = noteOrNull,
                        groupAction = groupAction,
                        signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                        signerHandle = signer,
                    )
                    FreezeVariant.UNFREEZE -> wallet.tokens.unfreeze(
                        identityId = context.identity.identityId,
                        tokenContractId = token.contractId,
                        tokenPosition = token.position,
                        frozenIdentityId = frozenId,
                        publicNote = noteOrNull,
                        groupAction = groupAction,
                        signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                        signerHandle = signer,
                    )
                    FreezeVariant.DESTROY_FROZEN_FUNDS -> wallet.tokens.destroyFrozenFunds(
                        identityId = context.identity.identityId,
                        tokenContractId = token.contractId,
                        tokenPosition = token.position,
                        frozenIdentityId = frozenId,
                        publicNote = noteOrNull,
                        groupAction = groupAction,
                        signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                        signerHandle = signer,
                    )
                }
            }
        },
    ) {
        FormSection(title = variant.targetLabel) {
            RecipientPicker(
                selection = target,
                onSelectionChange = { target = it },
                networkRaw = context.identity.networkRaw,
            )
        }
        PublicNoteField(note) { note = it }
    }
}

// ── Pause / Resume (← TokenPauseActionView / TokenResumeActionView) ────

@Composable
private fun PauseResumeForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
    isPause: Boolean,
) {
    val token = context.token
    var note by rememberSaveable { mutableStateOf("") }
    val banner = GroupActionRuleEvaluator.bannerState(
        token.emergencyActionRules, token.mainControlGroupPosition,
    )
    val title = if (isPause) "Pause" else "Resume"

    TokenActionScaffold(
        title = title,
        context = context,
        banner = banner,
        bannerActionLabel = if (isPause) "pause" else "resume",
        submitLabel = title,
        canSubmit = true,
        viewModel = viewModel,
        navController = navController,
        tokenSummaryRows = {
            LabeledContent("Currently paused", if (token.isPaused) "Yes" else "No")
        },
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                if (isPause) {
                    wallet.tokens.pause(
                        identityId = context.identity.identityId,
                        tokenContractId = token.contractId,
                        tokenPosition = token.position,
                        publicNote = noteOrNull,
                        groupAction = groupAction,
                        signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                        signerHandle = signer,
                    )
                } else {
                    wallet.tokens.resume(
                        identityId = context.identity.identityId,
                        tokenContractId = token.contractId,
                        tokenPosition = token.position,
                        publicNote = noteOrNull,
                        groupAction = groupAction,
                        signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                        signerHandle = signer,
                    )
                }
            }
        },
    ) {
        FormSection {
            Text(
                if (isPause) {
                    "Pausing halts all token operations until a resume is broadcast."
                } else {
                    "Resuming re-enables token operations after a pause."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        PublicNoteField(note) { note = it }
    }
}

// ── Claim (← TokenClaimActionView.swift) ───────────────────────────────

@Composable
private fun ClaimForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    // Perpetual wins when both exist (matches Drive's claim ordering).
    val available = buildList {
        if (token.perpetualDistribution != null) add(TokenDistributionType.PERPETUAL)
        if (token.preProgrammedDistribution != null) add(TokenDistributionType.PRE_PROGRAMMED)
    }
    var selectedOrdinal by rememberSaveable {
        mutableStateOf((available.firstOrNull() ?: TokenDistributionType.PERPETUAL).ordinal)
    }
    val selected = TokenDistributionType.entries[selectedOrdinal]
    var note by rememberSaveable { mutableStateOf("") }
    val canSubmit = available.contains(selected)

    TokenActionScaffold(
        title = "Claim",
        context = context,
        banner = BannerState.None, // claim is never group-gated
        bannerActionLabel = "claim",
        submitLabel = "Claim",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                wallet.tokens.claim(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    distributionType = selected,
                    publicNote = noteOrNull,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
            }
        },
    ) {
        if (available.isEmpty()) {
            FormSection {
                Text(
                    "No distributions available to claim.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            FormSection(title = "Distribution type") {
                SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                    available.forEachIndexed { index, dist ->
                        SegmentedButton(
                            selected = selected == dist,
                            onClick = { selectedOrdinal = dist.ordinal },
                            enabled = available.size > 1,
                            shape = SegmentedButtonDefaults.itemShape(
                                index = index, count = available.size,
                            ),
                            modifier = Modifier.testTag("tokenAction.distribution.${dist.name}"),
                        ) {
                            Text(
                                when (dist) {
                                    TokenDistributionType.PERPETUAL -> "Perpetual"
                                    TokenDistributionType.PRE_PROGRAMMED -> "Pre-Programmed"
                                },
                            )
                        }
                    }
                }
            }
        }
        PublicNoteField(note) { note = it }
    }
}

// ── Direct Purchase (← TokenPurchaseActionView.swift) ──────────────────

/** Loading / loaded state of the token's direct-purchase price. */
private sealed interface PurchasePriceState {
    data object Loading : PurchasePriceState
    data class Loaded(val pricing: TokenDirectPurchasePricing?) : PurchasePriceState
}

@Composable
private fun PurchaseForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    var amountText by rememberSaveable { mutableStateOf("") }
    val sdk by LocalAppState.current.sdk.collectAsStateWithLifecycle()

    // Fetch the configured direct-purchase price when the form opens
    // (mirrors the live balance/status fetch in
    // TokenActionPermissionsView). The canonical token id — needed by the
    // price query — is derived the same way the permissions screen does
    // (`calculateTokenId`). Stays `Loading` until the SDK connects; on a
    // resolved query with no price set it becomes `Loaded(null)` and Buy
    // stays disabled with a clear reason.
    var priceState by remember { mutableStateOf<PurchasePriceState>(PurchasePriceState.Loading) }
    LaunchedEffect(sdk, token.id.toHex()) {
        priceState = PurchasePriceState.Loading
        val activeSdk = sdk ?: return@LaunchedEffect
        val canonicalId = runCatching {
            activeSdk.tokenQueries.calculateTokenId(Base58.encode(token.contractId), token.position)
        }.getOrNull()
        if (canonicalId == null) {
            priceState = PurchasePriceState.Loaded(null)
            return@LaunchedEffect
        }
        val pricing = runCatching {
            activeSdk.tokenQueries.directPurchasePrices(listOf(canonicalId))
        }.getOrNull()?.let { TokenDirectPurchasePricing.parse(it, canonicalId) }
        priceState = PurchasePriceState.Loaded(pricing)
    }

    val amountRaw = TokenAmounts.parse(amountText, token.decimals)
    val pricing = (priceState as? PurchasePriceState.Loaded)?.pricing
    // `costFor` applies the exact tier rule Drive uses to validate the
    // purchase, so this equals the chain's required price. Null means the
    // amount isn't purchasable (below the minimum or outside u64).
    val expectedTotalCost = amountRaw?.let { pricing?.costFor(it) }
    val canSubmit = amountRaw != null && amountRaw > 0u && expectedTotalCost != null

    TokenActionScaffold(
        title = "Direct Purchase",
        context = context,
        banner = BannerState.None, // purchase is never group-gated
        bannerActionLabel = "purchase",
        submitLabel = "Buy",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val amount = amountRaw ?: return@TokenActionScaffold
            val totalCost = expectedTotalCost ?: return@TokenActionScaffold
            viewModel.submit {
                wallet.tokens.purchase(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    amount = amount,
                    expectedTotalCost = totalCost,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
            }
        },
    ) {
        AmountSection(
            value = amountText,
            onValueChange = { amountText = it },
            errorText = null,
        )
        FormSection {
            when (val state = priceState) {
                PurchasePriceState.Loading -> Text(
                    "Loading the configured direct-purchase price…",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("tokenAction.purchase.status"),
                )
                is PurchasePriceState.Loaded -> {
                    val loadedPricing = state.pricing
                    when {
                        loadedPricing == null -> Text(
                            "This token has no direct-purchase price configured, so it can't " +
                                "be bought directly.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("tokenAction.purchase.status"),
                        )
                        amountRaw == null || amountRaw == 0uL -> Text(
                            "Enter an amount to see the total cost.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("tokenAction.purchase.status"),
                        )
                        expectedTotalCost == null -> Text(
                            purchaseUnavailableReason(loadedPricing, amountRaw, token.decimals),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("tokenAction.purchase.status"),
                        )
                        else -> LabeledContent(
                            "Total cost",
                            "$expectedTotalCost credits",
                        )
                    }
                }
            }
        }
    }
}

/**
 * Why an entered [amount] can't be purchased at [pricing] — an under-minimum
 * hint for a tiered schedule, otherwise a generic "not available at this
 * amount" (covers a total beyond the protocol u64 range).
 */
private fun purchaseUnavailableReason(
    pricing: TokenDirectPurchasePricing,
    amount: ULong,
    decimals: Int,
): String {
    val minimum = pricing.minimumPurchaseAmount
    return if (BigInteger(amount.toString()) < minimum) {
        "The minimum direct purchase for this token is " +
            "${TokenAmounts.format(minimum.toString(), decimals)}."
    } else {
        "This amount can't be purchased at the configured price."
    }
}

// ── Set Price (← TokenSetPriceActionView.swift) ────────────────────────

@Composable
private fun SetPriceForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    var priceText by rememberSaveable { mutableStateOf("") }
    var note by rememberSaveable { mutableStateOf("") }

    val pricingRule = TokenDistributionChangeRules
        .parse(token.distributionChangeRules)
        ?.changeDirectPurchasePricingRules
    val banner = GroupActionRuleEvaluator.bannerState(
        pricingRule?.toJson(), token.mainControlGroupPosition,
    )
    // Flat credits-per-token price; 0 disables direct purchase.
    val price = priceText.trim().toULongOrNull()
    val canSubmit = price != null

    TokenActionScaffold(
        title = "Set Price",
        context = context,
        banner = banner,
        bannerActionLabel = "price change",
        submitLabel = "Set Price",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val pricePerToken = price ?: return@TokenActionScaffold
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                wallet.tokens.setPrice(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    pricePerToken = pricePerToken,
                    publicNote = noteOrNull,
                    groupAction = groupAction,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
            }
        },
    ) {
        FormSection {
            Text(
                "Set the per-token price for direct purchase. Buyers pay credits at this " +
                    "price; enter 0 to disable direct purchase.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        FormSection(title = "Price per token (credits)") {
            OutlinedTextField(
                value = priceText,
                onValueChange = { priceText = it },
                label = { Text("Credits") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                isError = priceText.isNotBlank() && price == null,
                supportingText = {
                    if (priceText.isNotBlank() && price == null) {
                        Text("Enter a whole, non-negative number of credits.")
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp)
                    .testTag("tokenAction.priceField"),
            )
        }
        PublicNoteField(note) { note = it }
    }
}

// ── Update Max Supply (← TokenUpdateMaxSupplyActionView.swift) ─────────

@Composable
private fun UpdateMaxSupplyForm(
    context: TokenActionContext,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
) {
    val token = context.token
    var removeCap by rememberSaveable { mutableStateOf(false) }
    var amountText by rememberSaveable { mutableStateOf("") }
    var note by rememberSaveable { mutableStateOf("") }

    val banner = GroupActionRuleEvaluator.bannerState(
        token.maxSupplyChangeRules, token.mainControlGroupPosition,
    )
    val amountRaw = TokenAmounts.parse(amountText, token.decimals)
    val canSubmit = removeCap || (amountRaw != null && amountRaw > 0u)

    TokenActionScaffold(
        title = "Update Max Supply",
        context = context,
        banner = banner,
        bannerActionLabel = "max-supply change",
        submitLabel = "Update Max Supply",
        canSubmit = canSubmit,
        viewModel = viewModel,
        navController = navController,
        tokenSummaryRows = {
            LabeledContent(
                "Current max supply",
                token.maxSupply?.let { TokenAmounts.format(it, token.decimals) } ?: "Unlimited",
            )
        },
        onSubmit = {
            val wallet = context.wallet ?: return@TokenActionScaffold
            val signer = context.signerHandle ?: return@TokenActionScaffold
            val change = if (removeCap) {
                TokenConfigChange.MaxSupply(null)
            } else {
                TokenConfigChange.MaxSupply(amountRaw ?: return@TokenActionScaffold)
            }
            val groupAction = banner.toGroupAction()
            val noteOrNull = note.toPublicNoteOrNull()
            viewModel.submit {
                wallet.tokens.updateConfig(
                    identityId = context.identity.identityId,
                    tokenContractId = token.contractId,
                    tokenPosition = token.position,
                    change = change,
                    publicNote = noteOrNull,
                    groupAction = groupAction,
                    signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                    signerHandle = signer,
                )
            }
        },
    ) {
        FormSection(title = "New max supply") {
            androidx.compose.foundation.layout.Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "Remove cap",
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.weight(1f),
                )
                Switch(
                    checked = removeCap,
                    onCheckedChange = { removeCap = it },
                    modifier = Modifier.testTag("tokenAction.removeCapToggle"),
                )
            }
            if (!removeCap) {
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it },
                    label = { Text("Amount") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .testTag("tokenAction.amountField"),
                )
            }
        }
        PublicNoteField(note) { note = it }
    }
}

// ── Deferred editors (← TokenActionPlaceholderView) ────────────────────

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun DeferredActionScreen(actionId: String, navController: NavHostController) {
    val kind = TokenActionKind.fromId(actionId)
    androidx.compose.material3.Scaffold(
        topBar = {
            androidx.compose.material3.TopAppBar(
                title = { Text(kind?.title ?: actionId) },
                navigationIcon = {
                    androidx.compose.material3.IconButton(
                        onClick = { navController.popBackStack() },
                    ) {
                        androidx.compose.material3.Icon(
                            Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                        )
                    }
                },
            )
        },
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                "This action is deferred to a future release",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

private fun BannerState.toGroupAction(): GroupAction = when (this) {
    is BannerState.Propose -> GroupAction.Propose(groupPosition)
    BannerState.None -> GroupAction.None
}
