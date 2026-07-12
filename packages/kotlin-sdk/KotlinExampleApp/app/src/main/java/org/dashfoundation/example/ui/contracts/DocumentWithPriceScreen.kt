package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.services.IdentityKeys
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.formatCredits
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * Price probe + purchase / set-price for one document — port of
 * `DocumentWithPriceView.swift` (the debounced document-id probe with
 * price / ownership badging) combined with the `PurchaseDocumentView` and
 * set-price submit flows from `DocumentsView.swift`, hosted as one screen.
 *
 * - The probe fetches the document via the bridged `Documents.fetch`
 *   (`documentSearch` with an `$id` where clause) and reads `$price` /
 *   `$ownerId` off the returned object.
 * - When the acting identity owns the document, the owner path offers
 *   Set Price via `DocumentTransactions.setPrice`
 *   (`platform_wallet_document_set_price`).
 * - Otherwise (and when a price is set) the buyer path purchases via
 *   `DocumentTransactions.purchase` (`platform_wallet_document_purchase`);
 *   consensus rejects self-buys, so the UI gates the owner out of it.
 *
 * The signing key resolves like iOS `DocumentActionRunner.resolveSigning`:
 * an enabled AUTHENTICATION key at or above the document type's
 * `signatureSecurityLevelRequirement` (default HIGH) whose private half is
 * in the Keystore.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentWithPriceScreen(
    contractIdHex: String,
    typeName: String,
    initialDocumentId: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

    val contractIdBytes = remember(contractIdHex) { contractIdHex.hexToBytes() }
    val contractIdBase58 = remember(contractIdHex) { Base58.encode(contractIdBytes) }

    val identities by container.database.identityDao()
        .observeWalletOwnedByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    var actingIdentity by remember { mutableStateOf<IdentityEntity?>(null) }
    LaunchedEffect(identities) {
        if (actingIdentity == null) actingIdentity = identities.firstOrNull()
    }

    var documentIdText by rememberSaveable { mutableStateOf(initialDocumentId) }
    var isLoading by remember { mutableStateOf(false) }
    var probeError by remember { mutableStateOf<String?>(null) }
    var documentJson by remember { mutableStateOf<ProbedDocument?>(null) }

    var newPriceText by rememberSaveable { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var successMessage by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var probeTick by remember { mutableStateOf(0) }

    // Debounced probe (← DocumentWithPriceView's 0.5 s debounce timer; also
    // fires for a pre-seeded id, matching its onAppear kick).
    LaunchedEffect(documentIdText, sdk, probeTick) {
        documentJson = null
        probeError = null
        val currentSdk = sdk ?: return@LaunchedEffect
        val trimmed = documentIdText.trim()
        if (trimmed.isEmpty()) return@LaunchedEffect
        val docIdBytes = Base58.decodeIdentifier(trimmed)
        if (docIdBytes == null) {
            probeError = "Invalid document ID format"
            return@LaunchedEffect
        }
        delay(500)
        isLoading = true
        try {
            val json = currentSdk.contracts.fetch(contractIdBase58).use { contract ->
                currentSdk.documents.fetch(contract, typeName, Base58.encode(docIdBytes))
            }
            if (json == null) {
                probeError = "Document not found"
            } else {
                documentJson = parseProbedDocument(json)
            }
        } catch (e: Exception) {
            probeError = e.message ?: "Fetch failed"
        } finally {
            isLoading = false
        }
    }

    val probed = documentJson
    val actingBase58 = actingIdentity?.let { Base58.encode(it.identityId) }
    val isOwner = probed != null && actingBase58 != null && probed.ownerBase58 == actingBase58
    val price = probed?.price

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Document Price") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isSubmitting) {
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
            FormSection(title = "Document") {
                LabeledContent("Contract", truncateMiddle(contractIdBase58, 10, 6))
                LabeledContent("Type", typeName)
                OutlinedTextField(
                    value = documentIdText,
                    onValueChange = { documentIdText = it },
                    label = { Text("Document ID (base58 or hex)") },
                    singleLine = true,
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("documentWithPrice.documentId"),
                )
                when {
                    isLoading -> Text(
                        "Loading…",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    probeError != null -> Text(
                        probeError!!,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("documentWithPrice.probeError"),
                    )
                    probed != null -> {
                        LabeledContent("Status", "Document found")
                        LabeledContent(
                            "Owner",
                            truncateMiddle(probed.ownerBase58 ?: "unknown", 10, 6),
                        )
                        when {
                            isOwner -> LabeledContent("Ownership", "You are the owner")
                            price != null && price > 0 ->
                                LabeledContent("Price", formatCredits(price))
                            else -> LabeledContent("Price", "Not for sale")
                        }
                    }
                    documentIdText.isNotBlank() -> Text(
                        "Enter a valid document ID to see its price",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Acting Identity") {
                if (identities.isEmpty()) {
                    Text(
                        "No wallet-owned identities on ${network.networkName} — " +
                            "create or load one first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Identity",
                        options = identities,
                        selected = actingIdentity ?: identities.first(),
                        optionLabel = {
                            it.alias ?: it.mainDpnsName
                                ?: truncateMiddle(Base58.encode(it.identityId), 10, 6)
                        },
                        testTag = "documentWithPrice.identityPicker",
                        onSelected = { actingIdentity = it },
                    )
                    Text(
                        if (isOwner) {
                            "You own this document — set or update its trade price below."
                        } else {
                            "The identity that buys and becomes the new owner. Must " +
                                "differ from the current owner."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            successMessage?.let { message ->
                Text(
                    message,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("documentWithPrice.success"),
                )
            }

            if (isOwner) {
                FormSection(title = "Set Price") {
                    OutlinedTextField(
                        value = newPriceText,
                        onValueChange = { newPriceText = it.filter(Char::isDigit) },
                        label = { Text("New price (credits)") },
                        supportingText = { Text("0 removes the price (not for sale)") },
                        singleLine = true,
                        enabled = !isSubmitting,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth().testTag("documentWithPrice.newPrice"),
                    )
                    SubmitButton(
                        text = "Set Price",
                        isLoading = isSubmitting,
                        enabled = !isSubmitting && manager != null &&
                            newPriceText.toLongOrNull() != null && probed != null,
                        modifier = Modifier.fillMaxWidth().testTag("documentWithPrice.setPrice"),
                    ) {
                        val owner = actingIdentity ?: return@SubmitButton
                        val newPrice = newPriceText.toLongOrNull() ?: return@SubmitButton
                        val docIdBytes = Base58.decodeIdentifier(documentIdText.trim())
                            ?: return@SubmitButton
                        submitDocumentTransaction(
                            begin = { isSubmitting = true; successMessage = null },
                            end = { isSubmitting = false },
                            fail = { error = it },
                            scope = scope,
                        ) {
                            val (wallet, mgr, signingKeyId) = resolveSigning(container, owner)
                            mgr.documentTransactions.setPrice(
                                walletHandle = wallet,
                                ownerId = owner.identityId,
                                contractId = contractIdBytes,
                                documentType = typeName,
                                documentId = docIdBytes,
                                price = newPrice,
                                signingKeyId = signingKeyId,
                                signerHandle = mgr.signerHandle,
                            )
                            successMessage = "Price updated on-chain."
                            probeTick++ // re-probe so the new $price renders
                        }
                    }
                }
            } else {
                FormSection(title = "Purchase") {
                    val purchasable = price != null && price > 0
                    if (!purchasable && probed != null) {
                        Text(
                            "This document is not currently for sale.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    SubmitButton(
                        text = "Purchase" + (price?.takeIf { it > 0 }
                            ?.let { " for ${formatCredits(it)}" } ?: ""),
                        isLoading = isSubmitting,
                        enabled = !isSubmitting && manager != null && purchasable &&
                            actingIdentity != null,
                        modifier = Modifier.fillMaxWidth().testTag("documentWithPrice.purchase"),
                    ) {
                        val purchaser = actingIdentity ?: return@SubmitButton
                        val purchasePrice = price ?: return@SubmitButton
                        val docIdBytes = Base58.decodeIdentifier(documentIdText.trim())
                            ?: return@SubmitButton
                        submitDocumentTransaction(
                            begin = { isSubmitting = true; successMessage = null },
                            end = { isSubmitting = false },
                            fail = { error = it },
                            scope = scope,
                        ) {
                            val (wallet, mgr, signingKeyId) = resolveSigning(container, purchaser)
                            mgr.documentTransactions.purchase(
                                walletHandle = wallet,
                                purchaserId = purchaser.identityId,
                                contractId = contractIdBytes,
                                documentType = typeName,
                                documentId = docIdBytes,
                                price = purchasePrice,
                                signingKeyId = signingKeyId,
                                signerHandle = mgr.signerHandle,
                            )
                            successMessage = "Purchase confirmed — you are the new owner."
                            probeTick++ // re-probe so ownership flips in the UI
                        }
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/** Launch [body] with begin/end/fail plumbing shared by the document submits. */
internal fun submitDocumentTransaction(
    begin: () -> Unit,
    end: () -> Unit,
    fail: (String) -> Unit,
    scope: kotlinx.coroutines.CoroutineScope,
    body: suspend () -> Unit,
) {
    begin()
    scope.launch {
        try {
            body()
        } catch (e: Exception) {
            fail(e.message ?: "Transaction failed")
        } finally {
            end()
        }
    }
}

/** `(walletHandle, manager, signingKeyId)` for [identity]'s submits. */
internal data class ResolvedSigning(
    val walletHandle: Long,
    val manager: org.dashfoundation.dashsdk.wallet.PlatformWalletManager,
    val signingKeyId: Int,
)

/**
 * Resolve the wallet handle + AUTHENTICATION signing key for [identity]
 * — the Kotlin counterpart of `DocumentActionRunner.resolveSigning`.
 * Throws with user-facing copy when the identity has no loaded wallet or
 * no qualifying key.
 */
internal suspend fun resolveSigning(
    container: org.dashfoundation.example.di.AppContainer,
    identity: IdentityEntity,
): ResolvedSigning {
    val manager = container.walletManagerStore.activeManager.value
        ?: throw IllegalStateException("Wallet manager not active")
    val walletIdHex = identity.walletId?.toHex()
        ?: throw IllegalStateException("Identity has no wallet linkage")
    val wallet = manager.wallets.value[walletIdHex]
        ?: throw IllegalStateException("Wallet not loaded in the wallet manager")

    val identityBase58 = Base58.encode(identity.identityId)
    val keys = container.database.publicKeyDao()
        .observeByIdentityId(identityBase58).first()
        .ifEmpty {
            container.database.publicKeyDao()
                .observeByIdentityId(identity.identityId.toHex()).first()
        }
    // Pre-filter to keys whose private half is in the Keystore (suspend
    // reads), then rank purely.
    val keysWithPrivate = keys.filter {
        container.walletStorage.hasPrivateKey(it.publicKeyData.toHex())
    }
    val signingKeyId = IdentityKeys.findAuthenticationSigningKeyId(
        keys = keysWithPrivate,
        minimumSecurityLevel = SIGNING_MINIMUM_LEVEL_HIGH,
        hasPrivateKey = { true },
    ) ?: throw IllegalStateException(
        "No enabled HIGH-or-stronger authentication key with a stored " +
            "private key on this identity",
    )
    return ResolvedSigning(wallet.handle, manager, signingKeyId)
}

/** DPP `SecurityLevel.HIGH` rank — the default document signing bound. */
internal const val SIGNING_MINIMUM_LEVEL_HIGH = 2

/** The fields the probe reads off the fetched document object. */
private data class ProbedDocument(
    val ownerBase58: String?,
    /** `$price` in credits, or null when the document has none. */
    val price: Long?,
)

/** Read `$ownerId` / `$price` (root or `data`-nested) from document JSON. */
private fun parseProbedDocument(json: String): ProbedDocument? = try {
    val root = LenientJson.parseToJsonElement(json).jsonObject
    val data = root["data"] as? kotlinx.serialization.json.JsonObject
    val owner = ((root["\$ownerId"] ?: root["ownerId"]) as? JsonPrimitive)?.content
    val price = ((root["\$price"] ?: data?.get("\$price")) as? JsonPrimitive)
        ?.content?.toLongOrNull()
    ProbedDocument(ownerBase58 = owner, price = price)
} catch (_: Exception) {
    null
}
