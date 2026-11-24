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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.PendingGroupActions
import org.dashfoundation.example.navigation.TokenAction
import org.dashfoundation.example.services.tokens.GroupActionRuleEvaluator
import org.dashfoundation.example.services.tokens.IdentityTokenSync
import org.dashfoundation.example.services.tokens.TokenActionKind
import org.dashfoundation.example.services.tokens.TokenActionResolver
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
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
 * the queries fail. Platform's paused status is also written to the
 * `isPaused` column, as the Swift view's `refreshTokenStatus` does.
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
    // Every emission, not only ones unequal by id: a rewritten row (paused
    // flag, rules) must replace the one on screen.
    val token by tokenFlow.collectEachAsStateWithLifecycle(initialValue = null)

    val contractFlow = remember(token?.contractId?.toHex()) {
        token?.contractId?.let { container.database.dataContractDao().observeById(it) }
    }
    val contract by (contractFlow
        ?: remember { kotlinx.coroutines.flow.flowOf<DataContractEntity?>(null) })
        .collectEachAsStateWithLifecycle(initialValue = null)

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

    // Watch the displayed token for the acting identity in the Rust
    // identity-token sync, so Transfer and Burn (which read
    // `token_balances`) see its balance even when this screen was reached
    // without the Tokens screen (Contracts > Token Search). Only this token
    // is watched, so the request carries no more than the live read below;
    // the other wallet identities are registered with none. The job is
    // launched as soon as the token is known and resolves everything else
    // from Room itself, so leaving the screen early (opening Transfer) does
    // not skip it: it reads the token's contract, takes the picked identity
    // (never a fallback) or else the first wallet identity on the contract's
    // network, and runs only when the contract, that identity and the active
    // manager are on the same network.
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    // Also re-run when the sender the screen resolves appears or changes
    // (identity discovery can land after the first Room read below).
    LaunchedEffect(
        manager,
        sdk,
        pickedIdentityHex,
        token?.id?.toHex(),
        resolvedIdentity?.identityId?.toHex(),
        identities.joinToString(",") { it.identityId.toHex() },
    ) {
        val activeManager = manager ?: return@LaunchedEffect
        val activeSdk = sdk ?: return@LaunchedEffect
        // During a network switch the new SDK is published before its
        // manager: never pair them across networks.
        if (activeManager.network != activeSdk.network) return@LaunchedEffect
        val displayed = token ?: return@LaunchedEffect
        val picked = pickedIdentityHex
        val database = container.database
        // Application scope: leaving the screen must not cancel the pass.
        container.applicationScope.launch {
            val contractNetwork = database.dataContractDao().getById(displayed.contractId)
                ?.networkRaw ?: return@launch
            if (activeManager.network.ffiValue != contractNetwork) return@launch
            val candidates = database.identityDao()
                .observeWalletOwnedByNetwork(contractNetwork).first()
            val identity = if (picked != null) {
                candidates.firstOrNull { it.identityId.toHex() == picked }
            } else {
                candidates.firstOrNull()
            } ?: return@launch
            if (identity.networkRaw != contractNetwork) return@launch
            IdentityTokenSync.refresh(
                identityId = identity.identityId,
                otherIdentityIds = candidates.map { it.identityId },
                tokens = listOf(displayed),
                calculateTokenId = activeSdk.tokenQueries::calculateTokenId,
                register = activeManager::registerIdentityForTokenSync,
                syncNow = activeManager::syncIdentityTokensNow,
                isStillStored = {
                    database.identityDao().getByIdentityId(identity.identityId) != null
                },
                isTokenStillSaved = { database.tokenDao().getTokenById(it.id) != null },
            )
        }
    }

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
        // Store Platform's answer, as `refreshTokenStatus` does in
        // TokenActionPermissionsView.swift, so the token list, the token
        // details and the Pause / Resume forms (which read the Room row)
        // agree with this screen. A missing or malformed entry keeps the
        // stored flag. Leaving the screen cancels this effect before the
        // write, so a read that started before a pause cannot overwrite the
        // pause form's own write.
        livePaused?.let { paused ->
            try {
                container.database.tokenDao().updatePaused(currentToken.id, paused)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // The screen still shows `livePaused`; the next open retries.
            }
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
                    val claimStore = container.oncePerIdentityClaimStore
                    val oncePerIdentityClaimed by remember(
                        currentToken.id.toHex(), identity.identityId.toHex(),
                    ) {
                        claimStore.observe(
                            identity.networkRaw, currentToken.id, identity.identityId,
                        )
                    }.collectAsStateWithLifecycle(initialValue = false)
                    TokenActionRows(
                        token = currentToken,
                        livePaused = livePaused,
                        identity = identity,
                        contract = contract,
                        oncePerIdentityClaimed = oncePerIdentityClaimed,
                    ) { kind ->
                        navController.navigate(
                            TokenAction(
                                actionId = kind.id,
                                tokenIdHex = tokenIdHex,
                                identityIdHex = identity.identityId.toHex(),
                            ),
                        )
                    }
                }
            }
        }
    }
}

/**
 * The action rows for [identity] on [token], resolved on every
 * recomposition. [livePaused], Platform's answer to the token status query,
 * overrides the stored `isPaused` column when present, so the Pause and
 * Resume gates agree with the paused banner.
 *
 * The rows are deliberately not cached with `remember`: [TokenEntity],
 * [IdentityEntity] and [DataContractEntity] compare equal by id alone, so a
 * cache keyed on them keeps the first rows it computed when the paused
 * status arrives or the stored row or contract changes (a resume left
 * locked as "Token is not paused" on a paused token). Resolving reads a few
 * small rule columns, so it is cheap.
 */
@Composable
internal fun TokenActionRows(
    token: TokenEntity,
    livePaused: Boolean?,
    identity: IdentityEntity,
    contract: DataContractEntity?,
    oncePerIdentityClaimed: Boolean,
    onOpen: (TokenActionKind) -> Unit,
) {
    val resolvedToken = livePaused?.let { token.copy(isPaused = it) } ?: token
    val rows = TokenActionResolver.resolve(
        resolvedToken, identity, contract, oncePerIdentityClaimed,
    ).filter { !it.permission.isHidden }
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
                .clickable(enabled = allowed) { onOpen(row.kind) }
                .testTag("tokenActions.row.${row.kind.id}"),
        )
    }
}
