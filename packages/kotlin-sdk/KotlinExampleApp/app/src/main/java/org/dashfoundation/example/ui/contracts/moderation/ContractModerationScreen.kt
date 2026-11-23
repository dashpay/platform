package org.dashfoundation.example.ui.contracts.moderation

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import java.lang.ref.WeakReference
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.moderation.ContractModerationList
import org.dashfoundation.dashsdk.moderation.ContractTeamActionEntry
import org.dashfoundation.dashsdk.moderation.ContractTeamActionStatus
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.contracts.ParsedContract
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes

/** Where the moderation hub drills down to (← `ContractModerationRoute`). */
sealed class ModerationDestination(val title: String) {
    data class Entries(val list: ContractModerationList) : ModerationDestination(list.title)
    data object Status : ModerationDestination("Identity Status")
    data class Removals(val documentTypeName: String) : ModerationDestination(documentTypeName)
    data object TeamActions : ModerationDestination("Team Actions")

    /** An action and the status it was listed under, which its signers are read with. */
    data class TeamAction(
        val action: ContractTeamActionEntry,
        val status: ContractTeamActionStatus,
    ) : ModerationDestination("Team Action")
    data object ActionCounts : ModerationDestination("Action Counts")
}

/**
 * The moderation hub's state, held by its back-stack entry so a tab switch keeps the screen
 * the user drilled into: the destinations pushed on top of the hub, and the SDK the hub was
 * opened on (← `ModerationScope`).
 *
 * The hub and every screen under it read and show only while that SDK is still the app's. A
 * network switch or a devnet edit replaces the SDK, and the same contract id can exist on
 * both chains, so neither the network nor the id says an answer is still the same chain's.
 * The SDK is held weakly: the hub compares it, it never keeps it alive.
 */
class ContractModerationViewModel : ViewModel() {
    /** The destinations above the hub, the last on top. */
    val stack = mutableStateListOf<ModerationDestination>()

    private var openedOn: WeakReference<Sdk>? = null

    /**
     * Capture the SDK the hub is opened on; later calls keep the first. Without an SDK yet
     * (it is still being built) nothing is captured, so the SDK that arrives is the one the
     * hub opens on.
     */
    fun open(sdk: Sdk?) {
        if (openedOn == null && sdk != null) openedOn = WeakReference(sdk)
    }

    /** Whether [current] is still the SDK the hub was opened on. */
    fun isOpen(current: Sdk?): Boolean = current != null && openedOn?.get() === current

    fun push(destination: ModerationDestination) {
        stack.add(destination)
    }

    fun pop() {
        if (stack.isNotEmpty()) stack.removeAt(stack.lastIndex)
    }
}

