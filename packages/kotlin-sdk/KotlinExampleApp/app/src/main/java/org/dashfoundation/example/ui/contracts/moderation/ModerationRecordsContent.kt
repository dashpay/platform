package org.dashfoundation.example.ui.contracts.moderation

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.moderation.ContractDocumentRemovalEntry
import org.dashfoundation.dashsdk.moderation.ContractTeamActionEntry
import org.dashfoundation.dashsdk.moderation.ContractTeamActionEvent
import org.dashfoundation.dashsdk.moderation.ContractTeamActionSigners
import org.dashfoundation.dashsdk.moderation.ContractTeamActionStatus
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection

/**
 * The removal records filter (← `RemovalsLookupFilter`): the ids looked up (null while every
 * record is read a page at a time) and the token of the read last started, so a read queued
 * before a newer one, or before an edit of the ids it looks up, is not shown.
 */
class RemovalsLookupFilter {
    var ids by mutableStateOf<List<String>?>(null)
        private set
    private var token = 0

    /** Starts a read of [ids]: shows them and returns the read's token. */
    fun begin(ids: List<String>?): Int {
        this.ids = ids
        return ++token
    }

    /**
     * The typed ids changed. A look up by ids no longer answers them: it is dropped (`true`,
     * the rows are to be cleared). Reading every record does not depend on them (`false`).
     */
    fun draftChanged(): Boolean {
        if (ids == null) return false
        ids = null
        token++
        return true
    }

    /** Whether the read of [token] is still the one to show. */
    fun isCurrent(token: Int): Boolean = token == this.token
}

/** The comma-separated ids typed, trimmed, empty ones dropped (← `parsedIds`). */
fun parseDocumentIds(input: String): List<String> =
    input.split(',').map { it.trim() }.filter { it.isNotEmpty() }

/**
 * The removal records one document type of a contract keeps: a page at a time, or the
 * records of the document ids typed in. Whether the type keeps records is the node's to
 * answer. Port of `ContractDocumentRemovalsView.swift`; the Reload button stands in for its
 * pull to refresh. The kept values come decoded by rs-sdk (one budget per page); this screen
 * only shows them.
 */
