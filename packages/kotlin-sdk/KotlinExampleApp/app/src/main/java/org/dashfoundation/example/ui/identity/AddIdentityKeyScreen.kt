package org.dashfoundation.example.ui.identity

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
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.identity.ContractBounds
import org.dashfoundation.dashsdk.identity.KeyPurpose
import org.dashfoundation.dashsdk.identity.KeyType
import org.dashfoundation.dashsdk.identity.SecurityLevel
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.services.IdentityKeyAdditionFlow
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.contracts.ParsedContract
import org.dashfoundation.example.ui.credits.rememberManagedWalletFor
import org.dashfoundation.example.util.hexToBytes

/**
 * Form for adding a new public key to an existing identity — port of
 * `AddIdentityKeyView.swift`. The form constrains the user's choices to
 * combinations Drive accepts:
 *
 * - Authentication → Critical / High / Medium, no contract bounds
 * - Encryption / Decryption → Medium (locked), contract bounds required
 * - Transfer → Critical (locked), no bounds
 *
 * Master / System / Voting / Owner aren't pickable — the first three are
 * minted at registration; Owner keys belong to masternode tooling.
 *
 * Submit runs [IdentityKeyAdditionFlow.prepareKeys] (derive → Keystore
 * persist → [org.dashfoundation.dashsdk.identity.IdentityPubkey]) and then
 * `PlatformWalletManager.identityUpdates.update` — the same
 * `platform_wallet_update_identity_with_signer` call Swift makes. The
 * derive step currently surfaces
 * [IdentityKeyAdditionFlow.UnbridgedSlotDeriver]'s named gap (the bridged
 * slot-derive returns the private scalar without the public half).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddIdentityKeyScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()

    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val network by appState.currentNetwork.collectAsStateWithLifecycle()

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val keys by container.database.publicKeyDao()
        .observeByIdentityId(org.dashfoundation.example.util.Base58.encode(idBytes))
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val savedContracts by container.database.dataContractDao()
        .observeByNetwork(network.ffiValue)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var keyType by remember { mutableStateOf(KeyType.ECDSA_SECP256K1) }
    var purpose by remember { mutableStateOf(KeyPurpose.AUTHENTICATION) }
    var authSecurityLevel by remember { mutableStateOf(SecurityLevel.HIGH) }
    var boundEntry by remember { mutableStateOf<BoundsPickerEntry?>(null) }
    var boundDocumentTypeName by remember { mutableStateOf("") }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // Locked combinations (← AddIdentityKeyView.effectiveKeyType /
    // effectiveSecurityLevel): ENCRYPTION/DECRYPTION do ECDH so the key
    // type is pinned to secp256k1 and DPP pins their level to MEDIUM;
    // TRANSFER is protocol-locked to CRITICAL.
    val boundsRequired = purpose == KeyPurpose.ENCRYPTION || purpose == KeyPurpose.DECRYPTION
    val effectiveKeyType = if (boundsRequired) KeyType.ECDSA_SECP256K1 else keyType
    val effectiveSecurityLevel = when (purpose) {
        KeyPurpose.TRANSFER -> SecurityLevel.CRITICAL
        KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION -> SecurityLevel.MEDIUM
        else -> authSecurityLevel
    }
    val nextKeyId = IdentityKeyAdditionFlow.nextKeyId(keys.map { it.keyId })

    // Bounds picker entries: DashPay (system, always available) first,
    // then user-saved contracts on this network (duplicates skipped).
    val pickerEntries = remember(savedContracts) {
        val system = listOf(dashPaySystemEntry())
        val systemIds = system.map { it.idHex }.toSet()
        system + savedContracts
            .filter { it.id.toHexLocal() !in systemIds }
            .map { entity ->
                BoundsPickerEntry(
                    idHex = entity.id.toHexLocal(),
                    displayName = entity.name,
                    allowsContractScope = true,
                    documentTypes = ParsedContract.from(entity)
                        ?.documentTypes?.keys?.sorted().orEmpty(),
                )
            }
    }
    val documentTypeRequired = boundEntry?.allowsContractScope == false
    val contractBoundsMissing = boundsRequired &&
        (boundEntry == null || (documentTypeRequired && boundDocumentTypeName.isBlank()))

    val canSubmit = !isSubmitting && wallet != null && manager != null &&
        effectiveKeyType != KeyType.BLS12_381 && !contractBoundsMissing

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Add Identity Key") },
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
            FormSection(title = "New Key") {
                if (boundsRequired) {
                    LabeledContent("Key Type", "ECDSA secp256k1")
                } else {
                    AccessiblePicker(
                        label = "Key Type",
                        options = listOf(
                            KeyType.ECDSA_SECP256K1, KeyType.ECDSA_HASH160, KeyType.BLS12_381,
                        ),
                        selected = keyType,
                        optionLabel = { it.displayName() },
                        testTag = "addIdentityKey.keyTypePicker",
                        onSelected = { keyType = it },
                    )
                }
                AccessiblePicker(
                    label = "Purpose",
                    options = listOf(
                        KeyPurpose.AUTHENTICATION, KeyPurpose.ENCRYPTION,
                        KeyPurpose.DECRYPTION, KeyPurpose.TRANSFER,
                    ),
                    selected = purpose,
                    optionLabel = { it.displayName() },
                    testTag = "addIdentityKey.purposePicker",
                    onSelected = { newPurpose ->
                        purpose = newPurpose
                        // Reset bounds when leaving Encryption/Decryption so a
                        // stale binding can't ride along (← Swift onChange).
                        if (newPurpose != KeyPurpose.ENCRYPTION &&
                            newPurpose != KeyPurpose.DECRYPTION
                        ) {
                            boundEntry = null
                            boundDocumentTypeName = ""
                        }
                    },
                )
                when (purpose) {
                    KeyPurpose.TRANSFER -> LabeledContent("Security Level", "Critical")
                    KeyPurpose.ENCRYPTION, KeyPurpose.DECRYPTION ->
                        LabeledContent("Security Level", "Medium")
                    else -> AccessiblePicker(
                        label = "Security Level",
                        options = listOf(
                            SecurityLevel.CRITICAL, SecurityLevel.HIGH, SecurityLevel.MEDIUM,
                        ),
                        selected = authSecurityLevel,
                        optionLabel = { it.displayName() },
                        testTag = "addIdentityKey.securityLevelPicker",
                        onSelected = { authSecurityLevel = it },
                    )
                }
            }

            if (boundsRequired) {
                FormSection(title = "Contract Bounds (required)") {
                    AccessiblePicker(
                        label = "Contract",
                        options = listOf<BoundsPickerEntry?>(null) + pickerEntries,
                        selected = boundEntry,
                        optionLabel = { it?.displayName ?: "Select a contract" },
                        testTag = "addIdentityKey.boundsContractPicker",
                        onSelected = { entry ->
                            boundEntry = entry
                            // Switching contracts invalidates the doc-type pick;
                            // auto-pick the single required type (DashPay →
                            // contactRequest), else default to contract scope.
                            boundDocumentTypeName =
                                if (entry != null && !entry.allowsContractScope &&
                                    entry.documentTypes.size == 1
                                ) {
                                    entry.documentTypes.first()
                                } else {
                                    ""
                                }
                        },
                    )
                    val docTypes = boundEntry?.documentTypes.orEmpty()
                    if (docTypes.isNotEmpty()) {
                        val options = if (documentTypeRequired) docTypes else listOf("") + docTypes
                        AccessiblePicker(
                            label = if (documentTypeRequired) {
                                "Document Type (required)"
                            } else {
                                "Document Type (optional)"
                            },
                            options = options,
                            selected = boundDocumentTypeName,
                            optionLabel = { it.ifEmpty { "Any document type" } },
                            testTag = "addIdentityKey.boundsDocTypePicker",
                            onSelected = { boundDocumentTypeName = it },
                        )
                    }
                    Text(
                        if (documentTypeRequired) {
                            "This contract requires the key to be bound to a specific " +
                                "document type — a contract-scope-only bound is rejected " +
                                "at submit."
                        } else {
                            "Encryption / decryption keys must be scoped to a specific " +
                                "contract. A document type narrows the scope further; " +
                                "leaving it blank covers all the contract's document types."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Slot") {
                LabeledContent("Auto-assigned key id", "#$nextKeyId")
                Text(
                    "Picked as max(existingKeyIds) + 1. Slots are non-recyclable — " +
                        "disabled keys leave a hole in the range; new keys always " +
                        "extend past the highest ever used.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            if (effectiveKeyType == KeyType.BLS12_381) {
                Text(
                    "BLS derivation is not yet wired through the FFI for this flow. " +
                        "Pick ECDSA secp256k1 or ECDSA Hash160 to add a key now.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            if (identity != null && identity?.walletId == null) {
                Text(
                    "Identity has no wallet linkage; cannot derive new keys.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            SubmitButton(
                text = "Add Key",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("addIdentityKey.submit"),
            ) {
                val w = wallet ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                val ident = identity ?: return@SubmitButton
                val bounds: ContractBounds? = if (boundsRequired) {
                    val entry = boundEntry ?: return@SubmitButton
                    val trimmedType = boundDocumentTypeName.trim()
                    if (trimmedType.isEmpty()) {
                        ContractBounds.SingleContract(entry.idHex.hexToBytes())
                    } else {
                        ContractBounds.SingleContractDocumentType(
                            entry.idHex.hexToBytes(), trimmedType,
                        )
                    }
                } else {
                    null
                }
                val spec = IdentityKeyAdditionFlow.KeySpec(
                    keyType = effectiveKeyType,
                    purpose = purpose,
                    securityLevel = effectiveSecurityLevel,
                    contractBounds = bounds,
                )
                isSubmitting = true
                scope.launch {
                    try {
                        val rows = IdentityKeyAdditionFlow.prepareKeys(
                            specs = listOf(spec),
                            existingKeyIds = keys.map { it.keyId },
                            identityIndex = ident.identityIndex,
                            walletStorage = container.walletStorage,
                            walletId = w.walletId,
                            // Real slot derive: keypair (public half incl.)
                            // via the manager's resolver-keyed FFI.
                            deriver = { identityIndex, keyId ->
                                val (priv, pub) = mgr.deriveIdentityKeyPair(
                                    walletId = w.walletId,
                                    identityIndex = identityIndex,
                                    keyIndex = keyId,
                                )
                                IdentityKeyAdditionFlow.DerivedKey(
                                    publicKey = pub,
                                    privateKey = priv,
                                )
                            },
                        )
                        mgr.identityUpdates.update(
                            walletHandle = w.handle,
                            identityId = idBytes,
                            addPublicKeys = rows,
                            signerHandle = mgr.signerHandle,
                        )
                        navController.popBackStack()
                    } catch (e: Exception) {
                        error = e.message ?: "Add key failed"
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * Unified row shape for the bounds picker — covers the static system
 * registry and per-network Room contract rows (← Swift
 * `BoundsPickerEntry`).
 */
