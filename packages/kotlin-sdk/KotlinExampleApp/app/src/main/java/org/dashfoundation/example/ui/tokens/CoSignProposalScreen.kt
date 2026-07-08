package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.tokens.GroupAction
import org.dashfoundation.dashsdk.tokens.Groups
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.CoSignProposal
import org.dashfoundation.example.services.tokens.GroupActionProposal
import org.dashfoundation.example.services.tokens.GroupActionSigner
import org.dashfoundation.example.services.tokens.ContractGroups
import org.dashfoundation.example.services.tokens.ProvenBalances
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.truncateMiddle

/**
 * Review-and-sign for an existing group-action proposal — port of
 * `CoSignProposalView.swift`. Loads the current signer set
 * (`Groups.actionSigners`) + the contract's group membership, renders
 * the original parameters read-only, and dispatches the matching
 * `Tokens.<action>` with `GroupAction.SignExisting(position, actionId,
 * actionIsProposer)` — the co-signer never edits the payload.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CoSignProposalScreen(route: CoSignProposal, navController: NavHostController) {
    val container = LocalAppContainer.current
    val context = rememberTokenActionContext(route.tokenIdHex, route.identityIdHex)
    val proposal = remember(route.proposalJson) { GroupActionProposal.parse(route.proposalJson) }
    val viewModel: TokenActionViewModel = viewModel()

    var signers by remember { mutableStateOf<List<GroupActionSigner>>(emptyList()) }
    var isLoadingSigners by remember { mutableStateOf(false) }
    var loadError by remember { mutableStateOf<String?>(null) }

    val wallet = context?.wallet

    LaunchedEffect(wallet, proposal?.actionIdBase58) {
        val currentProposal = proposal ?: return@LaunchedEffect
        val currentContext = context ?: return@LaunchedEffect
        if (wallet == null) return@LaunchedEffect
        val actionId = Base58.decodeIdentifier(currentProposal.actionIdBase58)
            ?: return@LaunchedEffect
        isLoadingSigners = true
        loadError = null
        try {
            val json = wallet.groups.actionSigners(
                tokenContractId = currentContext.token.contractId,
                groupContractPosition = route.groupPosition,
                status = Groups.Status.ACTIVE,
                actionId = actionId,
            )
            signers = GroupActionSigner.parseList(json)
        } catch (e: Exception) {
            loadError = e.message ?: e.toString()
        } finally {
            isLoadingSigners = false
        }
    }

    LaunchedEffect(viewModel.completed) {
        if (viewModel.completed) navController.popBackStack()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Co-sign Proposal") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        if (context == null || proposal == null) {
            Column(modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp)) {
                if (proposal == null && context != null) {
                    Text(
                        "This proposal couldn't be decoded.",
                        color = MaterialTheme.colorScheme.error,
                    )
                } else {
                    CircularProgressIndicator()
                }
            }
            return@Scaffold
        }

        val identityBase58 = Base58.encode(context.identity.identityId)
        val groups = ContractGroups.from(context.contract)
        val members = groups?.members(route.groupPosition) ?: emptyMap()
        val requiredPower = groups?.requiredPower(route.groupPosition) ?: 0
        val totalPower = signers.sumOf { it.power }
        val isUserMember = members.containsKey(identityBase58)
        val hasUserSigned = signers.any { it.identityIdBase58 == identityBase58 }
        val isUserProposer = proposal.proposerBase58 == identityBase58

        val canCoSign = wallet != null && isUserMember && !hasUserSigned &&
            proposal.isCoSignSupported
        val disabledReason = when {
            !isUserMember -> "Only members of the gating group can sign."
            hasUserSigned -> "You already signed this proposal."
            !proposal.isCoSignSupported ->
                "This proposal type isn't supported by the co-sign UI yet."
            else -> null
        }
        val buttonLabel = when {
            hasUserSigned -> "Already signed"
            !isUserMember -> "Not a group member"
            isUserProposer -> "Re-sign as proposer"
            else -> "Co-sign"
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Proposal") {
                LabeledContent("Type", proposal.displayLabel)
                LabeledContent("Action ID", truncateMiddle(proposal.actionIdBase58, 8, 6))
                LabeledContent("Proposer", truncateMiddle(proposal.proposerBase58, 8, 6))
                LabeledContent("Token position", "${proposal.tokenContractPosition}")
                LabeledContent("Group position", "${route.groupPosition}")
            }

            FormSection(title = "Parameters") {
                ProposalParams(proposal)
            }

            FormSection(title = "Signatures") {
                if (isLoadingSigners) {
                    CircularProgressIndicator(modifier = Modifier.padding(8.dp))
                } else {
                    LabeledContent("Total power", "$totalPower of $requiredPower")
                    LabeledContent(
                        "Threshold",
                        "${signers.size} of ${members.size} members signed",
                    )
                    Text(
                        when {
                            hasUserSigned -> "You have signed this proposal."
                            isUserMember -> "You have not signed yet."
                            else -> "You are not a member of this group."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = when {
                            hasUserSigned -> MaterialTheme.colorScheme.primary
                            isUserMember -> MaterialTheme.colorScheme.tertiary
                            else -> MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        modifier = Modifier.testTag("coSign.signStatus"),
                    )
                    loadError?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }

            if (signers.isNotEmpty()) {
                FormSection(title = "Signers") {
                    signers.forEach { signer ->
                        LabeledContent(
                            truncateMiddle(signer.identityIdBase58, 8, 6),
                            "Power ${signer.power}" +
                                if (signer.identityIdBase58 == proposal.proposerBase58) {
                                    " · Proposer"
                                } else {
                                    ""
                                },
                        )
                    }
                }
            }

            SubmitButton(
                text = buttonLabel,
                isLoading = viewModel.isSubmitting,
                enabled = canCoSign,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("coSign.submitButton"),
            ) {
                val currentWallet = wallet ?: return@SubmitButton
                val signer = context.signerHandle ?: return@SubmitButton
                val actionId = Base58.decodeIdentifier(proposal.actionIdBase58)
                    ?: return@SubmitButton
                val mode = GroupAction.SignExisting(
                    position = route.groupPosition,
                    actionId = actionId,
                    actionIsProposer = isUserProposer,
                )
                viewModel.submit {
                    dispatchCoSign(
                        proposal = proposal,
                        context = context,
                        mode = mode,
                        signerHandle = signer,
                        onBalances = { balances ->
                            // A signature that CLOSES a group mint / burn
                            // returns the proof-verified post-action balance —
                            // persist it so the closing co-signer's row updates
                            // immediately (← persistCoSignedBurnBalances).
                            ProvenBalances.persist(
                                balances, context.token, container.database.tokenDao(),
                            )
                        },
                    )
                }
            }
            disabledReason?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    ErrorAlertDialog(
        message = viewModel.error,
        title = "Co-sign failed",
        onDismiss = { viewModel.error = null },
    )
}

/** Read-only parameter rows per proposal type (← `paramsSection`). */
@Composable
private fun ProposalParams(proposal: GroupActionProposal) {
    val note = proposal.paramString("publicNote")
    when (proposal.type) {
        "mint" -> {
            LabeledContent("Amount", proposal.paramString("amount") ?: "?")
            LabeledContent(
                "Recipient",
                truncateMiddle(proposal.paramString("recipient") ?: "?", 8, 6),
            )
        }
        "burn" -> {
            LabeledContent("Amount", proposal.paramString("amount") ?: "?")
            LabeledContent(
                "Burn from",
                truncateMiddle(proposal.paramString("burnFrom") ?: "?", 8, 6),
            )
        }
        "freeze", "unfreeze" -> LabeledContent(
            "Target",
            truncateMiddle(proposal.paramString("target") ?: "?", 8, 6),
        )
        "destroyFrozenFunds" -> {
            LabeledContent(
                "Target",
                truncateMiddle(proposal.paramString("target") ?: "?", 8, 6),
            )
            LabeledContent("Amount", proposal.paramString("amount") ?: "?")
        }
        "pause" -> Text(
            "Halts all token operations.",
            style = MaterialTheme.typography.bodyMedium,
        )
        "resume" -> Text(
            "Resumes all token operations.",
            style = MaterialTheme.typography.bodyMedium,
        )
        "setPrice" -> {
            val price = proposal.paramString("pricePerToken")
            if (price != null) {
                LabeledContent("Price per token", "$price credits")
            } else {
                Text("Disables direct purchase.", style = MaterialTheme.typography.bodyMedium)
            }
        }
        "directPurchase" -> {
            LabeledContent("Amount", proposal.paramString("amount") ?: "?")
            LabeledContent("Total cost", "${proposal.paramString("totalCost") ?: "?"} credits")
        }
        else -> Text(
            "This proposal type isn't supported by the co-sign UI yet.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
    note?.let { LabeledContent("Note", it) }
}

/**
 * Route to the right `Tokens.<action>` from the proposal payload,
 * passing the original parameters straight through — port of
 * `CoSignProposalView.dispatch`.
 */
private suspend fun dispatchCoSign(
    proposal: GroupActionProposal,
    context: TokenActionContext,
    mode: GroupAction,
    signerHandle: Long,
    onBalances: suspend (String?) -> Unit,
) {
    val wallet = context.wallet ?: throw IllegalStateException("Wallet not loaded")
    val identityId = context.identity.identityId
    val contractId = context.token.contractId
    val tokenPosition = proposal.tokenContractPosition
    val note = proposal.paramString("publicNote")

    when (proposal.type) {
        "mint" -> {
            val balances = wallet.tokens.mint(
                identityId = identityId,
                tokenContractId = contractId,
                tokenPosition = tokenPosition,
                amount = proposal.paramAmount()
                    ?: throw IllegalStateException("Proposal amount missing"),
                issuedToIdentityId = proposal.paramString("recipient")
                    ?.let { Base58.decodeIdentifier(it) },
                publicNote = note,
                groupAction = mode,
                signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                signerHandle = signerHandle,
            )
            onBalances(balances)
        }
        "burn" -> {
            // The proposal carries `burnFrom`, but the FFI burns from
            // `identityId` — Platform validates the signature against the
            // original proposal's target (← the Swift comment).
            val balances = wallet.tokens.burn(
                identityId = identityId,
                tokenContractId = contractId,
                tokenPosition = tokenPosition,
                amount = proposal.paramAmount()
                    ?: throw IllegalStateException("Proposal amount missing"),
                publicNote = note,
                groupAction = mode,
                signingKeyId = TokenActionContext.SIGNING_KEY_ID,
                signerHandle = signerHandle,
            )
            onBalances(balances)
        }
        "freeze" -> wallet.tokens.freeze(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            frozenIdentityId = proposal.requiredTarget(),
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        "unfreeze" -> wallet.tokens.unfreeze(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            frozenIdentityId = proposal.requiredTarget(),
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        "destroyFrozenFunds" -> wallet.tokens.destroyFrozenFunds(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            frozenIdentityId = proposal.requiredTarget(),
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        "pause" -> wallet.tokens.pause(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        "resume" -> wallet.tokens.resume(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        "setPrice" -> wallet.tokens.setPrice(
            identityId = identityId,
            tokenContractId = contractId,
            tokenPosition = tokenPosition,
            pricePerToken = proposal.paramAmount("pricePerToken") ?: 0,
            publicNote = note,
            groupAction = mode,
            signingKeyId = TokenActionContext.SIGNING_KEY_ID,
            signerHandle = signerHandle,
        )
        else -> throw IllegalStateException(
            "Proposal type not supported by co-sign UI.",
        )
    }
}

private fun GroupActionProposal.requiredTarget(): ByteArray =
    paramString("target")?.let { Base58.decodeIdentifier(it) }
        ?: throw IllegalStateException("Proposal target missing")
