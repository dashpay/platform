# SwiftExampleApp → KotlinExampleApp Parity Checklist

One row per Swift view in
`packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/**` and
`.../Core/Views/`. Android paths are relative to
`packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/`.

Status legend:

- **ported** — feature-equivalent on Android (UI shape may be Material,
  testTags reuse the iOS accessibility identifiers).
- **partial** — screen exists and is navigable; the noted piece is
  missing (usually a submit path surfacing the named-missing-export
  dialog).
- **deferred** — not built (or built as a stub) because the backing FFI
  is not bridged into `rs-unified-sdk-jni`; the missing export is named.

## Views/

| Swift file | Android file / route | Status |
| --- | --- | --- |
| AddIdentityKeyView.swift | — | deferred — identity add-key state transition (`dash_sdk_identity_*` update path) not bridged |
| AddressFundFromAssetLockProgressView.swift | ui/funding/AddressFundProgressScreen.kt · `AddressFundProgress` | ported (controller/coordinator dismissal-safe like iOS) |
| AddressQueriesView.swift | ui/diagnostics/AddressQueriesScreen.kt · `AddressQueries` | partial — forms + local Platform-address browser live; execution deferred on `dash_sdk_address_fetch_info` / `dash_sdk_addresses_fetch_infos` |
| BannedAddressesView.swift | ui/diagnostics/BannedAddressesScreen.kt · `BannedAddresses` | deferred — `platform_wallet_manager_address_ban_info` not bridged; screen renders the iOS empty-state semantics and names the export |
| ContestDetailView.swift | ui/identity/ContestDetailScreen.kt · `ContestDetail` | deferred — contest shape + About rendered; vote state needs `platform_wallet_fetch_contest_vote_state`, casting needs `dash_sdk_contested_resource_cast_vote` |
| ContractsTabView.swift | ui/contracts/ContractsHomeScreen.kt · `ContractsHome` | ported |
| CountDocumentsView.swift | ui/contracts/CountDocumentsScreen.kt · `CountDocuments` | ported |
| CreateIdentityView.swift | ui/identity/CreateIdentityScreen.kt · `CreateIdentity` | ported |
| DPNSTestView.swift | ui/identity/DpnsTestScreen.kt · `DpnsTest` | ported |
| DataContractDetailsView.swift | ui/contracts/DataContractDetailsScreen.kt · `ContractDetail` | ported (incl. group drill-in) |
| DiagnosticsView.swift | ui/diagnostics/DiagnosticsScreen.kt · `Diagnostics` | partial — runs the 9 bridged registry queries against the shared testnet fixtures (iOS runs ~40); adds environment / sync-state / DB-count sections |
| DocumentFieldsView.swift | ui/contracts/DocumentFieldsScreen.kt · `DocumentFields` | ported |
| DocumentTypeDetailsView.swift | ui/contracts/DocumentTypeDetailsScreen.kt · `DocumentTypeDetail` | ported |
| DocumentWithPriceView.swift | — | deferred — document purchase / set-price transitions not bridged |
| DocumentsView.swift | ui/contracts/DocumentsScreen.kt · `Documents` | ported (query role; viewer role in DocumentFieldsScreen) |
| FriendsView.swift | ui/identity/FriendsScreen.kt · `Friends` | ported — full `loadFriends()` hydration: network sync (`platform_wallet_sync_contact_requests` / `platform_wallet_fetch_sent_contact_requests`) + managed-identity id enumeration (`platform_wallet_get_managed_identity` → `managed_identity_get_{incoming,sent}_contact_request_ids` / `..._established_contact_ids`, Room fallback); send / reject / accept all wired (accept via `managed_identity_get_incoming_contact_request` + the bridged `platform_wallet_accept_contact_request_with_signer`) |
| FundFromAssetLockPlatformAddressView.swift | ui/funding/FundFromAssetLockScreen.kt · `FundFromAssetLock` | ported — submit picks a fresh unused Platform address and funds via the now-bridged `platform_address_wallet_fund_from_asset_lock_signer` (+ resume variant on `ManagedPlatformWallet`); coordinator/progress/pending list drive the flow |
| TransferPlatformAddressView.swift (ADDR-02, #3923) | ui/credits/TransferPlatformAddressScreen.kt · `TransferPlatformAddress` | ported — wallet-signed DIP-17 credit transfer via the now-bridged `platform_address_wallet_transfer` (`ManagedPlatformWallet.transferCredits`, AUTO selection, null inputs/fee-strategy); source account + destination (own-wallet / external P2PKH hash) + amount only; gate reads version-locked `minInput`/`minOutput` via `walletPlatformAddressMinAmounts`. Launched from WalletDetailScreen's Platform Credits section |
| WithdrawPlatformAddressView.swift (ADDR-04, #3923) | ui/credits/WithdrawPlatformAddressScreen.kt · `WithdrawPlatformAddress` | ported — wallet-signed full-balance DIP-17 withdrawal to a Core L1 address via the now-bridged `platform_address_wallet_withdraw_to_address` (`ManagedPlatformWallet.withdrawCredits`); submit gated on `platform_address_wallet_preflight_withdrawal` (`preflightWithdrawal`, off the main thread on `Dispatchers.IO`); Fibonacci fee-rate picker mirrors `WithdrawalCoreFeeRates`. Launched from WalletDetailScreen's Platform Credits section |
| GroupDetailView.swift | ui/contracts/GroupDetailScreen.kt · `GroupDetail` | ported — members resolved against local identities; adds live open-proposals via bridged `Groups.pendingActions` |
| GroveDBPathElementsView.swift | — | deferred — `dash_sdk_system_get_path_elements` not bridged |
| IdentitiesView.swift | ui/identity/IdentitiesHomeScreen.kt · `IdentitiesHome` | ported |
| IdentityDetailView.swift | ui/identity/IdentityDetailScreen.kt · `IdentityDetail` | partial — contested-name rows absent (contested-username discovery `dash_sdk_dpns_get_contested_usernames_by_identity` not bridged); everything else ported |
| IdentityKeyAddition.swift | — | deferred — same missing add-key transition as AddIdentityKeyView |
| KeyDetailView.swift | ui/identity/KeyDetailScreen.kt · `KeyDetail` | ported |
| KeychainExplorerView.swift | ui/diagnostics/KeystoreExplorerScreen.kt · `KeystoreExplorer` | ported (adapted: WalletStorage entries masked + AndroidKeyStore aliases; adds biometric-gated mnemonic reveal) |
| KeysListView.swift | ui/identity/KeysListScreen.kt · `KeysList` | ported |
| LoadIdentityView.swift | ui/identity/LoadIdentityScreen.kt · `LoadIdentity` | ported |
| LocalDataContractsView.swift | ui/contracts/LocalDataContractsScreen.kt · `LocalContracts` | ported |
| OptionsView.swift | ui/settings/SettingsScreen.kt · `SettingsHome` (+ AboutSheet.kt) | ported — Network/SPV/Data/Platform/About sections incl. About bottom sheet |
| PendingPlatformFundFromAssetLocksList.swift | ui/funding/AddressFundProgressScreen.kt (`PendingAssetLocksList`) | ported (embedded in IdentitiesHome, matching iOS) |
| PendingRegistrationsList.swift | ui/identity/RegistrationProgressScreen.kt (`PendingRegistrationRow`) | ported (embedded in IdentitiesHome) |
| PlatformQueriesView.swift | ui/contracts/QueryDetailScreen.kt (`QueriesListScreen`) · `QueriesList` | partial — flat registry list instead of 12 categories; 9 bridged queries + Address Queries / Run All Queries entries; ~40 iOS queries wait on their JNI exports |
| PlatformStateTransitionsView.swift | ui/transitions/StateTransitionsScreen.kt · `StateTransitions` | ported (iOS file is a trivial wrapper around StateTransitionsView) |
| QueryDetailView.swift | ui/contracts/QueryDetailScreen.kt · `QueryDetail` | ported for the bridged registry |
| QuickBasicTokenView.swift | ui/tokens/QuickBasicTokenScreen.kt · `QuickBasicToken` | partial — form wired; contract-registration submit deferred (data-contract create transition not bridged) |
| RecoverWalletsSheet.swift | ui/wallet/RecoverWalletsFlow.kt | ported |
| RegisterContractSourceView.swift | — | deferred — data-contract create/register transition not bridged |
| RegisterNameView.swift | ui/identity/RegisterNameScreen.kt · `RegisterName` | ported — availability check + registration live; contest-status badge deferred (`dash_sdk_dpns_is_contested_username`), drill-in to ContestDetail wired |
| RegistrationProgressView.swift | ui/identity/RegistrationProgressScreen.kt · `RegistrationProgress` | ported |
| SearchWalletsForIdentitiesView.swift | ui/identity/SearchWalletsForIdentitiesScreen.kt · `SearchWalletsForIdentities` | ported |
| SeedShieldedPoolView.swift | ui/shielded/SeedShieldedPoolScreen.kt · `SeedShieldedPool` | ported — submit wired to `platform_wallet_manager_shielded_seed_pool_notes` (via `PlatformWalletManager.seedShieldedPoolNotes`) with the per-batch progress callback; idle→in-flight→completed/failed phase machine + dismissal gate mirror Swift |
| SelectMainNameView.swift | ui/identity/SelectMainNameScreen.kt · `SelectMainName` | ported |
| ShieldedFundFromAssetLockProgressView.swift | ui/shielded/ShieldedFundProgressScreen.kt · `ShieldedFundProgress` | ported |
| ShieldedFundFromAssetLockView.swift | ui/shielded/ShieldedFundScreen.kt · `ShieldedFund` | ported — submit wired to `platform_wallet_manager_shielded_fund_from_asset_lock` (via `PlatformWalletManager.shieldedFundFromAssetLock`); recipient defaults to the wallet's bound shielded address (`platform_wallet_manager_shielded_default_address`) or a pasted 43-byte override; runs through the coordinator/controller/progress flow (resume variant on `shieldedResumeFundFromAssetLock` also bridged) |
| StateTransitionsView.swift | ui/transitions/StateTransitionsScreen.kt · `StateTransitions` | ported |
| StorageExplorerView.swift | ui/storage/StorageExplorerScreen.kt · `StorageExplorer` | ported |
| StorageModelListViews.swift | ui/storage/StorageModelListScreen.kt · `StorageModelList` | ported |
| StorageRecordDetailViews.swift | ui/storage/StorageRecordDetailScreen.kt · `StorageRecordDetail` | ported |
| SumAverageDocumentsView.swift | ui/contracts/SumAverageDocumentsScreen.kt · `SumAverageDocuments` | ported |
| TokenActionPermissionsView.swift | ui/tokens/TokenActionPermissionsScreen.kt · `TokenActionPermissions` | ported — live balance (`calculateTokenId` + `getIdentityTokenBalances`) + on-chain pause reconciliation (`getTokenStatuses`) now bridged and wired, with persisted fallback |
| TokenDetailsView.swift | ui/tokens/TokenDetailsScreen.kt · `TokenDetail` | ported |
| TokenSearchView.swift | ui/tokens/TokenSearchScreen.kt · `TokenSearch` | ported |
| TokensView.swift | ui/tokens/TokensScreen.kt · `TokensHome` | ported |
| TopUpIdentityView.swift | ui/credits/TopUpIdentityScreen.kt · `TopUpIdentity` | ported — funding-input enumeration now bridged (`walletAddressesWithBalances` → `platform_address_wallet_addresses_with_balances`); submit greedily packs balance-carrying addresses and credits via the top-up FFI |
| TransferCreditsView.swift | ui/credits/TransferCreditsScreen.kt · `TransferCredits` | ported |
| TransitionCategoryView.swift | ui/transitions/TransitionCategoryScreen.kt · `TransitionCategoryRoute` | ported |
| TransitionDetailView.swift | ui/transitions/TransitionDetailScreen.kt · `TransitionDetailRoute` | partial — dynamic forms ported; unbridged transitions surface the named-missing-export dialog on submit |
| TransitionInputView.swift | ui/transitions/TransitionDetailScreen.kt (input rows) | ported (component folded into the detail screen) |
| WalletMemoryExplorerView.swift | ui/diagnostics/WalletMemoryExplorerScreen.kt · `WalletMemoryExplorer` | partial — wallets map, balances, SPV progress/tip, `is*SyncRunning` liveness live; per-wallet drill-downs deferred on `platform_wallet_manager_*` snapshot exports |
| WithdrawCreditsView.swift | ui/credits/WithdrawCreditsScreen.kt · `WithdrawCredits` | ported |

## Views/Components/

| Swift file | Android file | Status |
| --- | --- | --- |
| AccessiblePicker.swift | ui/components/AccessiblePicker.kt | ported |
| RecipientPickerView.swift | ui/components/RecipientPicker.kt | ported — all three source modes (Identities / DPNS via bridged `dpnsResolve` / Manual) |

## Views/TokenActions/

| Swift file | Android file / route | Status |
| --- | --- | --- |
| CoSignProposalView.swift | ui/tokens/CoSignProposalScreen.kt · `CoSignProposal` | ported |
| PendingGroupActionsView.swift | ui/tokens/PendingGroupActionsScreen.kt · `PendingGroupActions` | ported |
| TokenBurnActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(burn)` | ported |
| TokenClaimActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(claim)` | ported |
| TokenDestroyFrozenFundsActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(destroyFrozenFunds)` | ported |
| TokenFreezeActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(freeze)` | ported |
| TokenMintActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(mint)` | ported |
| TokenPauseActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(pause)` | ported |
| TokenPurchaseActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(directPurchase)` | ported |
| TokenResumeActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(resume)` | ported |
| TokenSetPriceActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(setPrice)` | ported |
| TokenTransferActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(transfer)` | ported |
| TokenUnfreezeActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(unfreeze)` | ported |
| TokenUpdateMaxSupplyActionView.swift | ui/tokens/TokenActionScreens.kt · `TokenAction(updateMaxSupply)` | ported |

## Core/Views/

| Swift file | Android file / route | Status |
| --- | --- | --- |
| AccountDetailView.swift | ui/wallet/AccountDetailScreen.kt · `AccountDetail` | ported |
| AccountListView.swift | ui/wallet/AccountList.kt | ported |
| CoreContentView.swift | ui/sync/SyncStatusScreen.kt · `SyncHome` (+ ui/MainScreen.kt tabs) | ported — Platform Sync "Clear" now actually clears synced data (#3959): fail-closed native reset (`platform_wallet_manager_platform_address_sync_reset` → `PlatformWalletManager.resetPlatformAddressSyncState`) then Room clear (in-place zero of `platform_addresses` scoped by wallet-id-on-network + delete `platform_addresses_sync_states` by network) via `PlatformBalanceSyncService.clearLocalState` |
| CreateWalletView.swift | ui/wallet/CreateWalletScreen.kt · `CreateWallet` | ported |
| IdentitiesContentView.swift | ui/identity/IdentitiesHomeScreen.kt · `IdentitiesHome` | ported |
| QRScannerView.swift | ui/scanner/QrScannerScreen.kt · `QrScanner` | ported |
| ReceiveAddressView.swift | ui/wallet/ReceiveAddressSheet.kt | ported |
| SeedBackupView.swift | ui/wallet/SeedBackupScreen.kt · `SeedBackup` | ported |
| SendTransactionView.swift | ui/wallet/SendTransactionScreen.kt · `SendTransaction` | partial — form + fee UI ported; broadcast deferred on `core_wallet_send_to_addresses` |
| ShieldedActivityView.swift | ui/shielded/ShieldedActivityScreen.kt · `ShieldedActivity` | ported |
| TransactionDetailView.swift | ui/wallet/TransactionDetailScreen.kt · `WalletTransactionDetail` | ported |
| TransactionListView.swift | ui/wallet/TransactionListScreen.kt · `WalletTransactions` | ported |
| WalletDetailView.swift | ui/wallet/WalletDetailScreen.kt · `WalletDetail` | ported |
| WalletKeyHealthSheet.swift | ui/wallet/WalletKeyHealthSheet.kt | ported — Missing keys now offer a Re-derive repair action via the bridged resolver-keyed derive FFI (`PlatformWalletManager.repairIdentityKey`) |
| WalletsContentView.swift | ui/wallet/WalletsScreen.kt · `WalletsHome` | ported |

## Totals

- **ported**: 75 (of 90 Swift views)
- **partial**: 8 (AddressQueriesView, DiagnosticsView, IdentityDetailView, PlatformQueriesView, QuickBasicTokenView, SendTransactionView, TransitionDetailView, WalletMemoryExplorerView)
- **deferred**: 7 (AddIdentityKeyView, BannedAddressesView, ContestDetailView, DocumentWithPriceView, GroveDBPathElementsView, IdentityKeyAddition, RegisterContractSourceView)

Every partial/deferred row names the missing FFI export; the app
surfaces the same name in a dialog at the point of use (grep for
`notBridged` under `ui/`).