private data class BoundsPickerEntry(
    val idHex: String,
    val displayName: String,
    /**
     * Mirrors the contract-level `requiresIdentityEncryptionBoundedKey`
     * flag — when false, `ContractBounds.SingleContract` is rejected by
     * DPP and a document type must be picked.
     */
    val allowsContractScope: Boolean,
    val documentTypes: List<String>,
)

/**
 * DashPay — the one system contract whose schema declares
 * `requiresIdentityEncryptionBoundedKey` (on `contactRequest` only, so
 * contract scope is not allowed). ID bytes:
 * `packages/dashpay-contract/src/lib.rs::ID_BYTES`; network-agnostic.
 */
private fun dashPaySystemEntry(): BoundsPickerEntry = BoundsPickerEntry(
    idHex = byteArrayOf(
        162.toByte(), 161.toByte(), 180.toByte(), 172.toByte(), 111, 239.toByte(), 34, 234.toByte(),
        42, 26, 104, 232.toByte(), 18, 54, 68, 179.toByte(),
        87, 135.toByte(), 95, 107, 65, 44, 24, 16,
        146.toByte(), 129.toByte(), 193.toByte(), 70, 231.toByte(), 178.toByte(), 113, 188.toByte(),
    ).toHexLocal(),
    displayName = "DashPay (System)",
    allowsContractScope = false,
    documentTypes = listOf("contactRequest"),
)

