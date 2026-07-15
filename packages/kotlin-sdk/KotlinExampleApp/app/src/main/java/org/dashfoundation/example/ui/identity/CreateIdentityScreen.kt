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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.RegistrationProgress
import org.dashfoundation.example.services.IdentityRegistrationController
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.wallet.toHexString

/**
 * Create-identity flow — port of `CreateIdentityView.swift`, split into the
 * source-wallet + [FundingSection] + [KeysSection] composables the Swift view
 * factors into. This milestone wires the **wallet-balance (Core-funded)**
 * path — `platform_wallet_register_identity_with_funding_signer` — which is
 * the single FFI entry the coordinator body invokes. The asset-lock-resume,
 * shielded-pool, and platform-address funding sources are deferred (see the
 * B-M3 deferrals); the walletless raw-proof path likewise.
 *
 * Submit sequence (mirrors Swift): preview + persist the identity keys (the
 * `.preparingKeys` phase), then `coordinator.startRegistration` with a body
 * that calls the single registration FFI entry point — no orchestration on
 * the Kotlin side.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateIdentityScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val coordinator = container.registrationCoordinator

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val wallets = remember(walletsMap) { walletsMap.values.toList() }

    var selectedWallet by remember(wallets) { mutableStateOf(wallets.firstOrNull()) }
    var fundingSource by remember { mutableStateOf(CreateIdentityFundingSource.CoreBalance) }
    var amountText by remember { mutableStateOf("50000000") } // 0.5 DASH in duffs default
    var identityIndexText by remember { mutableStateOf("0") }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    val scope = androidx.compose.runtime.rememberCoroutineScope()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Create Identity") },
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
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Source Wallet") {
                if (wallets.isEmpty()) {
                    Text(
                        "No wallets on ${network.displayName}. Create a wallet first.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    AccessiblePicker(
                        label = "Wallet",
                        options = wallets,
                        selected = selectedWallet ?: wallets.first(),
                        optionLabel = { it.walletIdHex.take(12) + "…" },
                        testTag = "createIdentity.sourceWalletPicker",
                        onSelected = { selectedWallet = it },
                    )
                }
            }

            FundingSection(
                source = fundingSource,
                onSourceChange = { newSource ->
                    fundingSource = newSource
                    // The shielded path spends a FIXED denomination, not a
                    // free-form amount — snap to a valid one when switching in.
                    if (newSource == CreateIdentityFundingSource.ShieldedBalance &&
                        amountText.toLongOrNull() !in
                        SHIELDED_IDENTITY_DENOMINATIONS.map { it.first }
                    ) {
                        amountText = SHIELDED_IDENTITY_DENOMINATIONS.first().first.toString()
                    }
                },
                amountText = amountText,
                onAmountChange = { amountText = it.filter(Char::isDigit) },
            )

            KeysSection(
                identityIndexText = identityIndexText,
                onIndexChange = { identityIndexText = it.filter(Char::isDigit) },
            )

            SubmitButton(
                text = "Create Identity",
                isLoading = isSubmitting,
                enabled = selectedWallet != null &&
                    fundingSource.wired &&
                    amountText.toLongOrNull() != null &&
                    identityIndexText.toIntOrNull() != null,
                modifier = Modifier.fillMaxWidth().testTag("createIdentity.submit"),
            ) {
                val wallet = selectedWallet ?: return@SubmitButton
                val amount = amountText.toLongOrNull() ?: return@SubmitButton
                val identityIndex = identityIndexText.toIntOrNull() ?: return@SubmitButton
                val mgr = manager ?: return@SubmitButton
                isSubmitting = true
                scope.launch {
                    try {
                        // Step 1 (`.preparingKeys`): derive + persist the full
                        // canonical identity key SET — the single allowed Kotlin
                        // persist step. Rust derives keyId 0..3 (MASTER auth,
                        // CRITICAL auth, HIGH auth, TRANSFER/CRITICAL) at this
                        // identity index and stamps each key's role by keyId at
                        // registration; we store each private key under its
                        // pubkey hex. Deriving only the MASTER key here (the old
                        // `count = 1` bug) left the identity unable to sign any
                        // document / token / transfer / withdrawal write.
                        val keys = mgr.identityRegistration.previewRegistrationKeySet(
                            walletHandle = wallet.handle,
                            mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                            identityIndex = identityIndex,
                            count = -1,
                        )
                        keys.forEach { key ->
                            try {
                                container.walletStorage.storePrivateKey(
                                    key.publicKeyHex,
                                    key.privateKey,
                                    // Prestored BEFORE the registration
                                    // broadcasts: only the owner index makes
                                    // it reachable by wallet deletion until
                                    // the public_keys row commits.
                                    ownerWalletId = wallet.walletId,
                                )
                            } finally {
                                // Keystore is authoritative from here; the JVM
                                // copy must not outlive the store (the
                                // IdentityKeyPreview retention rule — the
                                // registration blob reads only publicKey).
                                key.privateKey.fill(0)
                            }
                        }
                        // Step 2: hand the single registration FFI entry point to
                        // the coordinator as the body — no orchestration here. The
                        // funding source picks which registration FFI runs (ID-01
                        // Core asset lock vs ID-08 Platform addresses).
                        when (fundingSource) {
                            CreateIdentityFundingSource.CoreBalance -> {
                                coordinator.startRegistration(
                                    walletId = wallet.walletId,
                                    identityIndex = identityIndex,
                                    fundingKind = IdentityRegistrationController.FundingKind.AssetLock,
                                    body = {
                                        mgr.identityRegistration.registerWithWalletFunding(
                                            walletHandle = wallet.handle,
                                            amountDuffs = amount,
                                            accountIndex = 0,
                                            identityIndex = identityIndex,
                                            keys = keys,
                                            signerHandle = mgr.signerHandle,
                                            coreSignerHandle = mgr.mnemonicResolverHandle,
                                        )
                                    },
                                )
                            }
                            CreateIdentityFundingSource.PlatformAddress -> {
                                // Enumerate balance-carrying Platform-payment
                                // addresses, then greedily pack up to `amount`
                                // credits (largest first) — matching the ID-06
                                // top-up input assembly. Nonces are auto-fetched
                                // Rust-side by the register-from-addresses FFI.
                                val candidates = wallet.addressesWithBalances()
                                    .sortedByDescending { it.credits }
                                val inputs = packFundingInputs(candidates, amount)
                                if (inputs.isEmpty()) {
                                    error = "Not enough Platform-address balance to fund " +
                                        "$amount credits. Fund a Platform address first."
                                    return@launch
                                }
                                coordinator.startRegistration(
                                    walletId = wallet.walletId,
                                    identityIndex = identityIndex,
                                    fundingKind =
                                        IdentityRegistrationController.FundingKind.PlatformAddresses,
                                    body = {
                                        mgr.identityRegistration.registerFromAddresses(
                                            walletHandle = wallet.handle,
                                            identityIndex = identityIndex,
                                            keys = keys,
                                            signerHandle = mgr.signerHandle,
                                            inputs = inputs,
                                        )
                                    },
                                )
                            }
                            CreateIdentityFundingSource.ShieldedBalance -> {
                                // Type-20: spend a fixed-denomination note from
                                // the bound shielded pool. `amount` holds the
                                // chosen denomination (credits) from the picker.
                                // The 21-byte fallback failure address is REQUIRED
                                // — build it from the wallet's first Platform-
                                // payment address (1 variant tag + 20-byte hash),
                                // the same (addressType, hash) pairing ID-08 feeds.
                                val fallbackInput = wallet.addressesWithBalances().firstOrNull()
                                if (fallbackInput == null) {
                                    error = "This wallet has no Platform-payment address for " +
                                        "the required Type-20 creation-failure fallback. Fund " +
                                        "a Platform address first (Wallet → Platform Balance → " +
                                        "Top Up from Core)."
                                    return@launch
                                }
                                val fallbackAddress =
                                    byteArrayOf(fallbackInput.addressType.toByte()) +
                                        fallbackInput.hash
                                coordinator.startRegistration(
                                    walletId = wallet.walletId,
                                    identityIndex = identityIndex,
                                    fundingKind =
                                        IdentityRegistrationController.FundingKind.ShieldedPool,
                                    body = {
                                        mgr.shieldedIdentityCreateFromPool(
                                            walletId = wallet.walletId,
                                            identityIndex = identityIndex,
                                            keys = keys,
                                            denomination = amount,
                                            fallbackAddress = fallbackAddress,
                                        )
                                    },
                                    // Ambiguous broadcast: the identity may
                                    // already be live — record Unconfirmed
                                    // (slot held) instead of retryable Failed.
                                    isUnconfirmed = {
                                        (it as? org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed)
                                            ?.identityId
                                    },
                                )
                            }
                            CreateIdentityFundingSource.AssetLockResume -> {
                                error = "\"${fundingSource.label}\" funding is not wired yet."
                                return@launch
                            }
                        }
                        navController.navigate(
                            RegistrationProgress(wallet.walletId.toHexString(), identityIndex),
                        )
                    } catch (e: Exception) {
                        error = e.message ?: "Failed to prepare registration"
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
 * The funding sources `CreateIdentityView` offers. [CoreBalance] (ID-01,
 * `platform_wallet_register_identity_with_funding_signer`) and
 * [PlatformAddress] (ID-08, `platform_wallet_register_identity_with_signer`)
 * both execute; [AssetLockResume]
 * (`platform_wallet_resume_identity_with_existing_asset_lock_signer`) is
 * still deferred.
 *
 * @property amountInCredits true when the amount field is denominated in
 *   credits (the Platform-address path spends existing Platform credits);
 *   false when it is duffs (the Core path locks Dash).
 */
