package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.navigation.CoSignProposal
import org.dashfoundation.example.services.tokens.GroupActionProposal
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator
import org.dashfoundation.example.util.truncateMiddle

/**
 * Pending group-action proposals for a `(token, identity)` pair — port
 * of `PendingGroupActionsView.swift`. Computes the group positions the
 * token's rules reference (deduped across `MainGroup` / `Group:<n>`),
 * queries each via `Groups.pendingActions`, dedupes by action id, and
 * navigates rows into [CoSignProposal] carrying the proposal JSON + the
 * group position it was discovered under.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PendingGroupActionsScreen(
    tokenIdHex: String,
    identityIdHex: String,
    navController: NavHostController,
) {
    val context = rememberTokenActionContext(tokenIdHex, identityIdHex)

    var isLoading by remember { mutableStateOf(false) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var proposals by remember {
        mutableStateOf<List<Pair<Int, GroupActionProposal>>>(emptyList())
    }
    var reloadKey by remember { mutableIntStateOf(0) }

    val token = context?.token
    val wallet = context?.wallet

    LaunchedEffect(token?.id?.contentHashCode(), wallet, reloadKey) {
        val currentToken = token ?: return@LaunchedEffect
        if (wallet == null) {
            loadError = "Wallet not loaded"
            return@LaunchedEffect
        }
        isLoading = true
        loadError = null
        try {
            val positions = GroupActionRuleEvaluator.relevantGroupPositions(currentToken)
            val collected = ArrayList<Pair<Int, GroupActionProposal>>()
            for (position in positions) {
                val json = wallet.groups.pendingActions(
                    tokenContractId = currentToken.contractId,
                    groupContractPosition = position,
                )
                for (proposal in GroupActionProposal.parseList(json)) {
                    collected.add(position to proposal)
                }
            }
            // Dedupe by action id — first occurrence wins
            // (← TokenGroupRuleResolver.dedupeByActionId).
            val seen = HashSet<String>()
            proposals = collected.filter { seen.add(it.second.actionIdBase58) }
        } catch (e: Exception) {
            loadError = e.message ?: e.toString()
        } finally {
            isLoading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Pending Group Actions") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { reloadKey++ },
                        modifier = Modifier.testTag("pendingGroupActions.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        when {
            context == null || isLoading -> Row(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(24.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                CircularProgressIndicator(modifier = Modifier.size(20.dp))
                Text(
                    "Loading pending group actions…",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            loadError != null -> Text(
                loadError ?: "",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier
                    .padding(padding)
                    .padding(24.dp)
                    .testTag("pendingGroupActions.error"),
            )
            proposals.isEmpty() -> Text(
                "No pending group actions for this token.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .padding(padding)
                    .padding(24.dp),
            )
            else -> LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(proposals, key = { it.second.actionIdBase58 }) { (position, proposal) ->
                    val isUserProposer =
                        proposal.proposerBase58 == context.identity.identityId.let {
                            org.dashfoundation.example.util.Base58.encode(it)
                        }
                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                navController.navigate(
                                    CoSignProposal(
                                        tokenIdHex = tokenIdHex,
                                        identityIdHex = identityIdHex,
                                        groupPosition = position,
                                        proposalJson = proposal.rawJson,
                                    ),
                                )
                            }
                            .testTag("pendingGroupActions.row.${proposal.actionIdBase58}"),
                    ) {
                        ListItem(
                            headlineContent = { Text(proposal.displayLabel) },
                            supportingContent = {
                                Column {
                                    Text(proposal.summary)
                                    Text(
                                        "Proposer ${truncateMiddle(proposal.proposerBase58, 6, 4)}" +
                                            " · Action ${truncateMiddle(proposal.actionIdBase58, 6, 4)}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            },
                            trailingContent = {
                                if (isUserProposer) {
                                    Text(
                                        "Proposed by you",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.primary,
                                    )
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}
