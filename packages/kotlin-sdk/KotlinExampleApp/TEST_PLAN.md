# KotlinExampleApp — Android Test Plan

A catalog of **every action theoretically possible** on Dash via the Platform gRPC API + Dash Core (SPV) layer, each cross-referenced against **what is actually implemented in this Android app today**, and — for the implemented ones — assigned a **frequency tier** so a QA agent can run a meaningful subset.

This file is the Android counterpart of the iOS `SwiftExampleApp/TEST_PLAN.md`. Test IDs are shared across platforms — `CORE-05` on iOS and `CORE-05` on Android exercise the same gRPC / Core operation, just through different UI and persistence layers.

> **Provenance & maintenance.** Generated from the `PARITY.md` view-mapping and the iOS TEST_PLAN.md, assuming full feature parity. When features land or diverge, update the affected rows (status, entry point) and re-tier if behavior changes. Treat the codebase as the source of truth if a row looks stale.

---

## 1. How to use this document (for the QA agent)

Every catalog row carries four orthogonal, machine-filterable fields plus optional **tags**. Select tests by intersecting them.

**Selection grammar** — canonical tokens (case-insensitive):

- **Tier** ∈ `Essential` · `Common` · `Thorough` · `Uncommon` · `Manual`
- **Layer** ∈ `Core` · `Platform` · `Cross` · `Shielded`
- **Status** ∈ `✅` · `🧪` · `⚠️` · `🔌` · `🚫` · `➖`
- **Category** ∈ `Core` · `Identity` · `Address` · `DPNS` · `Voting` · `Contract` · `Document` · `Token` · `Shielded` · `DashPay` · `System`

**Tags** are cross-cutting modalities that span multiple categories. A test may carry zero or more tags (comma-separated in the Tags column). Use tags to select thematic subsets across categories — e.g. all `multiwallet` tests regardless of domain, or all `contested` tests. `multiwallet` and `group` used to be categories — they are now tags, so a multi-wallet token test lives in **Token** and is found with `Tag=multiwallet`.

A test is **automatable now** only if Status is `✅`, `🧪`, or `⚠️` (reachable and drivable in the emulator) **and** `Tier ≠ Manual`. `Tier=Manual` marks implemented features that need a human on a physical device — the automated QA agent must **skip and flag them for manual testing**, never mark them failed.

### Generic pass criteria (apply per action type unless a row overrides)

- **Write / state transition**: the broadcast returns a successful state-transition result (no consensus error), **and** the resulting state is observable — balance changes, a new row appears in Room, the object is fetchable from the network.
- **Read / query**: returns a non-error response with the expected shape (and, for proof-backed reads, a verified proof).
- **Wallet/local action** (backup phrase, alias, address derivation): the local state is produced and persists across an app relaunch.

### Test infrastructure note

Drive the app with **uiautomator / adb** on a booted Android emulator: tap/type/screenshot, read persisted state from the app's Room database, and stream Rust logs via `adb logcat`. Verify writes against **both** the UI and the persisted/queried state — don't trust the UI alone.

---

## 2. Prerequisites & fixtures

