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
import org.dashfoundation.dashsdk.identity.RegistrationKeys
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.RegistrationProgress
import org.dashfoundation.example.services.DashpayKeyProvisioning
import org.dashfoundation.example.services.IdentityRegistrationController
import org.dashfoundation.example.services.assetlock.IdentityAssetLockRecovery
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.wallet.toHexString

/**
 * Create-identity flow — port of `CreateIdentityView.swift`, split into the
 * source-wallet + [FundingSection] + [KeysSection] composables the Swift view
 * factors into. This milestone wires the **wallet-balance (Core-funded)**
 * path plus Platform-address, shielded, and existing-asset-lock funding.
 * Existing-lock recovery selects from Rust's tracked-lock snapshot and
 * submits that exact outpoint; the walletless raw-proof path remains out of
 * scope.
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
    val protocolVersion by appState.platformProtocolVersion.collectAsStateWithLifecycle()
    // The shielded fixed-denomination set is protocol-version-gated (0.03
    // and 0.25 DASH only exist from v13; 0.3 DASH is retired at v13).
    val shieldedDenominations = shieldedIdentityDenominations(protocolVersion)
    val coordinator = container.registrationCoordinator

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val walletsMap by remember(manager) {
        manager?.wallets ?: MutableStateFlow(emptyMap<String, ManagedPlatformWallet>())
    }.collectAsStateWithLifecycle()
    val wallets = remember(walletsMap) { walletsMap.values.toList() }

    var selectedWallet by remember(wallets) { mutableStateOf(wallets.firstOrNull()) }
    var fundingSource by remember { mutableStateOf(CreateIdentityFundingSource.CoreBalance) }
    var amountText by remember { mutableStateOf("50000000") } // 0.5 DASH in duffs default

    // Re-snap a shielded amount when the version-gated set changes (a
    // protocol-version refresh can land after the user picked, retiring
    // the chosen denomination mid-session).
    LaunchedEffect(shieldedDenominations) {
        if (fundingSource == CreateIdentityFundingSource.ShieldedBalance &&
            amountText.toLongOrNull() !in shieldedDenominations.map { it.first }
        ) {
            amountText = shieldedDenominations.first().first.toString()
        }
    }
    var identityIndexText by remember { mutableStateOf("0") }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var recoveryLocks by remember { mutableStateOf(emptyList<TrackedAssetLock>()) }
    var selectedRecoveryLock by remember { mutableStateOf<TrackedAssetLock?>(null) }
    var recoveryLoadError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(manager, selectedWallet, fundingSource) {
        recoveryLoadError = null
        recoveryLocks = try {
            val mgr = manager
            val wallet = selectedWallet
            if (mgr == null || wallet == null) emptyList() else {
                IdentityAssetLockRecovery.registrations(
                    mgr.trackedIdentityRecoveryAssetLocks(wallet.walletId),
                )
            }
        } catch (e: Exception) {
            recoveryLoadError = e.message ?: "Failed to load tracked registration locks"
            emptyList()
        }
        selectedRecoveryLock = recoveryLocks.firstOrNull()
        if (fundingSource == CreateIdentityFundingSource.AssetLockResume) {
            selectedRecoveryLock?.let { identityIndexText = it.registrationIndex.toString() }
        } else {
            selectedWallet?.let { wallet ->
                runCatching { coordinator.nextSafeIdentityIndex(wallet.walletId) }
                    .onSuccess { identityIndexText = it.toString() }
                    .onFailure {
                        recoveryLoadError = "Failed to determine the next safe identity index"
                    }
            }
        }
    }

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
                        shieldedDenominations.map { it.first }
                    ) {
                        amountText = shieldedDenominations.first().first.toString()
                    }
                },
                amountText = amountText,
                onAmountChange = { amountText = it.filter(Char::isDigit) },
                recoveryLocks = recoveryLocks,
                recoveryLoadError = recoveryLoadError,
                selectedRecoveryLock = selectedRecoveryLock,
                onRecoveryLockChange = {
                    selectedRecoveryLock = it
                    identityIndexText = it.registrationIndex.toString()
                },
                shieldedDenominations = shieldedDenominations,
            )

            KeysSection(
                identityIndexText = identityIndexText,
                onIndexChange = { identityIndexText = it.filter(Char::isDigit) },
            )

            SubmitButton(
                text = "Create Identity",
                isLoading = isSubmitting,
                enabled = selectedWallet != null &&
                    isCreateIdentityFundingAmountValid(
                        fundingSource,
                        amountText.toLongOrNull(),
                    ) &&
                    (fundingSource != CreateIdentityFundingSource.AssetLockResume ||
                        selectedRecoveryLock != null) &&
                    identityIndexText.toIntOrNull() != null,
                modifier = Modifier.fillMaxWidth().testTag("createIdentity.submit"),
            ) {
                val wallet = selectedWallet ?: return@SubmitButton
                val submission = createIdentitySubmissionSnapshot(
                    fundingSource = fundingSource,
                    amountText = amountText,
                    identityIndexText = identityIndexText,
                    selectedRecoveryLock = selectedRecoveryLock,
                ) ?: return@SubmitButton
                val amount = submission.amount
                if (!isCreateIdentityFundingAmountValid(submission.fundingSource, amount)) {
                    error = if (submission.fundingSource == CreateIdentityFundingSource.CoreBalance) {
                        val minimum = minimumCoreFundingDuffsForKeyCount(
                            RegistrationKeys.keyCount(submission.includesDashPayKeys),
                        )
                        "Core-funded registration with DashPay keys requires at least " +
                            "$minimum duffs."
                    } else {
                        "Funding amount must be positive."
                    }
                    return@SubmitButton
                }
                val identityIndex = submission.identityIndex
                val mgr = manager ?: return@SubmitButton
                val registerDashPayKeys = submission.includesDashPayKeys
                isSubmitting = true
                scope.launch {
                    try {
                        // Freeze and validate funding-specific prerequisites
                        // BEFORE deriving or persisting any registration key.
                        // The UI remains interactive while this coroutine runs,
                        // but every operation below uses only `submission`.
                        val platformInputs = if (
                            submission.fundingSource == CreateIdentityFundingSource.PlatformAddress
                        ) {
                            val requiredAmount = amount
                                ?: error("A funding amount is required")
                            val inputs = packFundingInputs(
                                candidates = wallet.addressesWithBalances(),
                                target = requiredAmount,
                                keyCount = RegistrationKeys.keyCount(registerDashPayKeys),
                            )
                            if (inputs.isEmpty()) {
                                error = "Not enough Platform-address balance to fund " +
                                    "$requiredAmount credits and reserve the identity-create fee."
                                return@launch
                            }
                            inputs
                        } else {
                            null
                        }
                        val shieldedFallbackAddress = if (
                            submission.fundingSource == CreateIdentityFundingSource.ShieldedBalance
                        ) {
                            val fallbackInput = wallet.addressesWithBalances().firstOrNull()
                            if (fallbackInput == null) {
                                error = "This wallet has no Platform-payment address for " +
                                    "the required Type-20 creation-failure fallback. Fund " +
                                    "a Platform address first (Wallet → Platform Balance → " +
                                    "Top Up from Core)."
                                return@launch
                            }
                            byteArrayOf(fallbackInput.addressType.toByte()) + fallbackInput.hash
                        } else {
                            null
                        }

                        val fundingKind = when (submission.fundingSource) {
                            CreateIdentityFundingSource.PlatformAddress ->
                                IdentityRegistrationController.FundingKind.PlatformAddresses
                            CreateIdentityFundingSource.ShieldedBalance ->
                                IdentityRegistrationController.FundingKind.ShieldedPool
                            else -> IdentityRegistrationController.FundingKind.AssetLock
                        }
                        coordinator.startRegistration(
                            walletId = wallet.walletId,
                            identityIndex = identityIndex,
                            fundingKind = fundingKind,
                            reservationKind = if (
                                submission.fundingSource == CreateIdentityFundingSource.AssetLockResume
                            ) {
                                org.dashfoundation.example.services.RegistrationCoordinator
                                    .ReservationKind.AssetLockResume
                            } else {
                                org.dashfoundation.example.services.RegistrationCoordinator
                                    .ReservationKind.Fresh
                            },
                            body = {
                                // Reserve the durable slot before deriving or
                                // persisting any key material.
                                val previews = mgr.identityRegistration.previewRegistrationKeySet(
                                    walletHandle = wallet.handle,
                                    mnemonicResolverHandle = mgr.mnemonicResolverHandle,
                                    identityIndex = identityIndex,
                                    count = RegistrationKeys.keyCount(registerDashPayKeys),
                                )
                                val keySet = DashpayKeyProvisioning.provision(
                                    previews = previews,
                                    includeDashPayKeys = registerDashPayKeys,
                                    walletId = wallet.walletId,
                                    persister = { hex, priv, owner ->
                                        container.walletStorage.storePrivateKey(
                                            hex,
                                            priv,
                                            ownerWalletId = owner,
                                        )
                                    },
                                )
                                when (submission.fundingSource) {
                                    CreateIdentityFundingSource.CoreBalance ->
                                        mgr.identityRegistration.registerWithWalletFunding(
                                            walletHandle = wallet.handle,
                                            amountDuffs = amount
                                                ?: error("A funding amount is required"),
                                            accountIndex = 0,
                                            identityIndex = identityIndex,
                                            keys = keySet.rows,
                                            signerHandle = mgr.signerHandle,
                                            coreSignerHandle = mgr.mnemonicResolverHandle,
                                        )
                                    CreateIdentityFundingSource.PlatformAddress ->
                                        mgr.identityRegistration.registerFromAddresses(
                                            walletHandle = wallet.handle,
                                            identityIndex = identityIndex,
                                            keys = keySet.rows,
                                            signerHandle = mgr.signerHandle,
                                            inputs = platformInputs
                                                ?: error("Platform funding inputs were not prepared"),
                                        )
                                    CreateIdentityFundingSource.ShieldedBalance ->
                                        mgr.shieldedIdentityCreateFromPool(
                                            walletId = wallet.walletId,
                                            identityIndex = identityIndex,
                                            keys = keySet.rows,
                                            denomination = amount
                                                ?: error("A shielded denomination is required"),
                                            fallbackAddress = shieldedFallbackAddress
                                                ?: error("Shielded fallback address was not prepared"),
                                        )
                                    CreateIdentityFundingSource.AssetLockResume -> {
                                        val lock = submission.recoveryLock
                                            ?: error("No resumable registration asset lock is selected")
                                        IdentityAssetLockRecovery.submitRegistrationResume(lock) {
                                            mgr.identityRegistration.resumeWithExistingAssetLock(
                                                walletHandle = wallet.handle,
                                                lock = it,
                                                identityIndex = it.registrationIndex,
                                                keys = keySet,
                                                signerHandle = mgr.signerHandle,
                                                coreSignerHandle = mgr.mnemonicResolverHandle,
                                            )
                                        }
                                    }
                                }
                            },
                            isUnconfirmed = {
                                (it as? org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.ShieldedCreateUnconfirmed)
                                    ?.identityId
                            },
                        )
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
 * both execute; [AssetLockResume] reuses the selected Rust-tracked outpoint
 * through `platform_wallet_resume_identity_with_existing_asset_lock_signer`.
 *
 * @property amountInCredits true when the amount field is denominated in
 *   credits (the Platform-address path spends existing Platform credits);
 *   false when it is duffs (the Core path locks Dash).
 */
