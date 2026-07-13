package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.voting.VoteChoice
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.truncateMiddle

/**
 * DPNS contest detail — port of `ContestDetailView.swift`.
 *
 * Live contest state loads through the bridged read query
 * `Voting.contestedResourceVoteState` (`dash_sdk_contested_resource_get_vote_state`,
 * result type `DocumentsAndVoteTally`) over the same DPNS vote-poll tuple
 * the iOS read path uses; vote casting broadcasts through
 * `VoteCasting.castVote` (`dash_sdk_contested_resource_cast_vote`) with the
 * masternode credential form the iOS `CastVoteSheet` collects. The iOS
 * wallet path additionally reports the poll's end time; the bridged query
 * carries tallies + winner only, so the status row derives from winner
 * presence alone.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContestDetailScreen(
    contestName: String,
    identityIdHex: String,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val viewerBase58 = remember(identityIdHex) {
        identityIdHex.takeIf { it.isNotEmpty() }?.let { Base58.encode(it.hexToBytes()) }
    }

    var voteState by remember { mutableStateOf<ContestVoteState?>(null) }
    var isRefreshing by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var refreshTick by remember { mutableStateOf(0) }

    var showVoteSheet by remember { mutableStateOf(false) }
    var isCastingVote by remember { mutableStateOf(false) }
    var voteResultMessage by remember { mutableStateOf<String?>(null) }
    var voteResultIsError by remember { mutableStateOf(false) }

    LaunchedEffect(sdk, contestName, refreshTick) {
        val currentSdk = sdk ?: return@LaunchedEffect
        isRefreshing = true
        try {
            val json = currentSdk.voting.contestedResourceVoteState(
                contractId = DPNS_CONTRACT_ID_BASE58,
                documentTypeName = DPNS_DOCUMENT_TYPE,
                indexName = DPNS_INDEX_NAME,
                indexValuesJson = dpnsIndexValuesJson(contestName),
                resultType = RESULT_TYPE_DOCUMENTS_AND_VOTE_TALLY,
            )
            voteState = json?.let(::parseContestVoteState)
            errorMessage = if (voteState == null) {
                "Contest not visible — it may have resolved, or no one is contending " +
                    "this name."
            } else {
                null
            }
        } catch (e: Exception) {
            errorMessage = "Refresh failed: ${e.message}"
        } finally {
            isRefreshing = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Contest Details") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        onClick = { refreshTick++ },
                        enabled = !isRefreshing,
                        modifier = Modifier.testTag("contestDetail.refresh"),
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Contest Information") {
                LabeledContent("Name", "$contestName.dash")
                val state = voteState
                when {
                    isRefreshing && state == null -> LabeledContent("Status", "Loading…")
                    state == null -> LabeledContent("Status", "Unavailable")
                    state.winner == null -> LabeledContent("Status", "Voting Ongoing")
                    state.winner is ContestWinner.WonByIdentity -> {
                        LabeledContent("Status", "Resolved")
                        LabeledContent("Winner", truncateMiddle(state.winner.identityIdBase58, 10, 6))
                    }
                    else -> LabeledContent("Status", "Locked")
                }
                if (viewerBase58 != null) {
                    LabeledContent("Viewing As", truncateMiddle(viewerBase58, 10, 6))
                }
            }

            errorMessage?.let { message ->
                Text(
                    message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("contestDetail.error"),
                )
            }

            FormSection(title = "Contenders") {
                val contenders = voteState?.sortedContenders().orEmpty()
                if (contenders.isEmpty()) {
                    Text(
                        "No contenders yet",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("contestDetail.contendersPlaceholder"),
                    )
                } else {
                    if (contenders.size == 1 && contenders.first().identityIdBase58 == viewerBase58) {
                        Text(
                            "You are currently the only contender for this name. Other " +
                                "users can still join until the contest's halfway point.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    contenders.forEach { contender ->
                        val label = if (contender.identityIdBase58 == viewerBase58) {
                            "You · ${truncateMiddle(contender.identityIdBase58, 8, 6)}"
                        } else {
                            truncateMiddle(contender.identityIdBase58, 8, 6)
                        }
                        LabeledContent(
                            label,
                            "${contender.voteCount} vote" +
                                if (contender.voteCount == 1L) "" else "s",
                        )
                    }
                }
            }

            FormSection(title = "Vote Summary") {
                val state = voteState
                LabeledContent("Abstain Votes", "${state?.abstainVotes ?: 0}")
                LabeledContent("Lock Votes", "${state?.lockVotes ?: 0}")
                LabeledContent("Total Votes", "${state?.totalVotes() ?: 0}")
            }

            FormSection(title = "Cast a Masternode Vote") {
                voteResultMessage?.let { message ->
                    Text(
                        message,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (voteResultIsError) {
                            MaterialTheme.colorScheme.error
                        } else {
                            MaterialTheme.colorScheme.primary
                        },
                        modifier = Modifier.testTag("contestDetail.voteResult"),
                    )
                }
                SubmitButton(
                    text = "Cast Vote…",
                    isLoading = isCastingVote,
                    enabled = sdk != null && manager != null && !isCastingVote,
                    modifier = Modifier.fillMaxWidth().testTag("contestDetail.castVote"),
                ) {
                    voteResultMessage = null
                    showVoteSheet = true
                }
                Text(
                    "Only masternodes can vote on contested names, using a masternode " +
                        "voting key tied to a pro_tx_hash. A regular wallet has no " +
                        "voting key, so a vote here will be rejected by Platform. " +
                        "Supply real masternode credentials to cast an accepted vote.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "About Contested Names") {
                Text(
                    "When multiple identities want the same DPNS username, " +
                        "masternodes vote to decide the winner. The identity with " +
                        "the most votes will be awarded the name when voting ends.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    if (showVoteSheet) {
        CastVoteDialog(
            contestName = contestName,
            contenders = voteState?.sortedContenders().orEmpty(),
            viewerBase58 = viewerBase58,
            isCasting = isCastingVote,
            onDismiss = { if (!isCastingVote) showVoteSheet = false },
            onSubmit = { choice, proTxHashHex, votingKeyHex ->
                val currentSdk = sdk
                val mgr = manager
                if (currentSdk == null || mgr == null) {
                    voteResultMessage = "SDK not initialized"
                    voteResultIsError = true
                    showVoteSheet = false
                    return@CastVoteDialog
                }
                val proTxHash = decode32ByteHex(proTxHashHex)
                if (proTxHash == null) {
                    voteResultMessage = "pro_tx_hash must be 32 bytes (64 hex characters)."
                    voteResultIsError = true
                    return@CastVoteDialog
                }
                val votingKey = decode32ByteHex(votingKeyHex)
                if (votingKey == null) {
                    voteResultMessage = "Voting private key must be 32 bytes (64 hex characters)."
                    voteResultIsError = true
                    return@CastVoteDialog
                }
                isCastingVote = true
                scope.launch {
                    try {
                        mgr.voteCasting.castVote(
                            sdk = currentSdk,
                            dataContractId = DPNS_CONTRACT_ID_BASE58,
                            documentTypeName = DPNS_DOCUMENT_TYPE,
                            indexName = DPNS_INDEX_NAME,
                            indexValues = listOf("dash", contestName),
                            choice = choice,
                            proTxHash = proTxHash,
                            votingPrivateKey = votingKey,
                            networkOrd = network.ffiValue,
                        )
                        voteResultMessage = "Vote accepted by Platform."
                        voteResultIsError = false
                        showVoteSheet = false
                        refreshTick++ // pull fresh tallies after a successful cast
                    } catch (e: Exception) {
                        voteResultMessage = "Vote rejected: ${e.message}"
                        voteResultIsError = true
                        showVoteSheet = false
                    } finally {
                        isCastingVote = false
                    }
                }
            },
        )
    }
}

/**
 * Modal for picking a vote choice and entering masternode credentials —
 * port of `CastVoteSheet` in `ContestDetailView.swift`.
 */