Most Platform actions have hard preconditions. Establish these fixtures before selecting tests, and skip (don't fail) rows whose preconditions can't be met in the environment.

| Fixture | Needed for | Notes |
|---|---|---|
| **Network selected** (testnet / devnet) | everything | Confirm the SDK protocol-version floor is applied. |
| **Funded Core wallet** | all `Layer=Core` and `Layer=Cross` | A wallet with confirmed, mature, spendable UTXOs. Asset-lock funding needs InstantSend/ChainLock, so masternode sync must complete. |
| **A registered identity with credit balance** | almost all `Layer=Platform` | Created via `ID-01`. Many transitions also need a specific **key purpose/security level** present on the identity. |
| **A loaded data contract** (with a token + a document type) | `Domain=Token`, `Domain=Document`, group | Token actions are gated by the contract's on-chain permission rules. |
| **A contested-name scenario** | `DPNS-05`, `VOTE-*` | Register a premium/contested name to create a live vote poll. |
| **Masternode / evonode voting key** | `VOTE-01` | Standard app QA on a non-masternode identity **cannot** exercise the actual vote broadcast. |
| **Shielded pool: configured + bound + prover warmed + synced** | `Domain=Shielded` | `SH-01` sync + `SH-09` prover warm-up are preconditions for any shielded spend. |
| **A second identity / contact** | credit transfer, token transfer, document transfer, DashPay | Needed as the counterparty/recipient. |

---

## 3. Legend

**Tiers:**

| Tier | Meaning |
|---|---|
| **Essential** | Core happy-path: create/restore wallet, send, receive, view balances & history, back up phrase, create & view identity, register/check/resolve usernames, shield/transfer/unshield. |
| **Common** | Frequent actions beyond the core experience — top-ups, contested usernames, key management, contracts, token transfer/view, DashPay, secondary shielded flows. |
| **Thorough** | Occasional or specialized role (contract author, voter, multi-wallet power user) — voting, contract update, document edit/delete, mint/burn/claim, group reads. |
| **Uncommon** | Rare / exotic / administrative edge cases. |
| **Manual** | Implemented features that **can't be driven in the emulator** and need a human on a physical device. |

**Layers:**

| Layer | Meaning |
|---|---|
| **Core** | L1 transparent SPV wallet only. |
| **Platform** | Pure L2: identity, contracts, documents, tokens, DPNS, voting, groups. |
| **Cross** | Bridges the two layers (asset-lock funding, credit withdrawal back to L1). |
| **Shielded** | Orchard private pool. |

**Status:**

| Status | Meaning | Runnable now? |
|---|---|---|
| ✅ | Implemented and reachable in the app UI. | Yes |
| 🧪 | Reachable **only** via *Settings → Platform State Transitions* (demo builder). | Yes (builder) |
| ⚠️ | UI exists but is **local-only / mock** — does not broadcast. | Partially (UI only) |
| 🔌 | FFI / JNI wrapper exists, but **no UI** to trigger it. | No (SDK only) |
| 🚫 | Not implemented anywhere (no FFI, no UI). | No |
| ➖ | Retired — folded into another row. | n/a |

---

## 4. Catalog

### 4.1 Core / Wallet — `Domain=Core`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| CORE-01 | Create wallet (new mnemonic) | Core | Essential | ✅ | | `CreateWalletScreen`. New 12/24-word phrase shown; wallet appears in Wallets tab. |
| CORE-02 | Restore wallet (existing mnemonic) | Core | Essential | ✅ | | `CreateWalletScreen` (Import Existing toggle). After sync, derived addresses + balance populate. |
| CORE-03 | Backup / view recovery phrase | Core | Essential | ✅ | | `SeedBackupScreen`. Phrase matches creation; biometric-gated reveal on Android. |
| CORE-04 | Receive (derive address + QR) | Core | Essential | ✅ | | `ReceiveAddressSheet` → `core_wallet_next_receive_address`. Fresh external address + scannable QR. |
| CORE-05 | Send Core L1 transaction | Core | Essential | ✅ | | Send flow (`SendTransactionScreen`, mode Core→Core) → `ManagedPlatformWallet.sendToAddresses` → `CoreTransactionBuilder` (build+sign) → `core_wallet_broadcast_transaction`. Tx broadcasts; balance drops; appears in history. |
| CORE-06 | View balance / tx history / UTXOs | Core | Essential | ✅ | | `WalletDetailScreen`, `TransactionListScreen`, `AccountDetailScreen` (Room). |
| CORE-07 | SPV sync (start / stop / progress) | Core | Essential | ✅ | | Global sync indicator (`GlobalSyncIndicator`) → `platform_wallet_manager_spv_*`. Headers/filters/masternodes advance to tip. |
| CORE-08 | QR scan recipient | Core | Manual | ✅ | | `QrScannerScreen` (CameraX), reachable in the Send flow. Emulators can use a virtual camera scene but reliability varies — treat as `Tier=Manual`. |
| CORE-09 | Multiple HD accounts (within one wallet) | Core | Common | ✅ | | Account selection / `AccountDetailScreen`; balances per `account_index`. |
| CORE-10 | Multi-recipient Core send | Core | Common | ✅ | | Send flow (`SendTransactionScreen`, Core→Core) → "Add recipient" appends extra address/amount rows → `CoreTransactionBuilder` (one `addOutput` per recipient) → `core_wallet_broadcast_transaction`. One tx with N outputs. |

#### Multiple wallets on one device

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| CORE-14 | Hold multiple wallets at once (wallet list) | Core | Thorough | ✅ | multiwallet | `WalletsScreen` lists every wallet for the current network; `PlatformWalletManager.wallets` holds N keyed by `wallet_id`. |
| CORE-15 | Create / import a second wallet (alongside existing) | Core | Thorough | ✅ | multiwallet | Wallets tab → "Add Wallet" → `CreateWalletScreen`. New wallet coexists; must not replace or corrupt the first. |
| CORE-16 | Switch active wallet | Core | Thorough | ✅ | multiwallet | Tap a wallet row → `WalletDetailScreen` scopes all Room queries to that `walletId`. Navigation-based — there is **no** global wallet picker. |
| CORE-17 | Remove / delete a wallet | Core | Uncommon | ✅ | multiwallet | `WalletDetailScreen` → Delete Wallet → `platform_wallet_manager_remove_wallet`; cascades Keystore mnemonic + that wallet's identities + Room rows. Verify other wallets untouched. |
| CORE-18 | Per-wallet isolation (identities / addresses / balances / shielded) | Core | Thorough | ✅ | multiwallet | Confirm wallet A's identities, addresses, Core/Platform balances and shielded state never surface under wallet B (Room queries filtered by `walletId`). |
| CORE-19 | Send between two on-device wallets | Core | Thorough | ✅ | multiwallet | Normal send from wallet A to wallet B's receive address. B's balance increases after sync. |
| CORE-20 | Concurrent SPV sync across all wallets | Core | Thorough | ✅ | multiwallet | One SPV runtime per network filters every wallet's addresses; `spvProgress` is manager-global. With 2+ wallets, confirm each reaches the tip. |
| CORE-22 | Re-add a previously deleted wallet (same network) | Core | Uncommon | ✅ | multiwallet | After `CORE-17`, re-import the same mnemonic on the same network. Re-derives the same `wallet_id`; must re-discover identities/addresses/balances cleanly. |
| CORE-23 | Re-add a deleted wallet that also exists on another network | Core | Uncommon | ✅ | multiwallet | Same mnemonic on two networks → distinct network-scoped `wallet_id`s. Delete on X, verify Y untouched, re-add on X, confirm both coexist. |

### 4.2 Identity — `Domain=Identity`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| ID-01 | Create identity (Core-funded asset lock) | Cross | Essential | ✅ | | `CreateIdentityScreen` / `IdentityRegistrationController` → `platform_wallet_register_identity_with_signer`. New identity + credit balance appear. |
| ID-02 | Load / discover identity from wallet | Platform | Essential | ✅ | | `LoadIdentityScreen` / `SearchWalletsForIdentitiesScreen` → `platform_wallet_discover_identities`. |
| ID-03 | View identity (info / balance / revision / keys) | Platform | Essential | ✅ | | `IdentityDetailScreen`, `KeysListScreen`, `KeyDetailScreen`. |
| ID-04 | Transfer credits identity → identity | Platform | Essential | ✅ | | `IdentityDetailScreen` → **Transfer Credits** (dialog, `TransferCreditsScreen`) → `platform_wallet_transfer_credits_with_signer` (Keystore-signed). Recipient entered via `RecipientPicker` (local identity / paste base58 id / DPNS name). |
| ID-05 | Top up identity (asset lock) | Cross | Common | ✅ | | `IdentityDetailScreen` → **Top Up from Core** (`TopUpIdentityFromCoreScreen`) → `platform_wallet_top_up_identity_with_funding_signer`. Builds + broadcasts a new Core asset lock (same mechanism as ID-01), signed by the asset lock's Core key. |
| ID-06 | Top up identity (from Platform addresses) | Cross | Common | ✅ | | `IdentityDetailScreen` → **Top Up from Platform addresses** (`TopUpIdentityScreen`) → `platform_wallet_top_up_from_addresses_with_signer`. Requires the wallet's Platform-payment addresses to hold credits first — fund them via `WalletDetailScreen` → Platform Balance → **Top Up from Core** (`FundFromAssetLockScreen`). |
| ID-07 | Update identity — add public key | Platform | Common | ✅ | | `AddIdentityKeyScreen` (from `KeysListScreen`) → `updateIdentity(addPublicKeys:)`. |
| ID-08 | Create identity (from Platform addresses) | Cross | Common | ✅ | | `CreateIdentityScreen` → funding source **Platform address** → `IdentityRegistration.registerFromAddresses` → `platform_wallet_register_identity_with_signer` (Keystore-signed; one signer drives both the identity-key and platform-address roles). Derives + persists the canonical key set (as ID-01), then greedily packs the wallet's balance-carrying Platform-payment addresses; nonces auto-fetched Rust-side. Requires funded Platform addresses first (fund via `WalletDetailScreen` → Platform Balance → **Top Up from Core**). |
| ID-09 | Set / edit local alias | Platform | Common | ✅ | | `IdentityDetailScreen` (Add Alias). Local only — persists across relaunch; no broadcast. |
| ID-10 | Withdraw credits → Dash L1 address | Cross | Common | ✅ | withdrawal | `IdentityDetailScreen` → **Withdraw Credits** (dialog, `WithdrawCreditsScreen`) → `platform_wallet_withdraw_credits_with_signer` (Keystore-signed). Destination L1 address typed in + validated. |
| ID-11 | Transfer credits → Platform addresses | Platform | Common | ✅ | | `IdentityDetailScreen` → **Transfer to Platform Address** (`TransferIdentityToAddressScreen`) → `IdentityCredits.transferToAddresses` → `platform_wallet_transfer_credits_to_addresses_with_signer` (Keystore-signed). Recipient = an own-wallet Platform address (credits stay in-wallet, recipient reconciles from proof) or a pasted external 40-hex P2PKH hash; amount gated `>= minOutput` and `<= identity balance`. |
| ID-12 | Update identity — disable key | Platform | Thorough | ✅ | | `KeyDetailScreen` (drill into a key from `KeysListScreen`) → **Key Status → Disable Key** → confirm → `platform_wallet_update_identity_with_signer` (Keystore-signed). Gated to match consensus. |
| ID-13 | Top up identity (builder path) | Cross | — | ➖ | | Retired — builder entry is a stub; covered by `ID-05`/`ID-06`. |
| ID-14 | Credit transfer between two on-device identities (A → B) | Platform | Thorough | ✅ | multiwallet | `IdentityDetailScreen` → **Transfer Credits** (`ID-04`), recipient = wallet B's identity (via `RecipientPicker`). Switch to B; verify credit balance rose. |
| ID-15 | Same identity restored into two wallets (duplicate seed) | Platform | Uncommon | ✅ | multiwallet | Importing the same mnemonic as a second wallet derives the **same** identity; verify consistency. |

### 4.3 Platform Addresses (DIP-17 credit addresses) — `Domain=Address`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| ADDR-01 | Query address info / multiple infos | Platform | Common | ✅ | | `AddressQueriesScreen` → `dash_sdk_address_fetch_info(s)`. |
| ADDR-02 | Transfer credits address → address | Platform | Thorough | ✅ | | `WalletDetailScreen` → Platform Balance row **⋯ menu → Transfer Credits** (`TransferPlatformAddressScreen`) → `platform_address_wallet_transfer` (Keystore-signed). Source = DIP-17 platform-payment account picker; destination = own-wallet address picker or pasted 20-byte P2PKH hash. |
| ADDR-03 | Top up address from an existing (pending) asset lock | Cross | Thorough | ✅ | | Resume a stuck/pending Platform-address asset-lock funding: `IdentitiesHomeScreen` → `PendingAssetLocksList` → `ManagedPlatformWallet.resumeFundFromAssetLock` → `WalletManagerNative.walletResumeFundFromAssetLock`. Distinct from `ADDR-09`, which builds a **new** lock from Core. Needs a pending-lock fixture (an asset lock whose consume did not complete). |
| ADDR-04 | Withdraw address credits → Core L1 | Cross | Thorough | ✅ | withdrawal | `WalletDetailScreen` → Platform Balance row **⋯ menu → Withdraw to Core** (`WithdrawPlatformAddressScreen`) → `platform_address_wallet_withdraw_to_address` (Keystore-signed). Full account balance withdrawn. |
| ADDR-06 | Display / share your Platform receive address | Platform | Common | ✅ | | "Receive Dash" sheet → **Platform** tab (`ReceiveAddressSheet`, platform tab): QR + bech32m DIP-17 address + Copy. |
| ADDR-07 | Platform address balance sync (BLAST) — start / progress; address balances populate to tip | Platform | Essential | ✅ | | Sync tab → **PLATFORM SYNC STATUS** (`SyncStatusScreen`, `container.platformBalanceSyncService`): State reaches `Synced`, Sync Height advances to tip, Active Addresses populate, "Sync Now" forces a pass. Precondition for the other address rows. |
| ADDR-08 | Clear & resync platform address balances | Platform | Common | ✅ | | Sync tab → Platform section **Clear** (`SyncStatusScreen`, `testTag("sync.platformClear")`) → `container.platformBalanceSyncService.clearLocalState` (fail-closed): wipes local platform-address balance state; the next sync repopulates from scratch to tip. |
| ADDR-09 | Top up Platform balance from Core | Cross | Essential | ✅ | | `WalletDetailScreen` → Platform Balance row **⋯ menu → Top Up from Core** (`FundFromAssetLockScreen`) → builds a **new** asset lock from the wallet's Core balance and credits a DIP-17 Platform address once the lock proves (IS→CL) → `dash_sdk_address_top_up_from_asset_lock`. Needs a Core (SPV) balance to build the lock. |

### 4.4 DPNS (usernames) — `Domain=DPNS`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DPNS-01 | Register username (normal) | Platform | Essential | ✅ | | `RegisterNameScreen` → `platform_wallet_register_dpns_name_with_signer`. Name resolves to the identity afterward. |
| DPNS-02 | Check availability / validate / normalize | Platform | Essential | ✅ | | `RegisterNameScreen` / `DpnsTestScreen`. |
| DPNS-03 | Resolve name → identity | Platform | Essential | ✅ | | `QueriesListScreen` (dpnsResolve) / `DpnsTestScreen`. |
| DPNS-04 | Get usernames for an identity | Platform | Essential | ✅ | | `IdentityDetailScreen` DPNS section. |
| DPNS-05 | Register username (contested / premium) | Platform | Common | ✅ | contested | `RegisterNameScreen` (auto-detects contested via `dash_sdk_dpns_is_contested_username`). Creates a live vote poll. |
| DPNS-06 | Select main / primary name | Platform | Common | ✅ | | `SelectMainNameScreen` (dialog from `IdentityDetailScreen`). |
| DPNS-07 | Search names by prefix | Platform | Common | ✅ | | `QueriesListScreen` (dpnsSearch) / `DpnsTestScreen`. |
| DPNS-08 | Contested DPNS race between two on-device identities | Platform | Uncommon | ✅ | multiwallet, contested | A and B both register the same premium/contested name (`DPNS-05`) → produces a contest observable via `VOTE-02`/`VOTE-03`. |

### 4.5 Voting / Contested Resources — `Domain=Voting`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| VOTE-01 | Vote on contested DPNS username (masternode vote) | Platform | Thorough | ✅ | contested, masternode | `ContestDetailScreen` cast-vote → `dash_sdk_contested_resource_cast_vote`. **Requires masternode voting credentials** — environment-limited otherwise. |
| VOTE-02 | Query contested resources | Platform | Thorough | ✅ | contested, read-only | `QueriesListScreen` (getContestedResources). |
| VOTE-03 | Query contested-resource vote state | Platform | Thorough | ✅ | contested, read-only | `QueriesListScreen` (getContestedResourceVoteState). |
| VOTE-04 | Query voters for a contestant identity | Platform | Thorough | ✅ | contested, read-only | `QueriesListScreen` (getContestedResourceVotersForIdentity). |
| VOTE-05 | Query an identity's votes | Platform | Thorough | ✅ | contested, read-only | `QueriesListScreen` (getContestedResourceIdentityVotes). |
| VOTE-06 | Query vote polls by end date | Platform | Thorough | ✅ | contested, read-only | `QueriesListScreen` (getVotePollsByEndDate). |
| VOTE-07 | Masternode vote (generic builder entry) | Platform | — | ➖ | | Retired — covered by `VOTE-01`. |

### 4.6 Data Contracts — `Domain=Contract`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DC-01 | Register / load contract from network | Platform | Common | ✅ | | `RegisterContractSourceScreen` / `LocalDataContractsScreen` / `ContractsHomeScreen`. |
| DC-02 | View contract / schema / doc types / history | Platform | Common | ✅ | | `DataContractDetailsScreen`, `DocumentTypeDetailsScreen`. |
| DC-03 | Create data contract | Platform | Common | ✅ | | `QuickBasicTokenScreen` and *Settings builder → Data Contract Create* → `platform_wallet_create_data_contract_with_signer`. |
| DC-04 | Update data contract | Platform | Thorough | 🧪 | | *Settings builder → Data Contract Update* → `platform_wallet_update_data_contract_with_signer`. |

### 4.7 Documents — `Domain=Document`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DOC-01 | Query documents / single document | Platform | Common | ✅ | | `DocumentsScreen` / `QueriesListScreen` → `dash_sdk_document_search` / `_fetch`. |
| DOC-02 | Create document (broadcast) | Platform | Common | ✅ | | Contracts → contract → document type → **New Document** (`DocumentTypeDetailsScreen` → schema-driven `CreateDocumentScreen`) → `DocumentTransactions.create` → `platform_wallet_create_document_with_signer`. Rust selects the AUTH+ECDSA signing key from the wallet's `IdentityManager`. |
| DOC-03 | Replace document | Platform | Thorough | ✅ | | Contracts → **Browse Documents** → document → **⋯** action menu (ownership-gated) → **Replace…** → `platform_wallet_document_replace`. |
| DOC-04 | Delete document | Platform | Thorough | ✅ | | **Browse Documents** → document → **⋯** → **Delete…** → `platform_wallet_document_delete`. |
| DOC-05 | Transfer document | Platform | Uncommon | ✅ | | **Browse Documents** → document → **⋯** → **Transfer…** (shown when `documentsTransferable`) → `platform_wallet_document_transfer`. |
| DOC-06 | Update document price | Platform | Uncommon | ✅ | | **Browse Documents** → document → **⋯** → **Set Price…** (shown when `tradeMode`) → `platform_wallet_document_set_price`. |
| DOC-07 | Purchase document | Platform | Uncommon | ✅ | | **Browse Documents** → document → **⋯** → **Purchase…** → `platform_wallet_document_purchase`. Buyer ≠ owner enforced. |
| DOC-08 | Document aggregation (umbrella) | Platform | Uncommon | ➖ | | Split into `DOC-10`..`DOC-14`. |
| DOC-09 | Create document (local demo) | Platform | — | ➖ | | Retired. Replaced by real broadcast flow; see `DOC-02`. |
| DOC-10 | Aggregation — count documents (total) | Platform | Uncommon | 🧪 | | **Count Documents** screen → `dash_sdk_document_count`. Requires `documentsCountable: true`. |
| DOC-11 | Aggregation — count documents, filtered (`where`) | Platform | Uncommon | 🧪 | | Same Count screen with a `where` clause → `dash_sdk_document_count(where_json=…)`. |
| DOC-12 | Aggregation — count documents, grouped (`group_by`) | Platform | Uncommon | 🧪 | | Same Count screen with `group_by` → `dash_sdk_document_count(group_by_json=…)`. |
| DOC-13 | Aggregation — sum of a numeric property | Platform | Uncommon | 🧪 | | **Sum / Average Documents** screen (op selector → **Sum**) → `dash_sdk_document_sum`. |
| DOC-14 | Aggregation — average of a numeric property | Platform | Uncommon | 🧪 | | Same screen (op selector → **Average**) → `dash_sdk_document_average`. |
| DOC-15 | Document transfer / purchase across wallets | Platform | Uncommon | ✅ | multiwallet | A creates + lists a document (`DOC-02`/`DOC-06`); B transfers/purchases it (`DOC-05`/`DOC-07`). |

### 4.8 Tokens — `Domain=Token`

All token actions support single-signer **and** group (propose / co-sign) modes via `platform-wallet`.

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| TOK-01 | View token balances / details / search | Platform | Common | ✅ | | `TokenDetailsScreen`, `TokensScreen`, `TokenSearchScreen`. |
| TOK-02 | Transfer token | Platform | Common | ✅ | | `TokenActionScreens(transfer)` → `wallet.tokenTransfer`. |
| TOK-03 | Direct purchase token | Platform | Common | ✅ | | `TokenActionScreens(directPurchase)` → `wallet.tokenPurchase`. |
| TOK-04 | Token queries (statuses / prices / contract info / supply / distributions) | Platform | Common | ✅ | | `QueriesListScreen` token category → `dash_sdk_token_get_*`. |
| TOK-05 | Mint (issuance) | Platform | Thorough | ✅ | | `TokenActionScreens(mint)` → `wallet.tokenMint`. |
| TOK-06 | Burn | Platform | Thorough | ✅ | | `TokenActionScreens(burn)` → `wallet.tokenBurn`. |
| TOK-07 | Claim distribution (perpetual / pre-programmed) | Platform | Thorough | ✅ | | `TokenActionScreens(claim)` → `wallet.tokenClaim`. |
| TOK-08 | Freeze an identity's balance | Platform | Uncommon | ✅ | | `TokenActionScreens(freeze)` → `wallet.tokenFreeze`. |
| TOK-09 | Unfreeze a balance | Platform | Uncommon | ✅ | | `TokenActionScreens(unfreeze)` → `wallet.tokenUnfreeze`. |
| TOK-10 | Destroy frozen funds | Platform | Uncommon | ✅ | | `TokenActionScreens(destroyFrozenFunds)`. |
| TOK-11 | Set / clear direct-purchase price | Platform | Uncommon | ✅ | | `TokenActionScreens(setPrice)` → `wallet.tokenSetPrice`. |
| TOK-12 | Emergency action — Pause | Platform | Uncommon | ✅ | | `TokenActionScreens(pause)` → `platform_wallet_token_pause`. |
| TOK-13 | Emergency action — Resume | Platform | Uncommon | ✅ | | `TokenActionScreens(resume)` → `platform_wallet_token_resume`. |
| TOK-14 | Config update / max supply | Platform | Uncommon | ✅ | | `TokenActionScreens(updateMaxSupply)` → `wallet.tokenUpdateConfig`. |
| TOK-15 | Group action — propose | Platform | Uncommon | ✅ | group | Token action in `.propose` mode (`CoSignProposalScreen`). |
| TOK-16 | Group action — co-sign existing | Platform | Uncommon | ✅ | group | `PendingGroupActionsScreen` / `CoSignProposalScreen`. Action executes when accumulated signer power ≥ required. |
| TOK-17 | Token transfer between two on-device identities | Platform | Thorough | ✅ | multiwallet, regression | `TOK-02`, recipient = wallet B's identity. Switch to B; verify the token balance arrived. |

### 4.9 Shielded Pool (Orchard) — `Domain=Shielded`

Shielded notes/balance/activity have **no read-side FFI** by design — Rust pushes them to Room via persistence callbacks; the app reads Room. Verify shielded reads against Room, not a query.

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| SH-01 | Shielded sync (start / stop / now) | Shielded | Essential | ✅ | | `PlatformWalletManager` → `platform_wallet_manager_shielded_sync_*`. Precondition for any shielded balance/spend. |
| SH-02 | View shielded activity / notes / balance | Shielded | Essential | ✅ | | `ShieldedActivityScreen` (Room Flow queries). |
| SH-03 | Shield from Platform balance (Type 15) | Shielded | Essential | ✅ | | Send flow (Platform→Shielded) → `walletManager.shieldedShield`. |
| SH-04 | Shield from Core L1 balance | Shielded | Essential | ✅ | | Send flow (Core→Shielded). |
| SH-05 | Shielded → shielded transfer (Type 16) | Shielded | Essential | ✅ | | Send flow (Shielded→Shielded) → `walletManager.shieldedTransfer`. Optional ≤32-byte memo. |
| SH-06 | Unshield → Platform address (Type 17) | Shielded | Essential | ✅ | | Send flow (Shielded→Platform) → `walletManager.shieldedUnshield`. |
| SH-07 | Shield from asset lock (Type 18) | Cross | Common | ✅ | | `ShieldedFundScreen` (from `WalletDetailScreen`) → `platform_wallet_manager_shielded_fund_from_asset_lock`. |
| SH-08 | Shielded withdraw → Core L1 (Type 19) | Cross | Common | ✅ | withdrawal | Send flow (Shielded→Core) → `walletManager.shieldedWithdraw` (`core_fee_per_byte` = 1, the dashmate default — same value the iOS send flow passes). |
| SH-09 | Prover warm-up / readiness | Shielded | Common | ✅ | | `warmUpShieldedProver` / `shieldedProverIsReady` (~30s Halo2 key build). |
| SH-10 | Seed shielded pool (anonymity set) | Shielded | Uncommon | ✅ | | `SeedShieldedPoolScreen` → `platform_wallet_manager_shielded_seed_pool_notes`. **Devnet/testnet only.** |
| SH-11 | Create identity from shielded pool (Type 20) | Cross | Common | ✅ | | `CreateIdentityScreen` → funding source **Shielded balance** (fixed denominations) → `platform_wallet_manager_shielded_identity_create_from_pool`. |
| SH-12 | Clear shielded state (wipe notes + re-sync) | Shielded | Uncommon | ✅ | | "Clear" button on the Sync tab (`SyncStatusScreen` → `ShieldedService.clearLocalState`). Stops sync, wipes EVERY wallet's Room rows (the Rust reset empties the SHARED tree, so any surviving watermark would re-freeze it), zeroes the mirror; bind credentials kept. The eager re-bind covers the whole fleet: the mirror via `bind`, every other loaded wallet via `bindEngine`, so cross-wallet rows (`SH-14/15/16`) work right after an SH-12 run without a relaunch. |
| SH-13 | Display / share your shielded receive address | Shielded | Common | ✅ | read-only | "Receive Dash" sheet → **Shielded** tab (`ReceiveAddressSheet`, shielded tab): QR + `tdash1…`/`dash1…` bech32m address + Copy (resolved per-wallet via `shieldedDefaultAddress`). Grab wallet B's address here for `SH-14`. |
| SH-14 | Shielded transfer between two on-device wallets | Shielded | Thorough | ✅ | multiwallet | Wallet A's pool → wallet B's shielded address (`SH-05`); copy B's address from Receive → Shielded tab (`SH-13`). Both wallets are engine-bound automatically at rebind (no wallet-swap needed); B's shielded balance rises on the next sync pass. |
| SH-15 | Unshield from A to a Platform address owned by B | Shielded | Uncommon | ✅ | multiwallet | A unshields (`SH-06`) to a Platform address belonging to wallet B. Both wallets are engine-bound automatically at rebind, so A can spend from its own pool without a wallet-swap. |
| SH-16 | Shielded withdraw from A to B's Core L1 address | Shielded | Uncommon | ✅ | multiwallet, withdrawal | Wallet A's pool → a Core L1 address owned by wallet B (`SH-08`). Verify B's Core balance rises after SPV sync. Both wallets are engine-bound automatically at rebind. |
| SH-17 | Multiple wallets bound to the shielded pool concurrently | Shielded | Uncommon | ✅ | multiwallet | `platform_wallet_manager_bind_shielded` is per `wallet_id`; the manager syncs all bound wallets. EVERY loaded wallet is engine-bound automatically at rebind (`AppContainer.rebindWalletScopedServices` → `ShieldedService.bindEngine`); the global Sync tab still mirrors one wallet, but per-wallet Receive/Balance surfaces read per-wallet. |

### 4.10 DashPay — `Domain=DashPay`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DP-01 | Send contact request | Platform | Common | ✅ | | DashPay tab → **Add Contact** (`dashpay.addContact` → `ui/dashpay/AddContactScreen.kt` · `DashPayAddContact`): by Identity ID (`dashpay.addContact.idInput`, base58 32-byte gated) or DPNS username (`dashpay.addContact.input`, ≥2-char debounced `searchDpnsNames`; not-found → `dashpay.addContact.retry`; collision dialog) → **Send** (`dashpay.addContact.send`) → `sendContactRequest` (`platform_wallet_send_contact_request_with_signer`). Its own route (no cross-screen optimistic overlay); the outgoing pending row appears once post-send sync persists it. |
| DP-02 | Accept contact request | Platform | Common | ✅ | | `ui/dashpay/ContactRequestsScreen.kt` · `DashPayRequests` (Incoming) → **Accept** (`dashpay.request.accept`) → `acceptIncomingRequest` (`platform_wallet_accept_contact_request_with_signer`). Contact moves to `ContactsScreen`; the bidirectional contact persists (both request-direction rows present for the pair). |
| DP-03 | Send DashPay payment to a contact | Platform | Common | ✅ | | `ui/dashpay/ContactDetailScreen.kt` → **Send Dash** (`dashpay.detail.sendDash`) → `SendDashPayPaymentSheet` (`dashpay.send.amount` → `dashpay.send.confirm`) → `sendPayment` (`platform_wallet_send_dashpay_payment`), then always `refreshDashPayPayments` (the durability invariant; manual refresh `dashpay.detail.refreshPayments`). A DashPay payment is an **L1 transaction**, so it requires the **Core SPV client running** (`CORE-07`) — with SPV stopped the broadcast fails. Payment appears in the contact's Room-backed history (txid via `txidDisplayHex`). **Verify both directions** — the channel is symmetric once established (A→B *and* B→A); each sender needs its own funded Core wallet + running SPV, both endpoints on the **same network**. Send is disabled while the payment channel is broken. |
| DP-04 | Create / update DashPay profile | Platform | Common | ✅ | | `ui/dashpay/DashPayProfileScreen.kt` · `DashPayProfile` → **Edit** (`dashpay.profile.edit`) → inline editor → **Done** (`dashpay.profile.done`) → `createOrUpdateProfile` (`platform_wallet_create_or_update_dashpay_profile_with_signer`), doCreate when no profile exists. Non-destructive update; avatar renders via the `DashPayAvatar` (Coil) composable off the re-fetched profile. |
| DP-05 | View profile / contacts / requests | Platform | Common | ✅ | read-only | DashPay tab (`dashpay.tab`): `ContactsScreen` (`dashpay.openContacts`, established + search `dashpay.search`), `ContactRequestsScreen` (`dashpay.openRequests`, incoming/outgoing), `ContactDetailScreen`, `DashPayProfileScreen` (`dashpay.openProfile`) — backed by Room `observeContactRequests`; established contacts are derived in-memory by joining each pair's incoming + outgoing request rows. Received-from-contacts balance at `dashpay.receivedBalance`. |
| DP-06 | Ignore a contact request (reversible local mute) | Platform | Thorough | ✅ | | `ContactRequestsScreen` → **Ignore** (`dashpay.request.ignore`) → `ignoreContactSender`. The sender leaves the requests list and appears in `ui/dashpay/IgnoredContactsScreen.kt` · `DashPayIgnored` (`dashpay.openIgnored`); **Un-ignore** there → `unignoreContactSender` reverses it. Local-only, no on-chain artifact (R1 privacy); persists across relaunch. |
| DP-07 | Attach `encryptedAccountLabel`; see contact's "Their account" on receive | Platform | Common | ✅ | | DIP-15 §8.5. Send: `AddContactScreen` → **Account label** (`dashpay.addContact.accountLabel`) carried into `sendContactRequest(accountLabel = …)`. Receive: the counterparty's `ContactDetailScreen` shows a read-only **"Their account"** block (assert on visible text — no testTag yet). Verify on a two-wallet loop (cf. `DP-11`): the ingested request carries the encrypted bytes, but the plaintext is decrypted **on accept** (the signer-bearing register step) and shown on the **incoming row only** (direction-specific). |
| DP-08 | QR auto-accept (build "Add me" QR + add via scanned URI) | Platform | Thorough | ✅ | | DIP-15 §8.13. Build: `DashPayProfileScreen` → **Add me (DIP-15 QR)** (`dashpay.profile.qrURI`; `du=…&dapk=…`, 1h validity `AUTO_ACCEPT_TTL_SECS=3600`) via `buildAutoAcceptQr`, rendered with the ZXing `generateQrBitmap` helper. Add: DashPay tab → **Add via QR** (`dashpay.addViaQR`) → the shared QR scanner → `sendContactRequestFromQr`. Two-wallet: A builds the QR, B scans it → the request is auto-accepted by A without A manually accepting (a distinct path from `DP-02`). **A's reciprocal is signer-backed**, so it only lands once A's wallet is **unlocked** (the deferred contact-crypto drain); the request + auto-accept proof reach A immediately, the reciprocal after unlock. A camera scan on a physical device is the Manual variant. |
| DP-09 | Publish encrypted on-chain `contactInfo` (private contact metadata) | Platform | Thorough | ✅ | | DIP-15 §10. `ContactDetailScreen` → edit **Alias** (`dashpay.detail.aliasEdit`) / **Note** (`dashpay.detail.noteEdit`) / **Hide contact** (`dashpay.detail.hideToggle`) → `setContactInfo` (`platform_wallet_set_dashpay_contact_info_with_signer`, ECB `encToUserId` + CBC `privateData`). Locally cached **and** published encrypted to Platform once the identity has **≥2 established contacts** → `ContactInfoPublishOutcome` (`Published` / `DeferredUntilTwoContacts` / `SkippedWatchOnly`), surfaced in the UI. Hide is reversible from `ui/dashpay/HiddenContactsScreen.kt` (`dashpay.openHidden`, `setContactInfo(displayHidden = false)` preserving alias/note). |
| DP-10 | Incoming-payment backfill rescan (restore-from-seed / pre-watch window) | Cross | Manual | ✅ | regression | DIP-15 §8.7 / §12.6 (on the DIP-16 SPV base). No UI trigger — automatic in DashPay sync: the Rust `reconcile_dashpay_rescan` lowers SPV `synced_height` to `min($coreHeightCreatedAt)` across new receival contacts so the filter manager backfills, driven through the Kotlin manager sync loop. Pass: a DashPay payment that landed on a contact's address **before** it was watched (restore-from-seed / second device / the offline-accept→pay window) appears after restore + SPV sync. Environment-limited (must construct the skew window); the regression pin for the §12.6 payment-loss gap. The identity-key breadcrumb backfill is deliberately not ported (no pre-breadcrumb Android installs), but the SPV rescan itself applies. |
| DP-11 | DashPay request → accept → payment, both endpoints on device | Platform | Thorough | ✅ | multiwallet | A's identity sends a contact request (`DP-01`) to B's; switch to wallet B's identity and accept (`DP-02`); then pay (`DP-03`). Full bidirectional loop entirely local. |

### 4.11 System / Protocol / Diagnostics — `Domain=System`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| SYS-01 | Status / total credits / quorums / prefunded balance | Platform | Thorough | ✅ | | `QueriesListScreen` system category. |
| SYS-02 | Epochs info / current / finalized / proposed blocks | Platform | Thorough | ✅ | | `QueriesListScreen` epoch category. |
| SYS-03 | Protocol-version upgrade state / vote status | Platform | Uncommon | ✅ | | `QueriesListScreen` protocol category. |
| SYS-04 | Run-all-queries / DPNS test harness | Platform | Thorough | ✅ | | `QueriesListScreen` diagnostics (`runAllQueries`, `testDPNSQueries`), `DiagnosticsScreen`. |
| SYS-05 | Storage / Keystore / Wallet-memory explorers | — | Thorough | ✅ | read-only | `StorageExplorerScreen`, `KeystoreExplorerScreen`, `WalletMemoryExplorerScreen` (Settings; debug tooling). |
| SYS-06 | Path elements (raw GroveDB) | Platform | Uncommon | 🧪 | read-only | **Get GroveDB Path Elements** read view → `dash_sdk_system_get_path_elements`. Enter `path` + `keys` JSON array; returns `[{key, element, type}]`. Use a **bounded** path. |
| SYS-07 | Platform balance sync covers every registered wallet | Platform | Thorough | ✅ | multiwallet | The Rust `PlatformAddressSyncManager` sweeps EVERY registered wallet each pass (like Core SPV, `CORE-20`); `PlatformBalanceSyncService.configure(manager)` merely reflects the shared loop. Verify wallet B's Platform address balances update **without** switching to B. (Row previously claimed per-active-wallet staleness — obsolete since the manager-level all-wallet sweep.) |
| SYS-08 | Per-wallet Platform isolation (identities / usernames / tokens / contacts) | Platform | Thorough | ✅ | multiwallet | Wallet A's identities, DPNS names, token balances, and DashPay contacts must never surface under wallet B. |

---

## 5. Summary matrix

Counts are of rows reachable in the app (Status `✅`/`🧪`/`⚠️`); `🔌`/`🚫`/stub rows are excluded. `Tier=Manual` rows are reachable but **not automatable** — counted separately, excluded from by-layer automatable totals.

**By tier:**

| Tier | Count (approx.) | Automatable? |
|---|---|---|
| Essential | 21 | yes |
| Common | 29 | yes |
| Thorough | 35 | yes |
| Uncommon | 25 | yes |
| Manual | 1 (`CORE-08`) | no — physical device |

**By layer (automatable only):**

| Layer | Count (approx.) |
|---|---|
| Core | 17 |
| Platform | ~74 |
| Cross | 7 |
| Shielded | 16 |

---

## 6. Category index

Membership of each feature category across **all** sections (primary section members + cross-cutting tests that live elsewhere).

- **Core / Wallet** — `CORE-01..23`
- **Identity** — `ID-01..15`, `SH-11`
- **Address** (DIP-17 platform addresses) — `ADDR-01..04`, `ADDR-06..09`, `ID-06`, `ID-08`, `ID-11`
- **DPNS** — `DPNS-01..08`
- **Voting** — `VOTE-01..07`, `DPNS-05`, `DPNS-08`
- **Contract** — `DC-01..04`
- **Document** — `DOC-01..15`
- **Token** — `TOK-01..17`
- **Shielded** — `SH-01..17`
- **DashPay** — `DP-01..11`
- **System / Diagnostics** — `SYS-01..08`

### Tag index

Tags are cross-cutting modalities. A test may appear under multiple tags.

- **multiwallet** — `CORE-14..23`, `ID-14`, `ID-15`, `TOK-17`, `DPNS-08`, `DP-11`, `DOC-15`, `SH-14`, `SH-15`, `SH-16`, `SYS-07`, `SYS-08`
- **group** — `TOK-15`, `TOK-16`
- **contested** — `DPNS-05`, `DPNS-08`, `VOTE-01..06`
- **withdrawal** — `ID-10`, `ADDR-04`, `SH-08`, `SH-16`
- **regression** — `TOK-17`, `DP-10`
- **read-only** — `SH-13`, `SYS-05`, `SYS-06`, `VOTE-02..06`
- **masternode** — `VOTE-01`