enum class CreateIdentityFundingSource(
    val label: String,
    val amountInCredits: Boolean,
) {
    CoreBalance("Core balance", amountInCredits = false),
    PlatformAddress("Platform address", amountInCredits = true),
    ShieldedBalance("Shielded balance", amountInCredits = true),
    AssetLockResume("Resume from asset lock", amountInCredits = false);

    /**
     * Whether this funding path provisions the DashPay ENCRYPTION/DECRYPTION
     * pair alongside the base four keys. Every fresh-funding path does;
     * [AssetLockResume] does NOT — a resumed asset lock funds the fixed key
     * count it was originally built for, so growing the transition it funds
     * risks a resume that fails after the DASH is already locked. Matches iOS,
     * which excludes DashPay provisioning from the resume path. A resumed
     * identity gets DashPay capability afterward via the Add Identity Key flow.
     */
    val includesDashPayKeys: Boolean
        get() = this != AssetLockResume
}

/** Immutable control values captured synchronously when Submit is tapped. */
internal data class CreateIdentitySubmissionSnapshot(
    val fundingSource: CreateIdentityFundingSource,
    val amount: Long?,
    val identityIndex: Int,
    val recoveryLock: TrackedAssetLock?,
) {
    val includesDashPayKeys: Boolean = fundingSource.includesDashPayKeys
}

