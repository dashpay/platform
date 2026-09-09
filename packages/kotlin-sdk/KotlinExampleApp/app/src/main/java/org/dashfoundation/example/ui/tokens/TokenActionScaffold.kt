package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Shared submit state for the 12 token-action forms — the Kotlin
 * counterpart of the `isSubmitting` / `submitError` / `submitGeneration`
 * trio each `TokenXxxActionView.swift` carries. Scoped to the action's
 * back-stack entry, so a pop + repush gets a fresh instance and a
 * late-resolving submit from the old instance can't write into the new
 * one (the role of the Swift generation counter).
 */
class TokenActionViewModel : ViewModel() {

    var isSubmitting by mutableStateOf(false)
        private set

    var error by mutableStateOf<String?>(null)

    /** Flips once on success; the scaffold observes it and pops back. */
    var completed by mutableStateOf(false)
        private set

    /** Run [block] once; concurrent submits are ignored while in flight. */
    fun submit(block: suspend () -> Unit) {
        if (isSubmitting || completed) return
        isSubmitting = true
        viewModelScope.launch {
            try {
                block()
                completed = true
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            } finally {
                isSubmitting = false
            }
        }
    }
}

/**
 * Everything a token action form needs to build its `Tokens.<action>`
 * call — the Android mapping of how the Swift forms resolve their
 * dependencies:
 *
 *  - iOS: `identity.wallet?.walletId` → `walletManager.wallet(for:)`;
 *    Android: `identity.walletId` hex → `manager.wallets` map.
 *  - iOS: a per-submit `KeychainSigner(modelContainer:)`;
 *    Android: the manager-owned `KeystoreSigner` via
 *    [PlatformWalletManager.signerHandle].
 *  - Signing key: iOS passes the default `signingKeyId: 0` — the FFI
 *    ignores it and Rust selects the first AUTHENTICATION /
 *    MASTER-or-HIGH ECDSA key (TokenActions.swift:75-79). Android does
 *    the same via [SIGNING_KEY_ID].
 */
data class TokenActionContext(
    val token: TokenEntity,
    val identity: IdentityEntity,
    val contract: DataContractEntity?,
    val manager: PlatformWalletManager?,
    val wallet: ManagedPlatformWallet?,
) {
    val signerHandle: Long? get() = manager?.signerHandle

    companion object {
        /** Rust-side key selection; mirrors the ignored iOS default. */
        const val SIGNING_KEY_ID: Int = 0
    }
}

/**
 * Load the `(token, identity, contract, wallet)` tuple behind a token
 * action route. Null while loading; [TokenActionContext.wallet] stays
 * null when the owning wallet isn't loaded (the forms surface that the
 * same way the Swift views do).
 */
@Composable
fun rememberTokenActionContext(
    tokenIdHex: String,
    identityIdHex: String,
): TokenActionContext? {
    val container = LocalAppContainer.current
    val tokenId = remember(tokenIdHex) { tokenIdHex.hexToBytes() }
    val identityId = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val tokenFlow = remember(tokenIdHex) {
        container.database.tokenDao().observeTokenById(tokenId)
    }
    val token by tokenFlow.collectAsStateWithLifecycle(initialValue = null)

    val identityFlow = remember(identityIdHex) {
        container.database.identityDao().observeByIdentityId(identityId)
    }
    val identity by identityFlow.collectAsStateWithLifecycle(initialValue = null)

    val contractIdHex = token?.contractId?.toHex()
    val contractFlow = remember(contractIdHex) {
        val id = token?.contractId
        if (id == null) null else container.database.dataContractDao().observeById(id)
    }
    val contract by (contractFlow
        ?: remember { kotlinx.coroutines.flow.flowOf<DataContractEntity?>(null) })
        .collectAsStateWithLifecycle(initialValue = null)

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by (manager?.wallets
        ?: remember {
            kotlinx.coroutines.flow.MutableStateFlow(
                emptyMap<String, ManagedPlatformWallet>(),
            )
        })
        .collectAsStateWithLifecycle()

    val currentToken = token ?: return null
    val currentIdentity = identity ?: return null
    val wallet = currentIdentity.walletId?.toHex()?.let { walletsMap[it] }
    return TokenActionContext(
        token = currentToken,
        identity = currentIdentity,
        contract = contract,
        manager = manager,
        wallet = wallet,
    )
}

/**
 * Shared frame for the 12 token-action forms — port of the recurring
 * `Form { Token / [group banner] / …sections… / submit }` shape of
 * the `TokenActions` Swift views: token summary header, group-action banner
 * (from [GroupActionRuleEvaluator]), caller-declared sections, submit
 * button with in-flight spinner, error alert, and pop-on-success.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TokenActionScaffold(
    title: String,
    context: TokenActionContext,
    banner: GroupActionRuleEvaluator.BannerState,
    bannerActionLabel: String,
    submitLabel: String,
    canSubmit: Boolean,
    viewModel: TokenActionViewModel,
    navController: NavHostController,
    onSubmit: () -> Unit,
    tokenSummaryRows: @Composable () -> Unit = {},
    content: @Composable () -> Unit,
) {
    LaunchedEffect(viewModel.completed) {
        if (viewModel.completed) navController.popBackStack()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
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
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Token") {
                LabeledContent("Token", context.token.name)
                tokenSummaryRows()
            }

            if (banner is GroupActionRuleEvaluator.BannerState.Propose) {
                GroupActionBanner(banner.groupPosition, bannerActionLabel)
            }

            content()

            SubmitButton(
                text = submitLabel,
                isLoading = viewModel.isSubmitting,
                enabled = canSubmit && context.wallet != null,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("tokenAction.submitButton"),
                onClick = onSubmit,
            )

            if (context.wallet == null) {
                Text(
                    "The wallet that owns this identity isn't loaded.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        }
    }

    ErrorAlertDialog(
        message = viewModel.error,
        title = "$title failed",
        onDismiss = { viewModel.error = null },
    )
}

/**
 * "This action requires a group action" banner — port of the section at
 * `TokenMintActionView.swift:61-76`.
 */
@Composable
fun GroupActionBanner(groupPosition: Int, actionLabel: String) {
    FormSection(title = "Group action") {
        Text(
            "This $actionLabel requires a group action.",
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.testTag("tokenAction.groupBanner"),
        )
        LabeledContent("Group position", "$groupPosition")
        Text(
            "Submitting will propose a new group action. Other group members " +
                "must sign before it broadcasts.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

/** Shared optional public-note field (every action form's last input). */
@Composable
fun PublicNoteField(
    value: String,
    onValueChange: (String) -> Unit,
) {
    FormSection(title = "Public note (optional)") {
        OutlinedTextField(
            value = value,
            onValueChange = onValueChange,
            label = { Text("Note") },
            minLines = 1,
            maxLines = 3,
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp)
                .testTag("tokenAction.noteField"),
        )
    }
}

/** Trimmed note → null when blank (mirror of the Swift `publicNoteOrNil`). */
fun String.toPublicNoteOrNull(): String? = trim().takeIf { it.isNotEmpty() }
