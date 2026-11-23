package org.dashfoundation.example.ui.contracts.moderation

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.moderation.ContractFeePotState
import org.dashfoundation.dashsdk.moderation.ContractFeePots
import org.dashfoundation.dashsdk.moderation.ContractModerationActionCounts
import org.dashfoundation.dashsdk.moderation.ContractModerationEntry
import org.dashfoundation.dashsdk.moderation.ContractModerationList
import org.dashfoundation.dashsdk.moderation.ContractModerationStatus
import org.dashfoundation.example.ui.components.FormSection

/** Rows per page of every paged moderation read. */
internal const val MODERATION_PAGE_SIZE: UInt = 20u

/**
 * The two fee pots of a contract: the balance of each and its last explicit claim. Port of
 * `ContractFeePotsSection.swift`.
 */
@Composable
internal fun ContractFeePotsSection(sdk: Sdk, contractId: String, isCurrent: () -> Boolean) {
    val scope = rememberCoroutineScope()
    val loader = remember(contractId) { ModerationSingleLoader<ContractFeePots?>() }
    val reload: suspend () -> Unit = {
        loader.reload({ sdk.contractModeration.feePots(contractId) }, isCurrent)
    }
    LaunchedEffect(contractId) { reload() }

    FormSection(title = "Fee Pots") {
        when (val load = loader.load) {
            ModerationLoad.Idle, ModerationLoad.Loading -> ModerationMessageRow(
                ModerationMessageKind.LOADING,
                "Loading fee pots…",
                "contractModeration.feePots.loading",
            )
            is ModerationLoad.Failed -> ModerationMessageRow(
                ModerationMessageKind.ERROR,
                load.message,
                "contractModeration.feePots.error",
            )
            is ModerationLoad.Loaded -> {
                val pots = load.value
                if (pots == null) {
                    ModerationMessageRow(
                        ModerationMessageKind.EMPTY,
                        "No answer for this contract.",
                        "contractModeration.feePots.empty",
                    )
                } else {
                    FeePotRow("Owner Pot", pots.owner, "contractModeration.feePots.owner")
                    FeePotRow("Moderators Pot", pots.moderators, "contractModeration.feePots.moderators")
                }
            }
        }
        TextButton(
            onClick = { scope.launch { reload() } },
            enabled = !loader.isLoading,
            modifier = Modifier.testTag("contractModeration.feePots.reload"),
        ) { Text("Reload") }
        HubFooter(
            "Each pot's balance from the contract's document action fees. The last claim is " +
                "the last explicit claim, at most one per epoch; a membership change of an " +
                "elected team can also pay out the moderators pot without changing it.",
        )
    }
}