/**
 * Resolve every mutable form control into one submission value before the
 * first suspension. Resume locks are defensively copied, including the txid
 * bytes, so later UI/state updates cannot redirect the in-flight operation.
 */
internal fun createIdentitySubmissionSnapshot(
    fundingSource: CreateIdentityFundingSource,
    amountText: String,
    identityIndexText: String,
    selectedRecoveryLock: TrackedAssetLock?,
): CreateIdentitySubmissionSnapshot? {
    val lock = if (fundingSource == CreateIdentityFundingSource.AssetLockResume) {
        selectedRecoveryLock?.copy(outpointTxid = selectedRecoveryLock.outpointTxid.copyOf())
            ?: return null
    } else {
        null
    }
    val amount = if (fundingSource == CreateIdentityFundingSource.AssetLockResume) {
        null
    } else {
        amountText.toLongOrNull() ?: return null
    }
    val identityIndex = lock?.registrationIndex
        ?: identityIndexText.toIntOrNull()
        ?: return null
    return CreateIdentitySubmissionSnapshot(
        fundingSource = fundingSource,
        amount = amount,
        identityIndex = identityIndex,
        recoveryLock = lock,
    )
}

/** Credits charged by the active fee schedule before the per-key surcharge. */
private const val IDENTITY_CREATE_BASE_COST_CREDITS = 2_000_000L

