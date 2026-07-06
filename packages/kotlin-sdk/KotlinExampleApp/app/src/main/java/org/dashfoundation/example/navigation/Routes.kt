package org.dashfoundation.example.navigation

import kotlinx.serialization.Serializable

/**
 * Type-safe navigation routes. The five tab-root routes mirror `RootTab` in
 * `ContentView.swift`; detail routes accumulate per feature milestone.
 * Binary IDs travel as hex strings (nav args must be primitives).
 */

// ── Tab roots ──────────────────────────────────────────────────────────

@Serializable object SyncHome

@Serializable object WalletsHome

@Serializable object IdentitiesHome

@Serializable object ContractsHome

@Serializable object DashPayHome

@Serializable object SettingsHome

// ── Settings graph ─────────────────────────────────────────────────────

@Serializable object DataManagement

// ── Wallets graph ──────────────────────────────────────────────────────

@Serializable object CreateWallet

@Serializable data class WalletDetail(val walletIdHex: String)

/**
 * Post-create seed backup + confirmation quiz (← `SeedBackupView.swift`).
 * Only the wallet id travels as a nav arg; the phrase is re-read from
 * [org.dashfoundation.dashsdk.security.WalletStorage] on-screen so it
 * never rides navigation state.
 */
@Serializable data class SeedBackup(val walletIdHex: String)

/** One account's addresses/UTXOs (← `AccountDetailView.swift`). */
@Serializable data class AccountDetail(val walletIdHex: String, val accountId: Long)

/** Send form (← `SendTransactionView.swift` + `SendViewModel.swift`). */
@Serializable data class SendTransaction(val walletIdHex: String)

/** Per-wallet transaction timeline (← `TransactionListView.swift`). */
@Serializable data class WalletTransactions(val walletIdHex: String)

/** One transaction's fields (← `TransactionDetailView.swift`). */
@Serializable data class WalletTransactionDetail(val txidHex: String)

/**
 * Camera QR scanner (← `QRScannerView.swift`). Returns the raw scanned
 * string to the caller via `previousBackStackEntry.savedStateHandle`
 * under [QrScanner.RESULT_KEY].
 */
@Serializable object QrScanner {
    const val RESULT_KEY: String = "scannedQr"
}

// ── Identities graph ───────────────────────────────────────────────────

/** Create-identity flow (← `CreateIdentityView.swift`). */
@Serializable object CreateIdentity

/** Load an existing identity by id (← `LoadIdentityView.swift`). */
@Serializable object LoadIdentity

/** Re-scan a wallet's identity tree (← `SearchWalletsForIdentitiesView.swift`). */
@Serializable object SearchWalletsForIdentities

/** DPNS availability tester (← `DPNSTestView.swift`). */
@Serializable object DpnsTest

/** One identity's detail (← `IdentityDetailView.swift`). */
@Serializable data class IdentityDetail(val identityIdHex: String)

/**
 * Live registration progress for a slot (← `RegistrationProgressView.swift`).
 * Resolves its controller from the coordinator by `(walletId, identityIndex)`;
 * dismissal-safe (the controller outlives this screen).
 */
@Serializable data class RegistrationProgress(val walletIdHex: String, val identityIndex: Int)

/** Register a DPNS name for an identity (← `RegisterNameView.swift`). */
@Serializable data class RegisterName(val identityIdHex: String)

/** Pick the identity's main DPNS name (← `SelectMainNameView.swift`). */
@Serializable data class SelectMainName(val identityIdHex: String)

/** All public keys of an identity (← `KeysListView.swift`). */
@Serializable data class KeysList(val identityIdHex: String)

/** One public key's detail + private-key reveal (← `KeyDetailView.swift`). */
@Serializable data class KeyDetail(val identityIdHex: String, val keyId: Int)

/** Add a public key to an identity (← `AddIdentityKeyView.swift`). */
@Serializable data class AddIdentityKey(val identityIdHex: String)

// ── Credits graph (← the credit-action rows on IdentityDetailView) ─────

/** Top up an identity from Platform addresses (← `TopUpIdentityView.swift`). */
@Serializable data class TopUpIdentity(val identityIdHex: String)

/** Top up an identity from a new Core asset lock (ID-05). */
@Serializable data class TopUpIdentityFromCore(val identityIdHex: String)

/** Transfer credits identity → identity (← `TransferCreditsView.swift`). */
@Serializable data class TransferCredits(val identityIdHex: String)

/** Transfer credits identity → Platform addresses (ID-11). */
@Serializable data class TransferToAddress(val identityIdHex: String)

/** Withdraw credits identity → L1 address (← `WithdrawCreditsView.swift`). */
@Serializable data class WithdrawCredits(val identityIdHex: String)

// ── State-transition catalog (← StateTransitions/PlatformStateTransitions) ──

/** The transition catalog root (← `StateTransitionsView.swift`). */
@Serializable object StateTransitions

/** One category's transition list (← `TransitionCategoryView.swift`). */
@Serializable data class TransitionCategoryRoute(val categoryName: String)

/** One transition's dynamic input form (← `TransitionDetailView.swift`). */
@Serializable data class TransitionDetailRoute(val transitionKey: String)

