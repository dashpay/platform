package org.dashfoundation.example.ui.tokens

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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.PendingGroupActions
import org.dashfoundation.example.navigation.TokenAction
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator
import org.dashfoundation.example.services.tokens.TokenActionResolver
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * "What can this identity do with this token, right now?" — port of
 * `TokenActionPermissionsView.swift`: acting-identity picker (or the
 * caller-pinned identity), paused banner, pending-group-actions entry
 * (only when a rule names a group), and the resolved action rows —
 * allowed rows navigate to the matching [TokenAction] form; denied rows
 * render dimmed with the denial reason (this doubles as the
 * control-rule viewer over the rules JSON columns).
 *
 * The live balance refresh (`calculateTokenId` +
 * `getIdentityTokenBalances`) and the on-chain pause-flag reconciliation
 * (`getTokenStatuses`) run against the acting identity once the SDK is
 * connected (matching `TokenActionPermissionsView.swift`); they fall back
 * to the persisted balance / `isPaused` columns when the SDK is offline or
 * the queries fail.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TokenActionPermissionsScreen(
    tokenIdHex: String,
    pinnedIdentityIdHex: String?,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val sdk by LocalAppState.current.sdk.collectAsStateWithLifecycle()
    val tokenId = remember(tokenIdHex) { tokenIdHex.hexToBytes() }

    val tokenFlow = remember(tokenIdHex) {
        container.database.tokenDao().observeTokenById(tokenId)
    }
    val token by tokenFlow.collectAsStateWithLifecycle(initialValue = null)

    val contractFlow = remember(token?.contractId?.toHex()) {
        token?.contractId?.let { container.database.dataContractDao().observeById(it) }
    }
    val contract by (contractFlow
        ?: remember { kotlinx.coroutines.flow.flowOf<DataContractEntity?>(null) })
        .collectAsStateWithLifecycle(initialValue = null)

    // Identity candidates: wallet-owned on the contract's network
    // (falls back to any wallet-owned when the contract isn't loaded —
    // same fallback as the Swift init).
    val networkRaw = contract?.networkRaw
    val identitiesFlow = remember(networkRaw) {
        val dao = container.database.identityDao()
        networkRaw?.let { dao.observeWalletOwnedByNetwork(it) } ?: dao.observeWalletOwned()
    }
    val identities by identitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())

    var pickedIdentityHex by rememberSaveable { mutableStateOf(pinnedIdentityIdHex) }
    val resolvedIdentity = identities.firstOrNull { it.identityId.toHex() == pickedIdentityHex }
        ?: identities.firstOrNull()

    // Live balance + on-chain pause reconciliation for the acting identity.
    // null balance = not yet fetched / unavailable; livePaused overrides the
    // persisted `isPaused` column when the status query succeeds. Mirrors
    // TokenActionPermissionsView.swift's `refreshLiveState`.
    var liveBalance by remember { mutableStateOf<ULong?>(null) }
    var livePaused by remember { mutableStateOf<Boolean?>(null) }
    LaunchedEffect(sdk, resolvedIdentity?.identityId?.toHex(), token?.contractId?.toHex(),
        token?.position) {
        liveBalance = null
        livePaused = null
        val activeSdk = sdk ?: return@LaunchedEffect
        val identity = resolvedIdentity ?: return@LaunchedEffect
        val currentToken = token ?: return@LaunchedEffect
        val canonicalId = runCatching {
            activeSdk.tokenQueries.calculateTokenId(
                Base58.encode(currentToken.contractId), currentToken.position,
            )
        }.getOrNull() ?: return@LaunchedEffect
        val identityBase58 = Base58.encode(identity.identityId)
        // Balances: {"<identityBase58>": <u64 balance>}
        runCatching {
            activeSdk.tokenQueries.identityTokenBalances(identityBase58, listOf(canonicalId))
        }.getOrNull()?.let { json ->
            liveBalance = runCatching {
                LenientJson.parseToJsonElement(json).jsonObject[canonicalId]
                    ?.jsonPrimitive?.content?.toULongOrNull()
            }.getOrNull()
        }
        // Statuses: {"<canonicalId>": {"paused": bool}}
        runCatching { activeSdk.tokenQueries.statuses(listOf(canonicalId)) }
            .getOrNull()?.let { json ->
                livePaused = runCatching {
                    LenientJson.parseToJsonElement(json).jsonObject[canonicalId]
                        ?.jsonObject?.get("paused")?.jsonPrimitive?.booleanOrNull
                }.getOrNull()
            }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Token Actions") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        val currentToken = token ?: return@Scaffold

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Acting as") {
                when {
                    identities.isEmpty() -> Text(
                        "No local identities on this network",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 8.dp),
                    )
                    pinnedIdentityIdHex != null && resolvedIdentity != null ->
                        androidx.compose.material3.ListItem(
                            headlineContent = { Text(resolvedIdentity.displayLabel()) },
                        )
                    else -> AccessiblePicker(
                        label = "Identity",
                        options = identities,
                        selected = resolvedIdentity ?: identities.first(),
                        optionLabel = { it.displayLabel() },
                        testTag = "tokenActions.identityPicker",
                        modifier = Modifier.padding(vertical = 8.dp),
                    ) { pickedIdentityHex = it.identityId.toHex() }
                }

                liveBalance?.let { bal ->
                    Text(
                        "Balance: " + TokenAmounts.format(bal, currentToken.decimals),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier
                            .padding(top = 8.dp)
                            .testTag("tokenActions.liveBalance"),
                    )
                }
            }

            // Reconcile against the on-chain status when available; otherwise
            // fall back to the persisted `isPaused` column.
            val isPaused = livePaused ?: currentToken.isPaused
            if (isPaused) {
                FormSection {
                    Text(
                        "This token is currently paused. Most actions are unavailable.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("tokenActions.pausedBanner"),
                    )
                }
            }

            val identity = resolvedIdentity
            if (identity != null &&
                GroupActionRuleEvaluator.relevantGroupPositions(currentToken).isNotEmpty()
            ) {
                FormSection {
                    ListItem(
                        headlineContent = { Text("Pending Group Actions") },
                        supportingContent = { Text("Review and co-sign open proposals") },
                        modifier = Modifier
                            .clickable {
                                navController.navigate(
                                    PendingGroupActions(
                                        tokenIdHex = tokenIdHex,
                                        identityIdHex = identity.identityId.toHex(),
                                    ),
                                )
                            }
                            .testTag("tokenActions.pendingGroupActions"),
                    )
                }
            }

            FormSection(title = "Actions") {
                if (identity == null) {
                    Text(
                        "Pick an identity to see available actions",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 8.dp),
                    )
                } else {
                    // Resolve against the live paused status when the status
                    // query succeeded — the persisted `isPaused` column lags a
                    // pause/resume broadcast, which would otherwise leave Resume
                    // locked ("Token is not paused") on a token that is in fact
                    // paused. The banner already uses `livePaused`; keep the row
                    // gates consistent with it.
                    val resolvedToken = remember(currentToken, livePaused) {
                        livePaused?.let { currentToken.copy(isPaused = it) } ?: currentToken
                    }
                    val rows = remember(resolvedToken, identity, contract) {
                        TokenActionResolver.resolve(resolvedToken, identity, contract)
                            .filter { !it.permission.isHidden }
                    }
                    rows.forEach { row ->
                        val allowed = row.permission.isAllowed
                        ListItem(
                            headlineContent = { Text(row.kind.title) },
                            supportingContent = {
                                Text(row.permission.deniedReason ?: row.kind.subtitle)
                            },
                            trailingContent = {
                                if (!allowed) {
                                    Text(
                                        "locked",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            },
                            modifier = Modifier
                                .alpha(if (allowed) 1f else 0.55f)
                                .clickable(enabled = allowed) {
                                    navController.navigate(
                                        TokenAction(
                                            actionId = row.kind.id,
                                            tokenIdHex = tokenIdHex,
                                            identityIdHex = identity.identityId.toHex(),
                                        ),
                                    )
                                }
                                .testTag("tokenActions.row.${row.kind.id}"),
                        )
                    }
                }
            }
        }
    }
}