enum class CreateIdentityFundingSource(
    val label: String,
    val wired: Boolean,
    val amountInCredits: Boolean,
) {
    CoreBalance("Core balance", true, amountInCredits = false),
    PlatformAddress("Platform address", true, amountInCredits = true),
    ShieldedBalance("Shielded balance", true, amountInCredits = true),
    AssetLockResume("Resume from asset lock", false, amountInCredits = false),
}

/**
 * The fixed exit denominations (in CREDITS) a Type-20 shielded identity
 * create may spend from the pool — 0.1 / 0.3 / 0.5 / 1.0 DASH. Source of
 * truth: `shielded_identity_create_denominations` in DPP; a submitted
 * denomination not in the on-chain set is rejected at validation (← iOS
 * `CreateIdentityView.shieldedIdentityCreateDenominations`).
 */
private val SHIELDED_IDENTITY_DENOMINATIONS: List<Pair<Long, String>> = listOf(
    10_000_000_000L to "0.1 DASH",
    30_000_000_000L to "0.3 DASH",
    50_000_000_000L to "0.5 DASH",
    100_000_000_000L to "1.0 DASH",
)

/**
 * Funding source + amount — the Swift `CreateIdentityView` funding/amount
 * sections. The source picker offers the three iOS options; only Core
 * balance executes (the other two carry the deferral note).
 */