/** Credits charged for each public key in an identity-create transition. */
private const val IDENTITY_KEY_CREATION_COST_CREDITS = 6_500_000L

/** Credits charged for every Platform-address funding input. */
private const val ADDRESS_FUNDING_INPUT_COST_CREDITS = 500_000L

/** Core asset-lock floor required before identity-create processing starts. */
private const val IDENTITY_ASSET_LOCK_BASE_DUFFS = 200_000L

private const val CREDITS_PER_DUFF = 1_000L

/**
 * Protocol minimum for a Core-funded identity create with [keyCount] keys.
 * Mirrors `IdentityCreateTransition::calculate_min_required_fee_v1` and the
 * iOS `CreateIdentityView.minFundingDuffs(forKeyCount:)` parity reference.
 */
internal fun minimumCoreFundingDuffsForKeyCount(keyCount: Int): Long {
    require(keyCount > 0) { "keyCount must be positive, got $keyCount" }
    return IDENTITY_ASSET_LOCK_BASE_DUFFS +
        (IDENTITY_CREATE_BASE_COST_CREDITS +
            IDENTITY_KEY_CREATION_COST_CREDITS * keyCount) / CREDITS_PER_DUFF
}

/**
 * Protocol fee for an address-funded identity create without a refund output.
 * Mirrors `IdentityCreateFromAddressesTransition::calculate_min_required_fee`.
 */
internal fun minimumPlatformAddressFundingFeeCredits(
    keyCount: Int,
    inputCount: Int,
): Long {
    require(keyCount > 0) { "keyCount must be positive, got $keyCount" }
    require(inputCount > 0) { "inputCount must be positive, got $inputCount" }
    return IDENTITY_CREATE_BASE_COST_CREDITS +
        IDENTITY_KEY_CREATION_COST_CREDITS * keyCount +
        ADDRESS_FUNDING_INPUT_COST_CREDITS * inputCount
}