/**
 * Contract moderation and fee pots (protocol version 14) for one contract: port of
 * `ContractModerationView.swift` (SwiftExampleApp/Views/ContractModeration). Its fee pots,
 * its banlist, suspension list and warning list, one identity's status, the removal records
 * of its document types, its team actions and the moderation action counts, each read
 * through `Sdk.contractModeration`.
 *
 * Every choice is offered whatever the contract declares: which lists it keeps, which types
 * keep removal records and whether it is elected are the node's to answer, and its refusal
 * is shown on the screen that asked. The drill-downs are pushed inside this destination
 * ([ContractModerationViewModel.stack]); system Back pops them first.
 *
 * @param documentTypeNames the contract's document types, offered for removal records.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContractModerationScreen(
    contractId: String,
    documentTypeNames: List<String>,
    navController: NavHostController,
    state: ContractModerationViewModel = viewModel(),
) {
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    state.open(appState.sdk.value)
    val isOpen = state.isOpen(sdk)
    val top = state.stack.lastOrNull()

    BackHandler(enabled = state.stack.isNotEmpty()) { state.pop() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(top?.title ?: "Moderation") },
                navigationIcon = {
                    IconButton(onClick = {
                        if (state.stack.isNotEmpty()) state.pop() else navController.popBackStack()
                    }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
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
            val current = sdk
            if (!isOpen || current == null) {
                // A replaced SDK: no row read through the old one stays on screen, and
                // nothing reads this id through the new one
                FormSection {
                    ModerationMessageRow(
                        ModerationMessageKind.ERROR,
                        "The connection changed since this contract was opened. Reopen the contract.",
                        "contractModeration.connectionChanged",
                    )
                }
                return@Column
            }
            val isCurrent = { state.isOpen(appState.sdk.value) }
            when (top) {
                null -> ModerationHub(current, contractId, documentTypeNames, isCurrent, state::push)
                is ModerationDestination.Entries ->
                    ContractModerationEntriesContent(current, contractId, top.list, isCurrent)
                ModerationDestination.Status ->
                    ContractModerationStatusContent(current, contractId, isCurrent)
                is ModerationDestination.Removals ->
                    ContractDocumentRemovalsContent(current, contractId, top.documentTypeName, isCurrent)
                ModerationDestination.TeamActions ->
                    ContractTeamActionsContent(current, contractId, isCurrent, state::push)
                is ModerationDestination.TeamAction ->
                    ContractTeamActionDetailContent(current, contractId, top.action, top.status, isCurrent)
                ModerationDestination.ActionCounts ->
                    ContractModerationActionCountsContent(current, contractId, isCurrent)
            }
        }
    }
}

/** The hub itself: the fee pots and a link to every read. */
@Composable
private fun ModerationHub(
    sdk: Sdk,
    contractId: String,
    documentTypeNames: List<String>,
    isCurrent: () -> Boolean,
    push: (ModerationDestination) -> Unit,
) {
    ContractFeePotsSection(sdk, contractId, isCurrent)

    FormSection(title = "Moderation Lists") {
        ContractModerationList.entries.forEach { list ->
            HubLink(list.title, "contractModeration.listLink.${list.jsonName}") {
                push(ModerationDestination.Entries(list))
            }
        }
        HubLink("Look Up an Identity", "contractModeration.statusLink") {
            push(ModerationDestination.Status)
        }
        HubFooter("A list the contract does not keep is refused by the node.")
    }

    FormSection(title = "Removal Records") {
        if (documentTypeNames.isEmpty()) {
            ModerationMessageRow(
                ModerationMessageKind.EMPTY,
                "This contract has no document types.",
                "contractModeration.removalsEmpty",
            )
        }
        documentTypeNames.forEach { name ->
            HubLink(name, "contractModeration.removalsLink.$name") {
                push(ModerationDestination.Removals(name))
            }
        }
        HubFooter(
            "Records of documents a moderator deleted. A type that keeps none is refused by the node.",
        )
    }

    FormSection(title = "Moderation Team") {
        HubLink("Team Actions", "contractModeration.teamActionsLink") {
            push(ModerationDestination.TeamActions)
        }
        HubLink("Moderation Action Counts", "contractModeration.actionCountsLink") {
            push(ModerationDestination.ActionCounts)
        }
        HubFooter(
            "Only an elected contract has a seated team; the node refuses any other. Team " +
                "actions are kept only when at least one document type sets " +
                "moderatorAbilities.deleteSettled: otherwise the node refuses them, which " +
                "differs from an empty list.",
        )
    }
}

@Composable
private fun HubLink(title: String, testTag: String, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        modifier = Modifier
            .clickable(onClick = onClick)
            .testTag(testTag),
    )
}

@Composable
internal fun HubFooter(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(vertical = 4.dp),
    )
}

/**
 * The moderation hub's navigation entry: reads the saved contract by its id and opens
 * [ContractModerationScreen] with its base58 id and its document types, sorted by name, as
 * `DataContractDetailsView.swift` builds its `ModerationRoute`.
 */
@Composable
fun ContractModerationEntryScreen(contractIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val contractFlow = remember(contractIdHex) {
        container.database.dataContractDao().observeById(contractIdHex.hexToBytes())
    }
    val contract by contractFlow.collectAsStateWithLifecycle(initialValue = null)
    val current = contract ?: return
    val documentTypeNames = remember(current.id, current.lastUpdated) {
        ParsedContract.from(current)?.documentTypes?.keys.orEmpty().sorted()
    }
    ContractModerationScreen(
        contractId = Base58.encode(current.id),
        documentTypeNames = documentTypeNames,
        navController = navController,
    )
}