@Composable
private fun CastVoteDialog(
    contestName: String,
    contenders: List<ContestContender>,
    viewerBase58: String?,
    isCasting: Boolean,
    onDismiss: () -> Unit,
    onSubmit: (VoteChoice, String, String) -> Unit,
) {
    var selection by remember { mutableStateOf<VoteSelection>(VoteSelection.Abstain) }
    var proTxHashHex by remember { mutableStateOf("") }
    var votingKeyHex by remember { mutableStateOf("") }

    val options = contenders.map { VoteSelection.Contender(it.identityIdBase58) } +
        listOf(VoteSelection.Abstain, VoteSelection.Lock)

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Cast Vote · $contestName.dash") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                AccessiblePicker(
                    label = "Vote",
                    options = options,
                    selected = selection,
                    optionLabel = { option ->
                        when (option) {
                            is VoteSelection.Contender ->
                                if (option.identityIdBase58 == viewerBase58) {
                                    "You"
                                } else {
                                    truncateMiddle(option.identityIdBase58, 8, 6)
                                }
                            VoteSelection.Abstain -> "Abstain"
                            VoteSelection.Lock -> "Lock (no winner)"
                        }
                    },
                    testTag = "contestDetail.votePicker",
                    onSelected = { selection = it },
                )
                OutlinedTextField(
                    value = proTxHashHex,
                    onValueChange = { proTxHashHex = it },
                    label = { Text("pro_tx_hash (64 hex chars)") },
                    singleLine = true,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("contestDetail.proTxHash"),
                )
                OutlinedTextField(
                    value = votingKeyHex,
                    onValueChange = { votingKeyHex = it },
                    label = { Text("voting private key (64 hex chars)") },
                    singleLine = true,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    modifier = Modifier.fillMaxWidth().testTag("contestDetail.votingKey"),
                )
                Text(
                    "These belong to a masternode. The voting public key " +
                        "(ECDSA_HASH160) and signer are derived from the private key " +
                        "on the Rust side; the key bytes are not stored.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        confirmButton = {
            TextButton(
                enabled = !isCasting && proTxHashHex.isNotBlank() && votingKeyHex.isNotBlank(),
                onClick = {
                    val choice = when (val current = selection) {
                        is VoteSelection.Contender ->
                            VoteChoice.TowardsIdentity(current.identityIdBase58)
                        VoteSelection.Abstain -> VoteChoice.Abstain
                        VoteSelection.Lock -> VoteChoice.Lock
                    }
                    onSubmit(choice, proTxHashHex, votingKeyHex)
                },
                modifier = Modifier.testTag("contestDetail.submitVote"),
            ) { Text(if (isCasting) "Broadcasting…" else "Submit Vote") }
        },
        dismissButton = {
            TextButton(enabled = !isCasting, onClick = onDismiss) { Text("Cancel") }
        },
    )
}