// ── Asset-lock funding graph (← FundFromAssetLockPlatformAddressView) ──

/** Fund a Platform address from an asset lock (← `FundFromAssetLockPlatformAddressView.swift`). */
@Serializable data class FundFromAssetLock(val walletIdHex: String)

/**
 * Wallet-signed transfer of Platform-address (DIP-17) credits
 * (← `TransferPlatformAddressView.swift`, ADDR-02). Wallet-scoped.
 */
@Serializable data class TransferPlatformAddress(val walletIdHex: String)

/**
 * Wallet-signed withdrawal of Platform-address (DIP-17) credits to a Core
 * L1 address (← `WithdrawPlatformAddressView.swift`, ADDR-04). Wallet-scoped.
 */
@Serializable data class WithdrawPlatformAddress(val walletIdHex: String)

/**
 * Live address-funding progress (← `AddressFundFromAssetLockProgressView.swift`).
 * Resolves its controller from the coordinator by
 * `(walletId, platformAccountIndex, recipientHash)`; dismissal-safe.
 */
@Serializable data class AddressFundProgress(
    val walletIdHex: String,
    val platformAccountIndex: Int,
    val recipientHashHex: String,
)

// ── Shielded funding graph (gated on Sdk.hasShielded) ──────────────────

/** Seed the shielded note pool (← `SeedShieldedPoolView.swift`). */
@Serializable data class SeedShieldedPool(val walletIdHex: String)

/** Shield funds from an asset lock (← `ShieldedFundFromAssetLockView.swift`). */
@Serializable data class ShieldedFund(val walletIdHex: String)

/**
 * Live shielded-funding progress (← `ShieldedFundFromAssetLockProgressView.swift`).
 * Resolves its controller from the coordinator by `(walletId, recipientRaw43)`.
 */
@Serializable data class ShieldedFundProgress(
    val walletIdHex: String,
    val recipientRaw43Hex: String,
)

/** Shielded activity timeline for a wallet (← `ShieldedActivityView.swift`). */
@Serializable data class ShieldedActivity(val walletIdHex: String)

// ── DashPay graph (hosted under the DashPay tab, ← the DashPay/ views) ──

/** Established-contacts list for an identity (← `ContactsView.swift`). */
@Serializable data class DashPayContacts(val identityIdHex: String)

/** Incoming + outgoing contact requests (← `ContactRequestsView.swift`). */
@Serializable data class DashPayRequests(val identityIdHex: String)

/** Add-a-contact form (DPNS search / paste id / QR, ← `AddContactView.swift`). */
@Serializable data class DashPayAddContact(val identityIdHex: String)

/** One contact's detail + payments (← `ContactDetailView.swift`). */
@Serializable data class DashPayContactDetail(
    val identityIdHex: String,
    val contactIdHex: String,
)

/** Read-only own-profile sheet (← `DashPayProfileView.swift`). */
@Serializable data class DashPayProfile(val identityIdHex: String)

/** Ignored-senders list (← `IgnoredContactsView.swift`). */
@Serializable data class DashPayIgnored(val identityIdHex: String)

/** Hidden established-contacts list (← `HiddenContactsView.swift`). */
@Serializable data class DashPayHidden(val ownerIdentityIdHex: String)

// ── Contracts graph ────────────────────────────────────────────────────

/** Fetch-a-contract screen (← `LocalDataContractsView.swift`). */
@Serializable object LocalContracts

/**
 * Manual JSON-editor register-a-contract screen for an owner identity
 * (← `RegisterContractSourceView.swift`).
 */
@Serializable data class RegisterContractSource(val identityIdHex: String)

/** Saved-contract details (← `DataContractDetailsView.swift`). */
@Serializable data class ContractDetail(val contractIdHex: String)

/** Document-type schema drill-in (← `DocumentTypeDetailsView.swift`). */
@Serializable data class DocumentTypeDetail(val contractIdHex: String, val typeName: String)

/** Document query builder + results (query role of `DocumentsView.swift`). */
@Serializable data class Documents(val contractIdHex: String, val typeName: String)

/**
 * Schema-driven create + broadcast form for a new document of this type
 * (← `CreateDocumentView` + `DocumentFieldsView` in `DocumentsView.swift`).
 * Reached from `DocumentTypeDetailsScreen`'s "New Document" action, so the
 * contract + document type are fixed.
 */
@Serializable data class NewDocument(val contractIdHex: String, val typeName: String)

/**
 * Single fetched document, pretty-printed key/value (viewer role of
 * `DocumentFieldsView.swift`). The document JSON travels in the route —
 * Platform documents are small JSON objects, well under the nav-args
 * transaction limit.
 */
@Serializable data class DocumentFields(val documentJson: String)

/** Count aggregate form (← `CountDocumentsView.swift`). */
@Serializable data class CountDocuments(val contractIdHex: String, val typeName: String)

/**
 * Price probe + purchase / set-price for one document
 * (← `DocumentWithPriceView.swift` + the `PurchaseDocumentView` /
 * set-price flows in `DocumentsView.swift`). [documentId] may arrive
 * empty (the transition-catalog entry lets the user type one).
 */