/** Single submit/UI gate for the active funding source's amount semantics. */
internal fun isCreateIdentityFundingAmountValid(
    source: CreateIdentityFundingSource,
    amount: Long?,
): Boolean = when (source) {
    CreateIdentityFundingSource.CoreBalance -> {
        val minimum = minimumCoreFundingDuffsForKeyCount(
            RegistrationKeys.keyCount(source.includesDashPayKeys),
        )
        amount != null && amount >= minimum
    }
    CreateIdentityFundingSource.PlatformAddress,
    CreateIdentityFundingSource.ShieldedBalance,
    -> amount != null && amount > 0
    CreateIdentityFundingSource.AssetLockResume -> true
}

/**
 * Protocol version at which the shielded exit-denomination set was revised:
 * 0.03 and 0.25 DASH added, 0.3 DASH retired.
 */
private const val SHIELDED_DENOMINATION_REVISION_VERSION = 13

/**
 * The fixed exit denominations (in CREDITS) a Type-20 shielded identity
 * create may spend from the pool, before protocol version 13 — 0.1 / 0.3 /
 * 0.5 / 1.0 DASH. Source of truth: `shielded_identity_create_denominations`
 * in DPP; a submitted denomination not in the on-chain set is rejected at
 * validation (← iOS `CreateIdentityView.shieldedDenominationsPreV13`).
 */
private val SHIELDED_IDENTITY_DENOMINATIONS_PRE_V13: List<Pair<Long, String>> = listOf(
    10_000_000_000L to "0.1 DASH",
    30_000_000_000L to "0.3 DASH",
    50_000_000_000L to "0.5 DASH",
    100_000_000_000L to "1.0 DASH",
)

/**
 * The revised set active from protocol version 13 — 0.03 / 0.1 / 0.25 /
 * 0.5 / 1.0 DASH (← iOS `CreateIdentityView.shieldedDenominationsV13`).
 */
private val SHIELDED_IDENTITY_DENOMINATIONS_V13: List<Pair<Long, String>> = listOf(
    3_000_000_000L to "0.03 DASH",
    10_000_000_000L to "0.1 DASH",
    25_000_000_000L to "0.25 DASH",
    50_000_000_000L to "0.5 DASH",
    100_000_000_000L to "1.0 DASH",
)

/**
 * The denomination set to offer, gated on the network's reported protocol
 * version ([org.dashfoundation.example.state.AppState.platformProtocolVersion]).
 * Until v13 activates the network only accepts the pre-v13 set, and after
 * activation 0.3 DASH is rejected — so the picker must track the live
 * version. An unknown version (refresh pending or failed) falls back to the
 * pre-v13 set, matching every currently deployed network.
 */
private fun shieldedIdentityDenominations(protocolVersion: Int?): List<Pair<Long, String>> =
    if (protocolVersion != null && protocolVersion >= SHIELDED_DENOMINATION_REVISION_VERSION) {
        SHIELDED_IDENTITY_DENOMINATIONS_V13
    } else {
        SHIELDED_IDENTITY_DENOMINATIONS_PRE_V13
    }

/**
 * Funding source + amount — the Swift `CreateIdentityView` funding/amount
 * sections, including an existing-lock picker sourced from Rust rather than
 * inferred from Room rows.
 */