@Composable
internal fun ContractDocumentRemovalsContent(
    sdk: Sdk,
    contractId: String,
    documentTypeName: String,
    isCurrent: () -> Boolean,
) {
    val scope = rememberCoroutineScope()
    var documentIdsInput by rememberSaveable(documentTypeName) { mutableStateOf("") }
    val filter = remember(documentTypeName) { RemovalsLookupFilter() }
    val loader = remember(documentTypeName) {
        ModerationPagedLoader<ContractDocumentRemovalEntry, String>()
    }
    val parsedIds = parseDocumentIds(documentIdsInput)

    // Starts a read of `ids` (null for every record, a page at a time): the filter shown
    // changes, its token advances, the loader drops what is shown, then the read runs
    fun submit(ids: List<String>?) {
        val token = filter.begin(ids)
        loader.begin()
        scope.launch {
            if (!filter.isCurrent(token)) return@launch
            val fetch: ModerationPagedLoader.Fetch<ContractDocumentRemovalEntry, String> = if (ids == null) {
                ModerationPagedLoader.Fetch { startAfter ->
                    sdk.contractModeration.documentRemovals(
                        contractId,
                        documentTypeName,
                        startAfter,
                        MODERATION_PAGE_SIZE,
                    )?.let { ModerationPage(it.removals, it.nextStartAfter) }
                }
            } else {
                ModerationPagedLoader.Fetch { _ ->
                    sdk.contractModeration.documentRemovals(contractId, documentTypeName, ids)
                        ?.let { ModerationPage(it.removals, it.nextStartAfter) }
                }
            }
            loader.reload(fetch, isCurrent)
        }
    }

    LaunchedEffect(documentTypeName) { submit(null) }

    val lookedUpIds = filter.ids

    FormSection(title = "Look Up") {
        OutlinedTextField(
            value = documentIdsInput,
            onValueChange = {
                documentIdsInput = it
                if (filter.draftChanged()) loader.clear()
            },
            label = { Text("Document IDs, comma-separated") },
            textStyle = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace),
            modifier = Modifier
                .fillMaxWidth()
                .testTag("contractModeration.removals.documentIds"),
        )
        TextButton(
            onClick = { submit(parsedIds) },
            enabled = parsedIds.isNotEmpty(),
            modifier = Modifier.testTag("contractModeration.removals.lookup"),
        ) { Text("Look Up These Documents") }
        if (lookedUpIds != null || !loader.loadedOnce) {
            TextButton(
                onClick = { submit(null) },
                modifier = Modifier.testTag("contractModeration.removals.showAll"),
            ) { Text("Show Every Record") }
        }
        HubFooter("A document with no record is left out of the answer.")
    }

    FormSection(title = if (lookedUpIds == null) "Records" else "Records of the Documents Named") {
        lookedUpIds?.let {
            ModerationFieldRow(
                "Looked up",
                it.joinToString(", "),
                monospaced = true,
                modifier = Modifier.testTag("contractModeration.removals.submittedIds"),
            )
        }
        loader.errorMessage?.let {
            ModerationMessageRow(ModerationMessageKind.ERROR, it, "contractModeration.removals.error")
        }
        if (!loader.loadedOnce && !loader.isLoading && loader.errorMessage == null) {
            ModerationMessageRow(
                ModerationMessageKind.EMPTY,
                "Look up the ids typed above, or show every record.",
                "contractModeration.removals.idle",
            )
        }
        if (loader.items.isEmpty()) {
            if (loader.isLoading) {
                ModerationMessageRow(
                    ModerationMessageKind.LOADING,
                    "Loading records…",
                    "contractModeration.removals.loading",
                )
            } else if (loader.loadedOnce && loader.errorMessage == null) {
                ModerationMessageRow(
                    ModerationMessageKind.EMPTY,
                    "No records.",
                    "contractModeration.removals.empty",
                )
            }
        }
        loader.items.forEachIndexed { index, removal ->
            if (index > 0) HorizontalDivider()
            ContractDocumentRemovalRow(removal)
        }
        if (loader.nextCursor != null && loader.items.isNotEmpty()) {
            ModerationLoadMoreButton(loader.isLoading, "contractModeration.removals.loadMore") {
                scope.launch { loader.loadMore() }
            }
        }
        TextButton(
            onClick = { submit(lookedUpIds) },
            enabled = !loader.isLoading,
            modifier = Modifier.testTag("contractModeration.removals.reload"),
        ) { Text("Reload") }
    }
}