private sealed interface VoteSelection {
    data class Contender(val identityIdBase58: String) : VoteSelection
    data object Abstain : VoteSelection
    data object Lock : VoteSelection
}

// ── Vote-state model + JSON parsing ─────────────────────────────────────

/** One contender row from the vote-state query. */
data class ContestContender(val identityIdBase58: String, val voteCount: Long)

/** Winner info once a contest resolves. */
sealed interface ContestWinner {
    data class WonByIdentity(val identityIdBase58: String) : ContestWinner
    data object Locked : ContestWinner
    data object NoWinner : ContestWinner
}

/** Parsed `dash_sdk_contested_resource_get_vote_state` response. */
data class ContestVoteState(
    val contenders: List<ContestContender>,
    val abstainVotes: Long,
    val lockVotes: Long,
    /** Null while voting is ongoing (no winner block written yet). */
    val winner: ContestWinner?,
) {
    /** Contenders by tally descending, ties broken by identity id. */
    fun sortedContenders(): List<ContestContender> =
        contenders.sortedWith(
            compareByDescending<ContestContender> { it.voteCount }
                .thenBy { it.identityIdBase58 },
        )

    fun totalVotes(): Long = abstainVotes + lockVotes + contenders.sumOf { it.voteCount }
}

/**
 * Parse the vote-state JSON:
 * `{"abstain_vote_tally":N,"lock_vote_tally":N,("winner_info":…,"block_info":…,)?
 * "contenders":[{"identity_id":"b58","vote_count":N,…}]}`.
 */
internal fun parseContestVoteState(json: String): ContestVoteState? = try {
    val root = LenientJson.parseToJsonElement(json).jsonObject
    val contenders = (root["contenders"] as? JsonArray)
        ?.mapNotNull { it as? JsonObject }
        ?.mapNotNull { row ->
            val id = (row["identity_id"] as? JsonPrimitive)?.content ?: return@mapNotNull null
            val count = (row["vote_count"] as? JsonPrimitive)?.content?.toLongOrNull() ?: 0L
            ContestContender(id, count)
        }
        .orEmpty()
    val winner = when (val info = root["winner_info"]) {
        null -> null
        is JsonPrimitive -> when (info.content) {
            "Locked" -> ContestWinner.Locked
            "NoWinner" -> ContestWinner.NoWinner
            else -> null
        }
        is JsonObject -> (info["identity_id"] as? JsonPrimitive)?.content
            ?.let { ContestWinner.WonByIdentity(it) }
        else -> null
    }
    ContestVoteState(
        contenders = contenders,
        abstainVotes = (root["abstain_vote_tally"] as? JsonPrimitive)
            ?.content?.toLongOrNull() ?: 0L,
        lockVotes = (root["lock_vote_tally"] as? JsonPrimitive)
            ?.content?.toLongOrNull() ?: 0L,
        winner = winner,
    )
} catch (_: Exception) {
    null
}

/** Decode exactly 32 bytes of hex; null on any other shape. */
private fun decode32ByteHex(input: String): ByteArray? {
    val trimmed = input.trim()
    if (trimmed.length != 64 || !trimmed.all { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }) {
        return null
    }
    return trimmed.hexToBytes()
}

/**
 * DPNS contest poll shape — the constants `ContestDetailView.swift`
 * hard-codes for the `(contract, document type, index)` tuple. The
 * contested label lives at the second index value (`["dash", label]`).
 */
internal const val DPNS_CONTRACT_ID_BASE58 = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
internal const val DPNS_DOCUMENT_TYPE = "domain"
internal const val DPNS_INDEX_NAME = "parentNameAndLabel"

/** `Documents_AND_VOTE_TALLY` result-type discriminant (rs-sdk-ffi). */
internal const val RESULT_TYPE_DOCUMENTS_AND_VOTE_TALLY = 2

/** JSON index-values array for a DPNS label, with minimal escaping. */
internal fun dpnsIndexValuesJson(label: String): String {
    val escaped = label.replace("\\", "\\\\").replace("\"", "\\\"")
    return "[\"dash\",\"$escaped\"]"
}