@Serializable data class DocumentWithPrice(
    val contractIdHex: String,
    val typeName: String,
    val documentId: String = "",
)

/** Sum/average aggregate form (← `SumAverageDocumentsView.swift`). */
@Serializable data class SumAverageDocuments(val contractIdHex: String, val typeName: String)

/** List of runnable platform queries (← `PlatformQueriesView.swift` role). */
@Serializable object QueriesList

/** One query's parameters + result (← `QueryDetailView.swift`). */
@Serializable data class QueryDetail(val queryName: String)

/** Raw GroveDB path-elements query (← `GroveDBPathElementsView.swift`). */
@Serializable object GroveDbPathElements

// ── Tokens graph (hosted under the Contracts tab, ← ContractsTabView) ──

/** Identity-scoped balances + network token list (← `TokensView.swift`). */
@Serializable object TokensHome

/** Search + capability filter chips (← `TokenSearchView.swift`). */
@Serializable object TokenSearch

/** Streamlined single-token contract form (← `QuickBasicTokenView.swift`). */
@Serializable object QuickBasicToken

/**
 * One token's full configuration (← `TokenDetailsView.swift`).
 * [tokenIdHex] is the synthetic `contractId || position.bigEndian` row id.
 */
@Serializable data class TokenDetail(val tokenIdHex: String)

/**
 * "What can this identity do with this token?" rows + control-rule
 * viewer (← `TokenActionPermissionsView.swift`). [identityIdHex] pins the
 * acting identity; null shows the picker.
 */
@Serializable data class TokenActionPermissions(
    val tokenIdHex: String,
    val identityIdHex: String? = null,
)

/**
 * One of the 12 token action forms (← the `TokenActions` Swift views).
 * [actionId] is a [org.dashfoundation.example.services.tokens.TokenActionKind] id.
 */
@Serializable data class TokenAction(
    val actionId: String,
    val tokenIdHex: String,
    val identityIdHex: String,
)

/** Open group-action proposals (← `PendingGroupActionsView.swift`). */
@Serializable data class PendingGroupActions(
    val tokenIdHex: String,
    val identityIdHex: String,
)

/**
 * Review-and-sign one proposal (← `CoSignProposalView.swift`). The
 * proposal element JSON travels in the route (same pattern as
 * [DocumentFields]); [groupPosition] is the group contract position the
 * proposal was discovered under.
 */
@Serializable data class CoSignProposal(
    val tokenIdHex: String,
    val identityIdHex: String,
    val groupPosition: Int,
    val proposalJson: String,
)

// ── Diagnostics graph (← OptionsView's Data + Platform sections) ───────

/**
 * Address balance/nonce queries (← `AddressQueriesView.swift`). The
 * backing reads (`dash_sdk_address_fetch_info`,
 * `dash_sdk_addresses_fetch_infos`) are not bridged yet; the screen
 * renders the forms plus the locally-synced Platform addresses and
 * name-defers execution.
 */
@Serializable object AddressQueries

/** DAPI address ban list (← `BannedAddressesView.swift`). */
@Serializable object BannedAddresses

/**
 * DPNS contest detail (← `ContestDetailView.swift`). [identityIdHex] is
 * the viewing identity ("You" badging on iOS); empty when opened outside
 * an identity context.
 */
@Serializable data class ContestDetail(
    val contestName: String,
    val identityIdHex: String = "",
)

/**
 * One contract group's members + live proposals (← `GroupDetailView.swift`).
 * [position] is the group contract position inside the contract's
 * `groups` map.
 */
@Serializable data class GroupDetail(val contractIdHex: String, val position: Int)

/** Query test-runner + environment snapshot (← `DiagnosticsView.swift`). */
@Serializable object Diagnostics

/** Live wallet-manager memory state (← `WalletMemoryExplorerView.swift`). */
@Serializable object WalletMemoryExplorer

/** Keystore-backed secret listing (← `KeychainExplorerView.swift`). */
@Serializable object KeystoreExplorer

// ── Storage explorer graph ─────────────────────────────────────────────

/** All tables with live counts (← `StorageExplorerView.swift`). */
@Serializable object StorageExplorer

/** Rows of one table (← `StorageModelListViews.swift`). */
@Serializable data class StorageModelList(val modelName: String)

/** All fields of one row (← `StorageRecordDetailViews.swift`). */
@Serializable data class StorageRecordDetail(val modelName: String, val rowId: Long)

/** The five bottom-bar tabs, in iOS order. */
enum class RootTab(
    val route: Any,
    val routeClass: kotlin.reflect.KClass<*>,
    val testTag: String,
) {
    SYNC(SyncHome, SyncHome::class, "rootTab.sync"),
    WALLETS(WalletsHome, WalletsHome::class, "rootTab.wallets"),
    IDENTITIES(IdentitiesHome, IdentitiesHome::class, "rootTab.identities"),
    DASHPAY(DashPayHome, DashPayHome::class, "rootTab.dashpay"),
    SETTINGS(SettingsHome, SettingsHome::class, "rootTab.settings"),
}