/** One removal record: who removed what, why and when, whether restored, and the kept values. */
@Composable
private fun ContractDocumentRemovalRow(removal: ContractDocumentRemovalEntry) {
    Column(
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = Modifier
            .padding(vertical = 6.dp)
            .semantics(mergeDescendants = true) {}
            .testTag("contractModeration.removals.row.${removal.documentId}"),
    ) {
        Text(
            removal.documentId,
            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
            maxLines = 1,
            overflow = TextOverflow.MiddleEllipsis,
        )
        ModerationFieldRow("Owner", removal.documentOwnerId, monospaced = true)
        ModerationFieldRow("Removed by", removal.moderatorId, monospaced = true)
        ModerationFieldRow("Removed", moderationDate(removal.removedAtMs))
        ModerationReasonView(removal.reason)
        removal.restoredBy?.let { ModerationFieldRow("Restored by", it, monospaced = true) }
        removal.restoredAtMs?.let { ModerationFieldRow("Restored", moderationDate(it)) }
        removal.keptFields.keys.sorted().forEach { path ->
            // The path above its value, the value wrapped in full so every member of a long
            // object or array can be read and selected
            Column {
                Text(
                    path,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                SelectionContainer {
                    Text(
                        removal.keptFields.getValue(path).displayText,
                        style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    )
                }
            }
        }
        if (removal.keptFields.isNotEmpty()) {
            Text(
                "Field names follow the contract's current layout.",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        ModerationFieldRow("Hash", removal.documentHash, monospaced = true)
    }
}

/** Where the next page of team actions starts, as the last page returned it. */
data class ContractTeamActionsCursor(val actionId: String, val included: Boolean)

/**
 * The actions an elected contract's seated moderation team votes on, active or closed, a
 * page at a time in action id order; each drills into [ContractTeamActionDetailContent].
 * Port of `ContractTeamActionsView.swift`; the Reload button stands in for its pull to
 * refresh.
 */
@Composable
internal fun ContractTeamActionsContent(
    sdk: Sdk,
    contractId: String,
    isCurrent: () -> Boolean,
    push: (ModerationDestination) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var status by rememberSaveable { mutableStateOf(ContractTeamActionStatus.ACTIVE) }
    // The status the rows shown were listed under, which their signers are read with
    var listedStatus by remember { mutableStateOf(ContractTeamActionStatus.ACTIVE) }
    val loader = remember { ModerationPagedLoader<ContractTeamActionEntry, ContractTeamActionsCursor>() }
    var readToken by remember { mutableStateOf(0) }

    fun read(selected: ContractTeamActionStatus) {
        val token = ++readToken
        listedStatus = selected
        loader.begin()
        scope.launch {
            if (token != readToken) return@launch
            loader.reload(
                { cursor ->
                    // The cursor goes back as the last page returned it
                    sdk.contractModeration.teamActions(
                        contractId,
                        selected,
                        startAtActionId = cursor?.actionId,
                        startAtActionIdIncluded = cursor?.included ?: false,
                        limit = MODERATION_PAGE_SIZE,
                    )?.let { page ->
                        ModerationPage(
                            page.actions,
                            page.nextStartAtActionId?.let {
                                ContractTeamActionsCursor(it, page.nextStartAtActionIdIncluded ?: false)
                            },
                        )
                    }
                },
                isCurrent,
            )
        }
    }

    LaunchedEffect(Unit) { read(status) }

    FormSection {
        AccessiblePicker(
            label = "Status",
            options = ContractTeamActionStatus.entries,
            selected = status,
            optionLabel = { if (it == ContractTeamActionStatus.ACTIVE) "Active" else "Closed" },
            testTag = "contractModeration.teamActions.status",
            onSelected = {
                if (it != status) {
                    status = it
                    read(it)
                }
            },
        )
        HubFooter(
            if (status == ContractTeamActionStatus.ACTIVE) {
                "Actions still gathering approvals."
            } else {
                "Actions whose approvals met the rule and ran."
            },
        )
    }

    loader.errorMessage?.let {
        FormSection {
            ModerationMessageRow(ModerationMessageKind.ERROR, it, "contractModeration.teamActions.error")
        }
    }

    FormSection(title = "Actions") {
        if (loader.items.isEmpty()) {
            if (loader.isLoading) {
                ModerationMessageRow(
                    ModerationMessageKind.LOADING,
                    "Loading actions…",
                    "contractModeration.teamActions.loading",
                )
            } else if (loader.loadedOnce && loader.errorMessage == null) {
                ModerationMessageRow(
                    ModerationMessageKind.EMPTY,
                    "No actions.",
                    "contractModeration.teamActions.empty",
                )
            }
        }
        loader.items.forEach { action ->
            ContractTeamActionRow(
                action,
                Modifier
                    .clickable { push(ModerationDestination.TeamAction(action, listedStatus)) }
                    .testTag("contractModeration.teamActions.row.${action.actionId}"),
            )
        }
        if (loader.nextCursor != null && loader.items.isNotEmpty()) {
            ModerationLoadMoreButton(loader.isLoading, "contractModeration.teamActions.loadMore") {
                scope.launch { loader.loadMore() }
            }
        }
        TextButton(
            onClick = { read(status) },
            enabled = !loader.isLoading,
            modifier = Modifier.testTag("contractModeration.teamActions.reload"),
        ) { Text("Reload") }
    }
}

/** One team action in the listing. */
@Composable
private fun ContractTeamActionRow(action: ContractTeamActionEntry, modifier: Modifier) {
    Column(
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp),
    ) {
        when (val event = action.event) {
            is ContractTeamActionEvent.DeleteSettledDocument -> {
                Text("Delete ${event.documentTypeName} document", style = MaterialTheme.typography.titleSmall)
                Text(
                    event.documentId,
                    style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.MiddleEllipsis,
                )
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(
                "${action.approvalCount} approvals",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                moderationDate(action.proposedAtMs),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/**
 * One team action: who proposed it, what it does, and who approved it. Port of
 * `ContractTeamActionDetailView.swift`. The signers are read by the status the action was
 * listed under.
 */
@Composable
internal fun ContractTeamActionDetailContent(
    sdk: Sdk,
    contractId: String,
    action: ContractTeamActionEntry,
    status: ContractTeamActionStatus,
    isCurrent: () -> Boolean,
) {
    val scope = rememberCoroutineScope()
    val loader = remember(action.actionId) { ModerationSingleLoader<ContractTeamActionSigners?>() }
    val reload: suspend () -> Unit = {
        loader.reload(
            { sdk.contractModeration.teamActionSigners(contractId, status, action.actionId) },
            isCurrent,
        )
    }
    LaunchedEffect(action.actionId) { reload() }

    FormSection(title = "Action") {
        ModerationFieldRow("Action ID", action.actionId, monospaced = true)
        ModerationFieldRow(
            "Status when listed",
            if (status == ContractTeamActionStatus.ACTIVE) "Active" else "Closed",
        )
        ModerationFieldRow("Proposer", action.proposerId, monospaced = true)
        ModerationFieldRow("Proposed", moderationDate(action.proposedAtMs))
        ModerationFieldRow("Approvals when listed", action.approvalCount.toString())
        HubFooter(
            "The status and approval count are a snapshot from the listing, taken when the " +
                "action was listed.",
        )
    }

    FormSection(title = "Event") {
        Column(
            verticalArrangement = Arrangement.spacedBy(4.dp),
            modifier = Modifier
                .semantics(mergeDescendants = true) {}
                .testTag("contractModeration.teamAction.event"),
        ) {
            when (val event = action.event) {
                is ContractTeamActionEvent.DeleteSettledDocument -> {
                    Text("Delete a settled document", style = MaterialTheme.typography.titleSmall)
                    ModerationFieldRow("Document type", event.documentTypeName)
                    ModerationFieldRow("Document", event.documentId, monospaced = true)
                    ModerationFieldRow("Last modified", moderationDate(event.documentLastModifiedAtMs))
                    event.documentRevision?.let { ModerationFieldRow("Revision", it.toString()) }
                    ModerationReasonView(event.reason)
                }
            }
        }
    }

    FormSection(title = "Signers") {
        when (val load = loader.load) {
            ModerationLoad.Idle, ModerationLoad.Loading -> ModerationMessageRow(
                ModerationMessageKind.LOADING,
                "Loading signers…",
                "contractModeration.teamAction.signers.loading",
            )
            is ModerationLoad.Failed -> ModerationMessageRow(
                ModerationMessageKind.ERROR,
                load.message,
                "contractModeration.teamAction.signers.error",
            )
            is ModerationLoad.Loaded -> {
                val signerIds = load.value?.signerIds.orEmpty()
                if (signerIds.isEmpty()) {
                    ModerationMessageRow(
                        ModerationMessageKind.EMPTY,
                        "No signers: the contract holds no such action with this status.",
                        "contractModeration.teamAction.signers.empty",
                    )
                }
                signerIds.forEach { signerId ->
                    SelectionContainer {
                        Text(
                            signerId,
                            style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                            maxLines = 1,
                            overflow = TextOverflow.MiddleEllipsis,
                            modifier = Modifier
                                .padding(vertical = 2.dp)
                                .testTag("contractModeration.teamAction.signers.row.$signerId"),
                        )
                    }
                }
            }
        }
        OutlinedButton(
            onClick = { scope.launch { reload() } },
            enabled = !loader.isLoading,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("contractModeration.teamAction.signers.reload"),
        ) { Text("Reload Signers") }
        HubFooter(
            "The signers are read separately, by the status the action was listed under, " +
                "possibly at a later block than the listing: an approval since then can show " +
                "more signers than the count, and an action that has closed since shows no " +
                "active signers. An active action's count is also an upper bound, as a member " +
                "who left the team keeps their approval in it until a later approval drops it.",
        )
    }
}