private fun ByteArray.toHexLocal(): String = joinToString("") { "%02x".format(it) }

private fun KeyType.displayName(): String = when (this) {
    KeyType.ECDSA_SECP256K1 -> "ECDSA secp256k1"
    KeyType.ECDSA_HASH160 -> "ECDSA Hash160"
    KeyType.BLS12_381 -> "BLS12-381"
    KeyType.BIP13_SCRIPT_HASH -> "BIP13 Script Hash"
    KeyType.EDDSA_25519_HASH160 -> "EdDSA 25519 Hash160"
}

private fun KeyPurpose.displayName(): String = when (this) {
    KeyPurpose.AUTHENTICATION -> "Authentication"
    KeyPurpose.ENCRYPTION -> "Encryption"
    KeyPurpose.DECRYPTION -> "Decryption"
    KeyPurpose.TRANSFER -> "Transfer"
    KeyPurpose.SYSTEM -> "System"
    KeyPurpose.VOTING -> "Voting"
    KeyPurpose.OWNER -> "Owner"
}

private fun SecurityLevel.displayName(): String = when (this) {
    SecurityLevel.MASTER -> "Master"
    SecurityLevel.CRITICAL -> "Critical"
    SecurityLevel.HIGH -> "High"
    SecurityLevel.MEDIUM -> "Medium"
}
