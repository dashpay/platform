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
| AddIdentityKeyView.swift | ui/identity/AddIdentityKeyScreen.kt · `AddIdentityKey` | ported — full form (locked purpose/level/type combos, contract-bounds picker, auto-assigned slot) + submit over `IdentityUpdates.update`; slot derive via the keypair-returning `IdentityNative.deriveIdentityKeyPairWithResolver` (private scalar Keystore-persisted, public half in the update row) |
| AddressFundFromAssetLockProgressView.swift | ui/funding/AddressFundProgressScreen.kt · `AddressFundProgress` | ported (controller/coordinator dismissal-safe like iOS) |
| AddressQueriesView.swift | ui/diagnostics/AddressQueriesScreen.kt · `AddressQueries` | ported — single + batch execution wired over the now-bridged `Sdk.addresses.fetchInfo` / `fetchInfos` (`dash_sdk_address_fetch_info` / `dash_sdk_addresses_fetch_infos`), plus the local Platform-address browser |
| BannedAddressesView.swift | ui/diagnostics/BannedAddressesScreen.kt · `BannedAddresses` | ported — live ban list from `PlatformWalletManager.addressBanInfo()` (the now-bridged `platform_wallet_manager_address_ban_info`), keeping the iOS empty-state semantics |
| ContestDetailView.swift | ui/identity/ContestDetailScreen.kt · `ContestDetail` | ported — live contenders/tallies/winner via the bridged `Voting.contestedResourceVoteState` (`dash_sdk_contested_resource_get_vote_state`, DocumentsAndVoteTally); masternode vote casting (TowardsIdentity / Abstain / Lock + pro_tx_hash / voting-key form) via `VoteCasting.castVote` (`dash_sdk_contested_resource_cast_vote`). The bridged read carries no poll end time (iOS's wallet path does), so the status row derives from winner presence and the countdown bar is omitted |
| ContractsTabView.swift | ui/contracts/ContractsHomeScreen.kt · `ContractsHome` | ported |
| CountDocumentsView.swift | ui/contracts/CountDocumentsScreen.kt · `CountDocuments` | ported |
| CreateIdentityView.swift | ui/identity/CreateIdentityScreen.kt · `CreateIdentity` | ported |
| DPNSTestView.swift | ui/identity/DpnsTestScreen.kt · `DpnsTest` | ported |
| DataContractDetailsView.swift | ui/contracts/DataContractDetailsScreen.kt · `ContractDetail` | ported (incl. group drill-in) |
| DiagnosticsView.swift | ui/diagnostics/DiagnosticsScreen.kt · `Diagnostics` | ported — "Run All Queries" executes every bridged registry query against the shared testnet fixtures (the registry now covers the iOS catalog); adds environment / sync-state / DB-count sections |
| DocumentFieldsView.swift | ui/contracts/DocumentFieldsScreen.kt · `DocumentFields` | ported |
| DocumentTypeDetailsView.swift | ui/contracts/DocumentTypeDetailsScreen.kt · `DocumentTypeDetail` | ported |
| DocumentWithPriceView.swift | ui/contracts/DocumentWithPriceScreen.kt · `DocumentWithPrice` | ported — debounced document-id probe (price / owner / ownership badging via `Documents.fetch`), plus the purchase (`DocumentTransactions.purchase` → `platform_wallet_document_purchase`) and owner set-price (`DocumentTransactions.setPrice` → `platform_wallet_document_set_price`) submit flows that live in `PurchaseDocumentView` / the set-price sheet on iOS, hosted as one screen; entries from DocumentsScreen rows ("Price…") and the transition catalog |
| DocumentsView.swift | ui/contracts/DocumentsScreen.kt · `Documents` | ported (query role; viewer role in DocumentFieldsScreen; the row-level Purchase… / Set Price… actions drill into DocumentWithPriceScreen; create / replace / delete / transfer stay unbridged — see TransitionDetailView) |
| ~~FriendsView.swift~~ (deleted upstream) | ui/identity/FriendsScreen.kt · `Friends` | **in migration** — the Swift view was deleted by PR #3841 and replaced by the first-class DashPay tab (`Views/DashPay/`, 10 views: DashPayTabView, ContactsView, ContactRequestsView, AddContactView, ContactDetailView, SendDashPayPaymentSheet, DashPayProfileView, IgnoredContactsView, HiddenContactsView, DashPayContactMeta), none of which are ported yet. FriendsScreen still covers a slice (sync / list / send / accept / ignore over the 17 bridged exports, reconciled with the post-#3841 FFI in `2298a2059f`); it is retired and superseded per `docs/dashpay/KOTLIN_MIGRATION_SPEC.md` (milestones K1–K3) |
| FundFromAssetLockPlatformAddressView.swift | ui/funding/FundFromAssetLockScreen.kt · `FundFromAssetLock` | ported — submit picks a fresh unused Platform address and funds via the now-bridged `platform_address_wallet_fund_from_asset_lock_signer` (+ resume variant on `ManagedPlatformWallet`); coordinator/progress/pending list drive the flow |
| TransferPlatformAddressView.swift (ADDR-02, #3923) | ui/credits/TransferPlatformAddressScreen.kt · `TransferPlatformAddress` | ported — wallet-signed DIP-17 credit transfer via the now-bridged `platform_address_wallet_transfer` (`ManagedPlatformWallet.transferCredits`, AUTO selection, null inputs/fee-strategy); source account + destination (own-wallet / external P2PKH hash) + amount only; gate reads version-locked `minInput`/`minOutput` via `walletPlatformAddressMinAmounts`. Launched from WalletDetailScreen's Platform Credits section |
| WithdrawPlatformAddressView.swift (ADDR-04, #3923) | ui/credits/WithdrawPlatformAddressScreen.kt · `WithdrawPlatformAddress` | ported — wallet-signed full-balance DIP-17 withdrawal to a Core L1 address via the now-bridged `platform_address_wallet_withdraw_to_address` (`ManagedPlatformWallet.withdrawCredits`); submit gated on `platform_address_wallet_preflight_withdrawal` (`preflightWithdrawal`, off the main thread on `Dispatchers.IO`), and when the gate refuses, the advisory "why not" from `preflightWithdrawalReason` (`platform_address_wallet_preflight_withdrawal_reason`) renders under the status row; Fibonacci fee-rate picker mirrors `WithdrawalCoreFeeRates`. Launched from WalletDetailScreen's Platform Credits section |
| GroupDetailView.swift | ui/contracts/GroupDetailScreen.kt · `GroupDetail` | ported — members resolved against local identities; adds live open-proposals via bridged `Groups.pendingActions` |
| GroveDBPathElementsView.swift | ui/contracts/GroveDBPathElementsScreen.kt · `GroveDbPathElements` | ported — path/keys JSON form + DPNS-contract preset over the bridged `SystemQueries.groveDbPathElements` (`dash_sdk_system_get_path_elements`); entry from the Platform Queries list, mirroring the iOS placement |
| IdentitiesView.swift | ui/identity/IdentitiesHomeScreen.kt · `IdentitiesHome` | ported |
| IdentityDetailView.swift | ui/identity/IdentityDetailScreen.kt · `IdentityDetail` | ported — contested-name rows now render in the DPNS section and drill into ContestDetail (adapted: contests are discovered by probing locally-known labels with `Voting.contestedResourceVoteState` and filtering to unresolved contests listing this identity as a contender; the network-wide by-identity discovery `dash_sdk_dpns_get_contested_usernames_by_identity` remains unbridged) |
| IdentityKeyAddition.swift | services/IdentityKeyAdditionFlow.kt | ported — derive → Keystore-persist → `IdentityPubkey` flow with the real keypair deriver injected (`PlatformWalletManager.deriveIdentityKeyPair`); slot assignment, Drive-combination validation, scalar scrubbing |
| KeyDetailView.swift | ui/identity/KeyDetailScreen.kt · `KeyDetail` | ported — incl. the Key Status section: `KeyDisableGate`-guarded destructive Disable Key with confirm dialog, submitting through the bridged `IdentityUpdates.disableKeys` (`platform_wallet_update_identity_with_signer`) |
| KeychainExplorerView.swift | ui/diagnostics/KeystoreExplorerScreen.kt · `KeystoreExplorer` | ported (adapted: WalletStorage entries masked + AndroidKeyStore aliases; adds biometric-gated mnemonic reveal) |
| KeysListView.swift | ui/identity/KeysListScreen.kt · `KeysList` | ported — toolbar Add action opens AddIdentityKeyScreen (← the AddIdentityKeyView sheet) |
| LoadIdentityView.swift | ui/identity/LoadIdentityScreen.kt · `LoadIdentity` | ported |
| LocalDataContractsView.swift | ui/contracts/LocalDataContractsScreen.kt · `LocalContracts` | ported |
| OptionsView.swift | ui/settings/SettingsScreen.kt · `SettingsHome` (+ AboutSheet.kt) | ported — Network/SPV/Data/Platform/About sections incl. About bottom sheet |
| PendingPlatformFundFromAssetLocksList.swift | ui/funding/AddressFundProgressScreen.kt (`PendingAssetLocksList`) | ported (embedded in IdentitiesHome, matching iOS) |
| PendingRegistrationsList.swift | ui/identity/RegistrationProgressScreen.kt (`PendingRegistrationRow`) | ported (embedded in IdentitiesHome) |
| PlatformQueriesView.swift | ui/contracts/QueryDetailScreen.kt (`QueriesListScreen`) · `QueriesList` | ported — the registry now spans the iOS query catalog (adapted: one flat Material list instead of 12 category groups) plus the dedicated GroveDB Path Elements / Address Queries / Run All Queries entries |
| PlatformStateTransitionsView.swift | ui/transitions/StateTransitionsScreen.kt · `StateTransitions` | ported (iOS file is a trivial wrapper around StateTransitionsView) |
| QueryDetailView.swift | ui/contracts/QueryDetailScreen.kt · `QueryDetail` | ported for the bridged registry |
| QuickBasicTokenView.swift | ui/tokens/QuickBasicTokenScreen.kt · `QuickBasicToken` | ported — contract-registration submit wired via `ManagedPlatformWallet.dataContracts.create` (`platform_wallet_create_data_contract_with_signer`) |
| RecoverWalletsSheet.swift | ui/wallet/RecoverWalletsFlow.kt | ported |
| RegisterContractSourceView.swift | ui/contracts/RegisterContractSourceScreen.kt · `RegisterContractSource` | ported — the manual JSON-editor source path broadcasting via `ManagedPlatformWallet.dataContracts.create` (`platform_wallet_create_data_contract_with_signer`); the quick-token source lives in QuickBasicTokenScreen |
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
| TransitionDetailView.swift | ui/transitions/TransitionDetailScreen.kt · `TransitionDetailRoute` | partial — dynamic forms with live Room-backed identity / token / contract / document-type pickers; 18 of the 23 catalog definitions execute: inline `identityUpdate` (disable path via `IdentityUpdates`) and `masternodeVote` (via `VoteCasting.castVote`), plus dedicated routes for the credit ops, identity create, contract register, document price / purchase (`DocumentWithPrice`), the eight token action forms, and the DPNS contest drill-in. Named dialogs remain for dataContractUpdate, documentCreate, documentReplace, documentDelete, documentTransfer (platform-wallet FFIs not bridged) and `identityUpdate`'s add-keys sub-path (scalar-only slot-derive; see AddIdentityKeyView) |
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
| SendTransactionView.swift | ui/wallet/SendTransactionScreen.kt · `SendTransaction` | ported — broadcast wired via `ManagedPlatformWallet.sendToAddresses` (the now-bridged `core_wallet_send_to_addresses` composite) |
| ShieldedActivityView.swift | ui/shielded/ShieldedActivityScreen.kt · `ShieldedActivity` | ported |
| TransactionDetailView.swift | ui/wallet/TransactionDetailScreen.kt · `WalletTransactionDetail` | ported |
| TransactionListView.swift | ui/wallet/TransactionListScreen.kt · `WalletTransactions` | ported |
| WalletDetailView.swift | ui/wallet/WalletDetailScreen.kt · `WalletDetail` | ported |
| WalletKeyHealthSheet.swift | ui/wallet/WalletKeyHealthSheet.kt | ported — Missing keys now offer a Re-derive repair action via the bridged resolver-keyed derive FFI (`PlatformWalletManager.repairIdentityKey`) |
| WalletsContentView.swift | ui/wallet/WalletsScreen.kt · `WalletsHome` | ported |

## Totals

- **ported**: 87 (of the 90 pre-#3841 Swift views; the FriendsView row above
  no longer counts — its Swift source is deleted)
- **partial**: 2 (TransitionDetailView — 5 of 23 catalog entries lack backing FFIs: dataContractUpdate, documentCreate/Replace/Delete/Transfer; WalletMemoryExplorerView — asset-lock drill-down summary only)
- **deferred**: 0
- **in migration**: the DashPay tab (10 `Views/DashPay/` views added by
  PR #3841) — see `docs/dashpay/KOTLIN_MIGRATION_SPEC.md`; this table gets
  its `Views/DashPay/` section when milestone K3 lands

Every partial/deferred row names the missing FFI export; the app
surfaces the same name in a dialog at the point of use (grep for
`notBridged` under `ui/`).