@Composable
private fun FundingSection(
    source: CreateIdentityFundingSource,
    onSourceChange: (CreateIdentityFundingSource) -> Unit,
    amountText: String,
    onAmountChange: (String) -> Unit,
    recoveryLocks: List<TrackedAssetLock>,
    recoveryLoadError: String?,
    selectedRecoveryLock: TrackedAssetLock?,
    onRecoveryLockChange: (TrackedAssetLock) -> Unit,
    shieldedDenominations: List<Pair<Long, String>>,
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
        if (source == CreateIdentityFundingSource.AssetLockResume) {
            if (recoveryLoadError != null) {
                Text(
                    recoveryLoadError,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("createIdentity.resume.error"),
                )
            } else if (recoveryLocks.isEmpty()) {
                Text(
                    "No Rust-tracked registration locks are currently resumable.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("createIdentity.resume.empty"),
                )
            } else {
                AccessiblePicker(
                    label = "Existing asset lock",
                    options = recoveryLocks,
                    selected = selectedRecoveryLock ?: recoveryLocks.first(),
                    optionLabel = IdentityAssetLockRecovery::label,
                    testTag = "createIdentity.resume.outpoint",
                    onSelected = onRecoveryLockChange,
                )
                Text(
                    "Resumes this exact outpoint. Built locks rebroadcast their existing " +
                        "transaction; no replacement funding transaction is created.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else if (source == CreateIdentityFundingSource.ShieldedBalance) {
            // Type-20 spends a FIXED exit denomination, not a free-form amount
            // (← iOS's denomination Picker). The picked value flows back through
            // [amountText] (in credits) so the submit path reads it uniformly.
            val selectedDenom = shieldedDenominations
                .firstOrNull { it.first.toString() == amountText }
                ?: shieldedDenominations.first()
            org.dashfoundation.example.ui.components.AccessiblePicker(
                label = "Denomination",
                options = shieldedDenominations,
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
 * Select the smallest feasible Platform-address input set that contributes
 * exactly [target] credits while leaving the full identity-create fee on the
 * address that Rust's default `DeductFromInput(0)` strategy will use.
 *
 * The native FFI collects inputs into a `BTreeMap<PlatformAddress, Credits>`,
 * so input 0 is the lowest `(addressType, unsigned hash bytes)` address, not
 * the first item in this list. Each attempted fee payer therefore selects only
 * lexicographically greater companion inputs. The payer keeps
 * [minimumPlatformAddressFundingFeeCredits] after its requested spend; all
 * selected inputs contribute at least one credit. Returns an empty list when
 * the requested identity balance plus fee cannot be funded.
 */
internal fun packFundingInputs(
    candidates: List<FundingInput>,
    target: Long,
    keyCount: Int = RegistrationKeys.keyCount(includeDashPayKeys = true),
): List<FundingInput> {
    if (target <= 0) return emptyList()

    val byAddress = candidates.sortedWith(::comparePlatformAddresses)
    for (inputCount in 1..byAddress.size) {
        if (target < inputCount) break // every encoded input must spend >= 1 credit
        val fee = minimumPlatformAddressFundingFeeCredits(keyCount, inputCount)

        for (payerIndex in byAddress.indices) {
            val payer = byAddress[payerIndex]
            val payerCapacity = payer.credits - fee
            if (payerCapacity < 1) continue

            // Only greater addresses may accompany this payer, otherwise the
            // BTreeMap would choose a different input 0 for fee deduction.
            val companions = byAddress
                .subList(payerIndex + 1, byAddress.size)
                .sortedByDescending { it.credits }
                .take(inputCount - 1)
            if (companions.size != inputCount - 1) continue

            var totalCapacity = payerCapacity
            for (companion in companions) {
                totalCapacity = saturatingAdd(totalCapacity, companion.credits)
            }
            if (totalCapacity < target) continue

            val selected = listOf(payer) + companions
            val spends = LongArray(inputCount) { 1L }
            var remaining = target - inputCount

            // Spend companions first so the BTreeMap-first payer retains as
            // much of its fee-bearing balance as possible.
            for (index in 1 until selected.size) {
                val extra = minOf(selected[index].credits - 1, remaining)
                spends[index] += extra
                remaining -= extra
            }
            val payerExtra = minOf(payerCapacity - 1, remaining)
            spends[0] += payerExtra
            remaining -= payerExtra
            check(remaining == 0L) { "feasible funding allocation left $remaining credits" }

            return selected.mapIndexed { index, input -> input.copy(credits = spends[index]) }
        }
    }
    return emptyList()
}

private fun comparePlatformAddresses(left: FundingInput, right: FundingInput): Int {
    val typeOrder = left.addressType.compareTo(right.addressType)
    if (typeOrder != 0) return typeOrder
    for (index in left.hash.indices) {
        val byteOrder = left.hash[index].toUByte().compareTo(right.hash[index].toUByte())
        if (byteOrder != 0) return byteOrder
    }
    return 0
}

private fun saturatingAdd(left: Long, right: Long): Long =
    if (Long.MAX_VALUE - left < right) Long.MAX_VALUE else left + right

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