@Composable
private fun FundingSection(
    source: CreateIdentityFundingSource,
    onSourceChange: (CreateIdentityFundingSource) -> Unit,
    amountText: String,
    onAmountChange: (String) -> Unit,
) {
    FormSection(title = "Funding") {
        org.dashfoundation.example.ui.components.AccessiblePicker(
            label = "Funding source",
            options = CreateIdentityFundingSource.entries,
            selected = source,
            optionLabel = { it.label },
            testTag = "createIdentity.fundingSourcePicker",
            onSelected = onSourceChange,
        )
        if (!source.wired) {
            Text(
                "\"${source.label}\" funding is not bridged yet (needs the " +
                    "platform-address signer accessor). Core balance is the wired path.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
        if (source == CreateIdentityFundingSource.ShieldedBalance) {
            // Type-20 spends a FIXED exit denomination, not a free-form amount
            // (← iOS's denomination Picker). The picked value flows back through
            // [amountText] (in credits) so the submit path reads it uniformly.
            val selectedDenom = SHIELDED_IDENTITY_DENOMINATIONS
                .firstOrNull { it.first.toString() == amountText }
                ?: SHIELDED_IDENTITY_DENOMINATIONS.first()
            org.dashfoundation.example.ui.components.AccessiblePicker(
                label = "Denomination",
                options = SHIELDED_IDENTITY_DENOMINATIONS,
                selected = selectedDenom,
                optionLabel = { it.second },
                testTag = "createIdentity.denominationPicker",
                onSelected = { onAmountChange(it.first.toString()) },
            )
            Text(
                "Spends one fixed-denomination note from the shielded pool. " +
                    "Requires a synced pool with a note of this size.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
        } else {
            OutlinedTextField(
                value = amountText,
                onValueChange = onAmountChange,
                label = {
                    Text(if (source.amountInCredits) "Amount (credits)" else "Amount (duffs)")
                },
                singleLine = true,
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(
                    keyboardType = KeyboardType.Number,
                ),
                modifier = Modifier.fillMaxWidth().testTag("createIdentity.amount"),
            )
        }
    }
}

/**
 * Greedily select funding inputs from [candidates] (assumed sorted
 * largest-balance-first) until their combined credits cover [target],
 * spending only what's needed from the final address. Returns an empty list
 * when the candidates can't cover [target]. Mirrors the ID-06 top-up
 * `packInputs`.
 */
private fun packFundingInputs(candidates: List<FundingInput>, target: Long): List<FundingInput> {
    if (target <= 0) return emptyList()
    var remaining = target
    val picked = ArrayList<FundingInput>()
    for (input in candidates) {
        if (remaining <= 0) break
        val spend = minOf(input.credits, remaining)
        picked.add(input.copy(credits = spend))
        remaining -= spend
    }
    return if (remaining <= 0) picked else emptyList()
}

/** Identity-slot section — the Swift `CreateIdentityView` identity-index stepper. */
@Composable
private fun KeysSection(identityIndexText: String, onIndexChange: (String) -> Unit) {
    FormSection(title = "Identity Slot") {
        OutlinedTextField(
            value = identityIndexText,
            onValueChange = onIndexChange,
            label = { Text("Identity index (HD slot)") },
            singleLine = true,
            keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth().testTag("createIdentity.identityIndex"),
        )
    }
}