/** One pot: its balance and, once it was claimed, the last explicit claim. */
@Composable
private fun FeePotRow(title: String, pot: ContractFeePotState, testTag: String) {
    Column(
        verticalArrangement = Arrangement.spacedBy(2.dp),
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .semantics(mergeDescendants = true) {}
            .testTag(testTag),
    ) {
        Text(title, style = MaterialTheme.typography.titleSmall)
        ModerationFieldRow("Balance (credits)", pot.credits.toString())
        val epoch = pot.lastClaimEpoch
        if (epoch != null) {
            ModerationFieldRow("Last claim epoch", epoch.toString())
            pot.lastClaimTimeMs?.let { ModerationFieldRow("Last claim time", moderationDate(it)) }
            pot.lastClaimantId?.let { ModerationFieldRow("Last claimant", it, monospaced = true) }
        } else {
            Text(
                "Never claimed",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/**
 * One moderation list of a contract, a page at a time, in identity id order. "Load More"
 * passes the cursor the last page returned; a page without one was the last. Port of
 * `ContractModerationEntriesView.swift`; the Reload button stands in for its pull to refresh.
 */
@Composable
internal fun ContractModerationEntriesContent(
    sdk: Sdk,
    contractId: String,
    list: ContractModerationList,
    isCurrent: () -> Boolean,
) {
    val scope = rememberCoroutineScope()
    val loader = remember(list) { ModerationPagedLoader<ContractModerationEntry, String>() }
    val reload: suspend () -> Unit = {
        loader.reload(
            { startAfter ->
                sdk.contractModeration.entries(contractId, list, startAfter, MODERATION_PAGE_SIZE)
                    ?.let { ModerationPage(it.entries, it.nextStartAfter) }
            },
            isCurrent,
        )
    }
    LaunchedEffect(list) { reload() }

    loader.errorMessage?.let {
        FormSection {
            ModerationMessageRow(ModerationMessageKind.ERROR, it, "contractModeration.entries.error")
        }
    }
    FormSection(title = list.title) {
        if (loader.items.isEmpty()) {
            if (loader.isLoading) {
                ModerationMessageRow(
                    ModerationMessageKind.LOADING,
                    "Loading entries…",
                    "contractModeration.entries.loading",
                )
            } else if (loader.loadedOnce && loader.errorMessage == null) {
                ModerationMessageRow(
                    ModerationMessageKind.EMPTY,
                    "Nobody is on this list.",
                    "contractModeration.entries.empty",
                )
            }
        }
        loader.items.forEach { entry -> ContractModerationEntryRow(entry) }
        if (loader.nextCursor != null && loader.items.isNotEmpty()) {
            ModerationLoadMoreButton(loader.isLoading, "contractModeration.entries.loadMore") {
                scope.launch { loader.loadMore() }
            }
        }
        TextButton(
            onClick = { scope.launch { reload() } },
            enabled = !loader.isLoading,
            modifier = Modifier.testTag("contractModeration.entries.reload"),
        ) { Text("Reload") }
    }
}

/**
 * One entry: the identity, why it is on the list, and the end of a suspension or, for a
 * warning list entry, every warning it carries.
 */
@Composable
private fun ContractModerationEntryRow(entry: ContractModerationEntry) {
    val tag = "contractModeration.entries.row.${entry.identityId}"
    Column(
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = Modifier.padding(vertical = 4.dp),
    ) {
        SelectionContainer {
            Text(
                entry.identityId,
                style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                maxLines = 1,
                overflow = TextOverflow.MiddleEllipsis,
                modifier = Modifier.testTag(tag),
            )
        }
        entry.untilMs?.let { ModerationFieldRow("Suspended until", moderationDate(it)) }
        val warnings = entry.warnings
        if (!warnings.isNullOrEmpty()) {
            ModerationWarningsView(warnings, tag)
        } else {
            ModerationReasonView(entry.reason)
        }
    }
}

/**
 * One identity's status on a contract's moderation lists. With no list chosen, rs-sdk reads
 * every list the contract keeps; a list chosen must be one the contract keeps, which the node
 * judges. Port of `ContractModerationStatusView.swift`.
 *
 * The inputs are taken at the tap, and an edit of the identity or the lists, or a newer look
 * up, drops the result shown and whatever a running read brings back.
 */
@Composable
internal fun ContractModerationStatusContent(sdk: Sdk, contractId: String, isCurrent: () -> Boolean) {
    val scope = rememberCoroutineScope()
    var identityId by rememberSaveable { mutableStateOf("") }
    var chosenLists by remember { mutableStateOf(emptySet<ContractModerationList>()) }
    var load by remember { mutableStateOf<ModerationLoad<ContractModerationStatus?>>(ModerationLoad.Idle) }
    var submittedIdentityId by remember { mutableStateOf<String?>(null) }
    // Bumped on every submission and edit, so the completion of a superseded one is dropped
    var submission by remember { mutableIntStateOf(0) }

    fun clearResult() {
        submission++
        submittedIdentityId = null
        load = ModerationLoad.Idle
    }

    val trimmed = identityId.trim()

    FormSection(title = "Identity") {
        OutlinedTextField(
            value = identityId,
            onValueChange = {
                identityId = it
                clearResult()
            },
            label = { Text("Identity ID (base58)") },
            singleLine = true,
            textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
            modifier = Modifier
                .fillMaxWidth()
                .testTag("contractModeration.status.identityId"),
        )
    }

    FormSection(title = "Lists") {
        ContractModerationList.entries.forEach { list ->
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(list.title, modifier = Modifier.weight(1f))
                Switch(
                    checked = list in chosenLists,
                    onCheckedChange = { isOn ->
                        chosenLists = if (isOn) chosenLists + list else chosenLists - list
                        clearResult()
                    },
                    modifier = Modifier.testTag("contractModeration.status.list.${list.jsonName}"),
                )
            }
        }
        HubFooter("None chosen reads every list the contract keeps.")
    }

    Button(
        onClick = {
            // The inputs are taken now, at the tap
            submission++
            val token = submission
            val lookUpId = trimmed
            // In the order the lists are offered; none chosen reads every list
            val lists = ContractModerationList.entries.filter { it in chosenLists }.ifEmpty { null }
            submittedIdentityId = lookUpId
            load = ModerationLoad.Loading
            scope.launch {
                val result = try {
                    ModerationLoad.Loaded(sdk.contractModeration.status(contractId, lookUpId, lists))
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    ModerationLoad.Failed(moderationErrorText(e))
                }
                if (token == submission && isCurrent()) load = result
            }
        },
        enabled = trimmed.isNotEmpty() && load !is ModerationLoad.Loading,
        modifier = Modifier
            .fillMaxWidth()
            .testTag("contractModeration.status.lookup"),
    ) { Text("Look Up") }

    if (load !is ModerationLoad.Idle) {
        FormSection(title = "Status") {
            submittedIdentityId?.let {
                ModerationFieldRow(
                    "Identity",
                    it,
                    monospaced = true,
                    modifier = Modifier.testTag("contractModeration.status.submittedIdentityId"),
                )
            }
            when (val current = load) {
                ModerationLoad.Idle -> Unit
                ModerationLoad.Loading -> ModerationMessageRow(
                    ModerationMessageKind.LOADING,
                    "Reading the lists…",
                    "contractModeration.status.loading",
                )
                is ModerationLoad.Failed -> ModerationMessageRow(
                    ModerationMessageKind.ERROR,
                    current.message,
                    "contractModeration.status.error",
                )
                is ModerationLoad.Loaded -> {
                    val status = current.value
                    if (status == null) {
                        ModerationMessageRow(
                            ModerationMessageKind.EMPTY,
                            "No answer for this identity.",
                            "contractModeration.status.empty",
                        )
                    } else {
                        ContractModerationStatusResult(status)
                    }
                }
            }
        }
    }
}

/** The status read: each list read, and what it says. A list not read is not shown as clear. */
@Composable
private fun ContractModerationStatusResult(status: ContractModerationStatus) {
    ModerationFieldRow(
        "Lists read",
        status.lists.joinToString(", ") { it.title },
        modifier = Modifier.testTag("contractModeration.status.result"),
    )
    status.banned?.let { banned ->
        Column(
            modifier = Modifier
                .semantics(mergeDescendants = true) {}
                .testTag("contractModeration.status.banned"),
        ) {
            ModerationFieldRow("Banned", if (banned) "Yes" else "No")
            status.banReason?.let { ModerationReasonView(it) }
        }
    }
    if (ContractModerationList.SUSPENSIONS in status.lists) {
        Column(
            modifier = Modifier
                .semantics(mergeDescendants = true) {}
                .testTag("contractModeration.status.suspendedUntil"),
        ) {
            ModerationFieldRow(
                "Suspended until",
                status.suspendedUntilMs?.let(::moderationDate) ?: "Not suspended",
            )
            status.suspensionReason?.let { ModerationReasonView(it) }
        }
    }
    status.warnings?.let { warnings ->
        ModerationFieldRow(
            "Warnings",
            warnings.size.toString(),
            modifier = Modifier.testTag("contractModeration.status.warnings"),
        )
        ModerationWarningsView(warnings, "contractModeration.status")
    }
}

/**
 * How many counted moderation actions each member of an elected contract's seated team
 * signed since the moderators pot was last claimed or settled. Port of
 * `ContractModerationActionCountsView.swift`.
 */
@Composable
internal fun ContractModerationActionCountsContent(sdk: Sdk, contractId: String, isCurrent: () -> Boolean) {
    val scope = rememberCoroutineScope()
    val loader = remember(contractId) { ModerationSingleLoader<ContractModerationActionCounts?>() }
    val reload: suspend () -> Unit = {
        loader.reload({ sdk.contractModeration.actionCounts(contractId) }, isCurrent)
    }
    LaunchedEffect(contractId) { reload() }

    FormSection(title = "Counts Since the Last Claim or Settlement") {
        when (val load = loader.load) {
            ModerationLoad.Idle, ModerationLoad.Loading -> ModerationMessageRow(
                ModerationMessageKind.LOADING,
                "Loading counts…",
                "contractModeration.actionCounts.loading",
            )
            is ModerationLoad.Failed -> ModerationMessageRow(
                ModerationMessageKind.ERROR,
                load.message,
                "contractModeration.actionCounts.error",
            )
            is ModerationLoad.Loaded -> {
                val counts = load.value?.counts.orEmpty()
                if (counts.isEmpty()) {
                    ModerationMessageRow(
                        ModerationMessageKind.EMPTY,
                        "No member acted since the last claim or settlement.",
                        "contractModeration.actionCounts.empty",
                    )
                }
                counts.forEach { count ->
                    ModerationFieldRow(
                        count.identityId,
                        count.count.toString(),
                        monospaced = true,
                        modifier = Modifier
                            .semantics(mergeDescendants = true) {}
                            .testTag("contractModeration.actionCounts.row.${count.identityId}"),
                    )
                }
            }
        }
        HubFooter(
            "Bans, suspensions, warnings and document deletions since the moderators pot was " +
                "last claimed or settled. A settlement (a membership change of the team) resets " +
                "the counts even when it pays nothing out. A member that did not act since has " +
                "no count.",
        )
    }
    TextButton(
        onClick = { scope.launch { reload() } },
        enabled = !loader.isLoading,
        modifier = Modifier.testTag("contractModeration.actionCounts.reload"),
    ) { Text("Reload") }
}
