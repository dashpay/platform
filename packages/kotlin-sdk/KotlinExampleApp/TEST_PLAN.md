# KotlinExampleApp — Android Test Plan

A catalog of **every action theoretically possible** on Dash via the Platform gRPC API + Dash Core (SPV) layer, each cross-referenced against **what is actually implemented in this Android app today**, and — for the implemented ones — assigned a **frequency tier** so a QA agent can run a meaningful subset.

This file is the Android counterpart of the iOS `SwiftExampleApp/TEST_PLAN.md`. Test IDs are shared across platforms — `CORE-05` on iOS and `CORE-05` on Android exercise the same gRPC / Core operation, just through different UI and persistence layers.

> **Provenance & maintenance.** Generated from the `PARITY.md` view-mapping and the iOS TEST_PLAN.md, assuming full feature parity. When features land or diverge, update the affected rows (status, entry point) and re-tier if behavior changes. Treat the codebase as the source of truth if a row looks stale.

---

## 1. How to use this document (for the QA agent)

Every catalog row carries four orthogonal, machine-filterable fields. Select tests by intersecting them.

**Selection grammar** — canonical tokens (case-insensitive):

- **Tier** ∈ `Essential` · `Common` · `Thorough` · `Uncommon` · `Manual`
- **Layer** ∈ `Core` · `Platform` · `Cross` · `Shielded`
- **Status** ∈ `✅` · `🧪` · `⚠️` · `🔌` · `🚫` · `➖`
- **Category** ∈ `Core` · `Identity` · `Address` · `DPNS` · `Voting` · `Contract` · `Document` · `Token` · `Shielded` · `DashPay` · `Group` · `System` · `MultiWallet`

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

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| CORE-01 | Create wallet (new mnemonic) | Core | Essential | ✅ | `CreateWalletScreen`. New 12/24-word phrase shown; wallet appears in Wallets tab. |
| CORE-02 | Restore wallet (existing mnemonic) | Core | Essential | ✅ | `CreateWalletScreen` (Import Existing toggle). After sync, derived addresses + balance populate. |
| CORE-03 | Backup / view recovery phrase | Core | Essential | ✅ | `SeedBackupScreen`. Phrase matches creation; biometric-gated reveal on Android. |
| CORE-04 | Receive (derive address + QR) | Core | Essential | ✅ | `ReceiveAddressSheet` → `core_wallet_next_receive_address`. Fresh external address + scannable QR. |
| CORE-05 | Send Core L1 transaction | Core | Essential | ✅ | Send flow (`SendTransactionScreen`, mode Core→Core) → `core_wallet_send_to_addresses`. Tx broadcasts; balance drops; appears in history. |
| CORE-06 | View balance / tx history / UTXOs | Core | Essential | ✅ | `WalletDetailScreen`, `TransactionListScreen`, `AccountDetailScreen` (Room). |
| CORE-07 | SPV sync (start / stop / progress) | Core | Essential | ✅ | Global sync indicator (`GlobalSyncIndicator`) → `platform_wallet_manager_spv_*`. Headers/filters/masternodes advance to tip. |
| CORE-08 | QR scan recipient | Core | Manual | ✅ | `QrScannerScreen` (CameraX), reachable in the Send flow. Emulators can use a virtual camera scene but reliability varies — treat as `Tier=Manual`. |
| CORE-09 | Multiple HD accounts (within one wallet) | Core | Common | ✅ | Account selection / `AccountDetailScreen`; balances per `account_index`. |
| CORE-10 | Multi-recipient Core send | Core | Common | ✅ | Send flow (`SendTransactionScreen`, Core→Core) → "Add recipient" appends extra address/amount rows → `core_wallet_send_to_addresses`. One tx with N outputs. |

#### Multiple wallets on one device

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| CORE-14 | Hold multiple wallets at once (wallet list) | Core | Thorough | ✅ | `WalletsScreen` lists every wallet for the current network; `PlatformWalletManager.wallets` holds N keyed by `wallet_id`. |
| CORE-15 | Create / import a second wallet (alongside existing) | Core | Thorough | ✅ | Wallets tab → "Add Wallet" → `CreateWalletScreen`. New wallet coexists; must not replace or corrupt the first. |
| CORE-16 | Switch active wallet | Core | Thorough | ✅ | Tap a wallet row → `WalletDetailScreen` scopes all Room queries to that `walletId`. Navigation-based — there is **no** global wallet picker. |
| CORE-17 | Remove / delete a wallet | Core | Uncommon | ✅ | `WalletDetailScreen` → Delete Wallet → `platform_wallet_manager_remove_wallet`; cascades Keystore mnemonic + that wallet's identities + Room rows. Verify other wallets untouched. |
| CORE-18 | Per-wallet isolation (identities / addresses / balances / shielded) | Core | Thorough | ✅ | Confirm wallet A's identities, addresses, Core/Platform balances and shielded state never surface under wallet B (Room queries filtered by `walletId`). |
| CORE-19 | Send between two on-device wallets | Core | Thorough | ✅ | Normal send from wallet A to wallet B's receive address. B's balance increases after sync. |
| CORE-20 | Concurrent SPV sync across all wallets | Core | Thorough | ✅ | One SPV runtime per network filters every wallet's addresses; `spvProgress` is manager-global. With 2+ wallets, confirm each reaches the tip. |
| CORE-21 | Multiple wallets bound to the shielded pool concurrently | Shielded | Uncommon | ✅ | `platform_wallet_manager_bind_shielded` is per `wallet_id`; the manager syncs all bound wallets. |
| CORE-22 | Re-add a previously deleted wallet (same network) | Core | Uncommon | ✅ | After `CORE-17`, re-import the same mnemonic on the same network. Re-derives the same `wallet_id`; must re-discover identities/addresses/balances cleanly. |
| CORE-23 | Re-add a deleted wallet that also exists on another network | Core | Uncommon | ✅ | Same mnemonic on two networks → distinct network-scoped `wallet_id`s. Delete on X, verify Y untouched, re-add on X, confirm both coexist. |

### 4.2 Identity — `Domain=Identity`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| ID-01 | Create identity (Core-funded asset lock) | Cross | Essential | ✅ | `CreateIdentityScreen` / `IdentityRegistrationController` → `platform_wallet_register_identity_with_signer`. New identity + credit balance appear. |
| ID-02 | Load / discover identity from wallet | Platform | Essential | ✅ | `LoadIdentityScreen` / `SearchWalletsForIdentitiesScreen` → `platform_wallet_discover_identities`. |
| ID-03 | View identity (info / balance / revision / keys) | Platform | Essential | ✅ | `IdentityDetailScreen`, `KeysListScreen`, `KeyDetailScreen`. |
| ID-04 | Transfer credits identity → identity | Platform | Essential | ✅ | `IdentityDetailScreen` → **Transfer Credits** (dialog, `TransferCreditsScreen`) → `platform_wallet_transfer_credits_with_signer` (Keystore-signed). Recipient entered via `RecipientPicker` (local identity / paste base58 id / DPNS name). |
| ID-05 | Top up identity (asset lock) | Cross | Common | ✅ | `TopUpIdentityScreen` (dialog from `IdentityDetailScreen`). |
| ID-06 | Top up identity (from Platform addresses) | Cross | Common | ✅ | `AddressQueriesScreen` → TopUpIdentityFromAddresses → `dash_sdk_identity_top_up_from_addresses`. |
| ID-07 | Update identity — add public key | Platform | Common | ✅ | `AddIdentityKeyScreen` (from `KeysListScreen`) → `updateIdentity(addPublicKeys:)`. |
| ID-08 | Create identity (from Platform addresses) | Cross | Common | 🔌 | `dash_sdk_identity_create_from_addresses` exists in rs-sdk-ffi but is not bridged to Android — no JNI export or Kotlin caller yet. |
| ID-09 | Set / edit local alias | Platform | Common | ✅ | `IdentityDetailScreen` (Add Alias). Local only — persists across relaunch; no broadcast. |
| ID-10 | Withdraw credits → Dash L1 address | Cross | Common | ✅ | `IdentityDetailScreen` → **Withdraw Credits** (dialog, `WithdrawCreditsScreen`) → `platform_wallet_withdraw_credits_with_signer` (Keystore-signed). Destination L1 address typed in + validated. |
| ID-11 | Transfer credits → Platform addresses | Platform | Common | 🔌 | `dash_sdk_identity_transfer_credits_to_addresses` exists in rs-sdk-ffi but is not bridged to Android — no JNI export or Kotlin caller yet. |
| ID-12 | Update identity — disable key | Platform | Thorough | ✅ | `KeyDetailScreen` (drill into a key from `KeysListScreen`) → **Key Status → Disable Key** → confirm → `platform_wallet_update_identity_with_signer` (Keystore-signed). Gated to match consensus. |
| ID-13 | Top up identity (builder path) | Cross | — | ➖ | Retired — builder entry is a stub; covered by `ID-05`/`ID-06`. |

### 4.3 Platform Addresses (DIP-17 credit addresses) — `Domain=Address`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| ADDR-01 | Query address info / multiple infos | Platform | Common | ✅ | `AddressQueriesScreen` → `dash_sdk_address_fetch_info(s)`. |
| ADDR-02 | Transfer credits address → address | Platform | Thorough | ✅ | `WalletDetailScreen` → Platform Balance row **⋯ menu → Transfer Credits** (`TransferPlatformAddressScreen`) → `platform_address_wallet_transfer` (Keystore-signed). Source = DIP-17 platform-payment account picker; destination = own-wallet address picker or pasted 20-byte P2PKH hash. |
| ADDR-03 | Top up address from asset lock | Cross | Thorough | ✅ | `FundFromAssetLockScreen` → `dash_sdk_address_top_up_from_asset_lock`. |
| ADDR-04 | Withdraw address credits → Core L1 | Cross | Thorough | ✅ | `WalletDetailScreen` → Platform Balance row **⋯ menu → Withdraw to Core** (`WithdrawPlatformAddressScreen`) → `platform_address_wallet_withdraw_to_address` (Keystore-signed). Full account balance withdrawn. |
| ADDR-06 | Display / share your Platform receive address | Platform | Common | ✅ | "Receive Dash" sheet → **Platform** tab (`ReceiveAddressSheet`, platform tab): QR + bech32m DIP-17 address + Copy. |

### 4.4 DPNS (usernames) — `Domain=DPNS`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DPNS-01 | Register username (normal) | Platform | Essential | ✅ | `RegisterNameScreen` → `platform_wallet_register_dpns_name_with_signer`. Name resolves to the identity afterward. |
| DPNS-02 | Check availability / validate / normalize | Platform | Essential | ✅ | `RegisterNameScreen` / `DpnsTestScreen`. |
| DPNS-03 | Resolve name → identity | Platform | Essential | ✅ | `QueriesListScreen` (dpnsResolve) / `DpnsTestScreen`. |
| DPNS-04 | Get usernames for an identity | Platform | Essential | ✅ | `IdentityDetailScreen` DPNS section. |
| DPNS-05 | Register username (contested / premium) | Platform | Common | ✅ | `RegisterNameScreen` (auto-detects contested via `dash_sdk_dpns_is_contested_username`). Creates a live vote poll. |
| DPNS-06 | Select main / primary name | Platform | Common | ✅ | `SelectMainNameScreen` (dialog from `IdentityDetailScreen`). |
| DPNS-07 | Search names by prefix | Platform | Common | ✅ | `QueriesListScreen` (dpnsSearch) / `DpnsTestScreen`. |

### 4.5 Voting / Contested Resources — `Domain=Voting`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| VOTE-01 | Vote on contested DPNS username (masternode vote) | Platform | Thorough | ✅ | `ContestDetailScreen` cast-vote → `dash_sdk_contested_resource_cast_vote`. **Requires masternode voting credentials** — environment-limited otherwise. |
| VOTE-02 | Query contested resources | Platform | Thorough | ✅ | `QueriesListScreen` (getContestedResources). |
| VOTE-03 | Query contested-resource vote state | Platform | Thorough | ✅ | `QueriesListScreen` (getContestedResourceVoteState). |
| VOTE-04 | Query voters for a contestant identity | Platform | Thorough | ✅ | `QueriesListScreen` (getContestedResourceVotersForIdentity). |
| VOTE-05 | Query an identity's votes | Platform | Thorough | ✅ | `QueriesListScreen` (getContestedResourceIdentityVotes). |
| VOTE-06 | Query vote polls by end date | Platform | Thorough | ✅ | `QueriesListScreen` (getVotePollsByEndDate). |
| VOTE-07 | Masternode vote (generic builder entry) | Platform | — | ➖ | Retired — covered by `VOTE-01`. |

### 4.6 Data Contracts — `Domain=Contract`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DC-01 | Register / load contract from network | Platform | Common | ✅ | `RegisterContractSourceScreen` / `LocalDataContractsScreen` / `ContractsHomeScreen`. |
| DC-02 | View contract / schema / doc types / history | Platform | Common | ✅ | `DataContractDetailsScreen`, `DocumentTypeDetailsScreen`. |
| DC-03 | Create data contract | Platform | Common | ✅ | `QuickBasicTokenScreen` and *Settings builder → Data Contract Create* → `platform_wallet_create_data_contract_with_signer`. |
| DC-04 | Update data contract | Platform | Thorough | 🧪 | *Settings builder → Data Contract Update* → `platform_wallet_update_data_contract_with_signer`. |

### 4.7 Documents — `Domain=Document`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DOC-01 | Query documents / single document | Platform | Common | ✅ | `DocumentsScreen` / `QueriesListScreen` → `dash_sdk_document_search` / `_fetch`. |
| DOC-02 | Create document (broadcast) | Platform | Common | ✅ | Contracts → contract → document type → **New Document** (`DocumentTypeDetailsScreen` / schema-driven `DocumentFieldsScreen`) → `platform_wallet_create_document_with_signer`. |
| DOC-03 | Replace document | Platform | Thorough | ✅ | Contracts → **Browse Documents** → document → **⋯** action menu (ownership-gated) → **Replace…** → `platform_wallet_document_replace`. |
| DOC-04 | Delete document | Platform | Thorough | ✅ | **Browse Documents** → document → **⋯** → **Delete…** → `platform_wallet_document_delete`. |
| DOC-05 | Transfer document | Platform | Uncommon | ✅ | **Browse Documents** → document → **⋯** → **Transfer…** (shown when `documentsTransferable`) → `platform_wallet_document_transfer`. |
| DOC-06 | Update document price | Platform | Uncommon | ✅ | **Browse Documents** → document → **⋯** → **Set Price…** (shown when `tradeMode`) → `platform_wallet_document_set_price`. |
| DOC-07 | Purchase document | Platform | Uncommon | ✅ | **Browse Documents** → document → **⋯** → **Purchase…** → `platform_wallet_document_purchase`. Buyer ≠ owner enforced. |
| DOC-08 | Document aggregation (umbrella) | Platform | Uncommon | ➖ | Split into `DOC-10`..`DOC-14`. |
| DOC-09 | Create document (local demo) | Platform | — | ➖ | Retired. Replaced by real broadcast flow; see `DOC-02`. |
| DOC-10 | Aggregation — count documents (total) | Platform | Uncommon | 🧪 | **Count Documents** screen → `dash_sdk_document_count`. Requires `documentsCountable: true`. |
| DOC-11 | Aggregation — count documents, filtered (`where`) | Platform | Uncommon | 🧪 | Same Count screen with a `where` clause → `dash_sdk_document_count(where_json=…)`. |
| DOC-12 | Aggregation — count documents, grouped (`group_by`) | Platform | Uncommon | 🧪 | Same Count screen with `group_by` → `dash_sdk_document_count(group_by_json=…)`. |
| DOC-13 | Aggregation — sum of a numeric property | Platform | Uncommon | 🧪 | **Sum / Average Documents** screen (op selector → **Sum**) → `dash_sdk_document_sum`. |
| DOC-14 | Aggregation — average of a numeric property | Platform | Uncommon | 🧪 | Same screen (op selector → **Average**) → `dash_sdk_document_average`. |

### 4.8 Tokens — `Domain=Token`

All token actions support single-signer **and** group (propose / co-sign) modes via `platform-wallet`.

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| TOK-01 | View token balances / details / search | Platform | Common | ✅ | `TokenDetailsScreen`, `TokensScreen`, `TokenSearchScreen`. |
| TOK-02 | Transfer token | Platform | Common | ✅ | `TokenActionScreens(transfer)` → `wallet.tokenTransfer`. |
| TOK-03 | Direct purchase token | Platform | Common | ✅ | `TokenActionScreens(directPurchase)` → `wallet.tokenPurchase`. |
| TOK-04 | Token queries (statuses / prices / contract info / supply / distributions) | Platform | Common | ✅ | `QueriesListScreen` token category → `dash_sdk_token_get_*`. |
| TOK-05 | Mint (issuance) | Platform | Thorough | ✅ | `TokenActionScreens(mint)` → `wallet.tokenMint`. |
| TOK-06 | Burn | Platform | Thorough | ✅ | `TokenActionScreens(burn)` → `wallet.tokenBurn`. |
| TOK-07 | Claim distribution (perpetual / pre-programmed) | Platform | Thorough | ✅ | `TokenActionScreens(claim)` → `wallet.tokenClaim`. |
| TOK-08 | Freeze an identity's balance | Platform | Uncommon | ✅ | `TokenActionScreens(freeze)` → `wallet.tokenFreeze`. |
| TOK-09 | Unfreeze a balance | Platform | Uncommon | ✅ | `TokenActionScreens(unfreeze)` → `wallet.tokenUnfreeze`. |
| TOK-10 | Destroy frozen funds | Platform | Uncommon | ✅ | `TokenActionScreens(destroyFrozenFunds)`. |
| TOK-11 | Set / clear direct-purchase price | Platform | Uncommon | ✅ | `TokenActionScreens(setPrice)` → `wallet.tokenSetPrice`. |
| TOK-12 | Emergency action — Pause | Platform | Uncommon | ✅ | `TokenActionScreens(pause)` → `platform_wallet_token_pause`. |
| TOK-13 | Emergency action — Resume | Platform | Uncommon | ✅ | `TokenActionScreens(resume)` → `platform_wallet_token_resume`. |
| TOK-14 | Config update / max supply | Platform | Uncommon | ✅ | `TokenActionScreens(updateMaxSupply)` → `wallet.tokenUpdateConfig`. |
| TOK-15 | Group action — propose | Platform | Uncommon | ✅ | Token action in `.propose` mode (`CoSignProposalScreen`). |
| TOK-16 | Group action — co-sign existing | Platform | Uncommon | ✅ | `PendingGroupActionsScreen` / `CoSignProposalScreen`. Action executes when accumulated signer power ≥ required. |

### 4.9 Shielded Pool (Orchard) — `Domain=Shielded`

Shielded notes/balance/activity have **no read-side FFI** by design — Rust pushes them to Room via persistence callbacks; the app reads Room. Verify shielded reads against Room, not a query.

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| SH-01 | Shielded sync (start / stop / now) | Shielded | Essential | ✅ | `PlatformWalletManager` → `platform_wallet_manager_shielded_sync_*`. Precondition for any shielded balance/spend. |
| SH-02 | View shielded activity / notes / balance | Shielded | Essential | ✅ | `ShieldedActivityScreen` (Room Flow queries). |
| SH-03 | Shield from Platform balance (Type 15) | Shielded | Essential | ✅ | Send flow (Platform→Shielded) → `walletManager.shieldedShield`. |
| SH-04 | Shield from Core L1 balance | Shielded | Essential | ✅ | Send flow (Core→Shielded). |
| SH-05 | Shielded → shielded transfer (Type 16) | Shielded | Essential | ✅ | Send flow (Shielded→Shielded) → `walletManager.shieldedTransfer`. Optional ≤32-byte memo. |
| SH-06 | Unshield → Platform address (Type 17) | Shielded | Essential | ✅ | Send flow (Shielded→Platform) → `walletManager.shieldedUnshield`. |
| SH-07 | Shield from asset lock (Type 18) | Cross | Common | ✅ | `ShieldedFundScreen` (from `WalletDetailScreen`) → `platform_wallet_manager_shielded_fund_from_asset_lock`. |
| SH-08 | Shielded withdraw → Core L1 (Type 19) | Cross | Common | ✅ | Send flow (Shielded→Core) → `walletManager.shieldedWithdraw` (custom `core_fee_per_byte`). |
| SH-09 | Prover warm-up / readiness | Shielded | Common | ✅ | `warmUpShieldedProver` / `shieldedProverIsReady` (~30s Halo2 key build). |
| SH-10 | Seed shielded pool (anonymity set) | Shielded | Uncommon | ✅ | `SeedShieldedPoolScreen` → `platform_wallet_manager_shielded_seed_pool_notes`. **Devnet/testnet only.** |
| SH-11 | Create identity from shielded pool (Type 20) | Cross | Common | ✅ | `CreateIdentityScreen` → funding source **Shielded balance** (fixed denominations) → `platform_wallet_manager_shielded_identity_create_from_pool`. |
| SH-12 | Clear shielded state (wipe notes + re-sync) | Shielded | Uncommon | ✅ | "Clear" button on the Sync tab (`SyncStatusScreen` → `ShieldedService.clearLocalState`). Stops sync, wipes Room rows, zeroes the mirror; bind credentials kept. |
| SH-13 | Display / share your shielded receive address | Shielded | Common | ✅ | "Receive Dash" sheet → **Shielded** tab (`ReceiveAddressSheet`, shielded tab): QR + `tdash1…`/`dash1…` bech32m address + Copy. |

### 4.10 DashPay — `Domain=DashPay`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DP-01 | Send contact request | Platform | Common | ✅ | `FriendsScreen` (AddFriend) → `platform_wallet_send_contact_request_with_signer`. |
| DP-02 | Accept contact request | Platform | Common | ✅ | `FriendsScreen` → `platform_wallet_accept_contact_request_with_signer`. |
| DP-03 | Send DashPay payment to a contact | Platform | Common | ✅ | `FriendsScreen` → `platform_wallet_send_dashpay_payment`. |
| DP-04 | Create / update DashPay profile | Platform | Common | ✅ | `IdentityDetailScreen` profile editor → `platform_wallet_create_or_update_dashpay_profile_with_signer`. |
| DP-05 | View profile / contacts / requests | Platform | Common | ✅ | `FriendsScreen`, `EstablishedContactEntity` (Room). |
| DP-06 | Reject contact request | Platform | Thorough | ✅ | `FriendsScreen` → `wallet.rejectContactRequest`. |

### 4.11 System / Protocol / Diagnostics — `Domain=System`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| SYS-01 | Status / total credits / quorums / prefunded balance | Platform | Thorough | ✅ | `QueriesListScreen` system category. |
| SYS-02 | Epochs info / current / finalized / proposed blocks | Platform | Thorough | ✅ | `QueriesListScreen` epoch category. |
| SYS-03 | Protocol-version upgrade state / vote status | Platform | Uncommon | ✅ | `QueriesListScreen` protocol category. |
| SYS-04 | Run-all-queries / DPNS test harness | Platform | Thorough | ✅ | `QueriesListScreen` diagnostics (`runAllQueries`, `testDPNSQueries`), `DiagnosticsScreen`. |
| SYS-05 | Storage / Keystore / Wallet-memory explorers | — | Thorough | ✅ | `StorageExplorerScreen`, `KeystoreExplorerScreen`, `WalletMemoryExplorerScreen` (Settings; debug tooling). |
| SYS-06 | Path elements (raw GroveDB) | Platform | Uncommon | 🧪 | **Get GroveDB Path Elements** read view → `dash_sdk_system_get_path_elements`. Enter `path` + `keys` JSON array; returns `[{key, element, type}]`. Use a **bounded** path. |

### 4.12 Multi-wallet on-device Platform scenarios (same network) — `Domain=MultiWallet`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| MW-01 | Credit transfer between two on-device identities (A → B) | Platform | Thorough | ✅ | `IdentityDetailScreen` → **Transfer Credits** (`ID-04`), recipient = wallet B's identity (via `RecipientPicker`). Switch to B; verify credit balance rose. |
| MW-02 | Token transfer between two on-device identities | Platform | Thorough | ✅ | `TOK-02`, recipient = wallet B's identity. Switch to B; verify the token balance arrived. |
| MW-03 | DashPay request → accept → payment, both endpoints on device | Platform | Thorough | ✅ | A sends contact request (`DP-01`) to B; switch to wallet B and accept (`DP-02`); then pay (`DP-03`). Full bidirectional loop. |
| MW-04 | Document transfer / purchase across wallets | Platform | Uncommon | ✅ | A creates + lists a document (`DOC-02`/`DOC-06`); B transfers/purchases it (`DOC-05`/`DOC-07`). |
| MW-05 | Contested DPNS race between two on-device identities | Platform | Uncommon | ✅ | A and B both register the same premium/contested name (`DPNS-05`) → produces a contest observable via `VOTE-02`/`VOTE-03`. |
| MW-06 | Shielded transfer between two on-device wallets | Shielded | Thorough | ✅ | Wallet A's pool → wallet B's shielded address (`SH-05`); copy B's address from Receive → Shielded tab (`SH-13`). Both wallets must be bound + synced. |
| MW-07 | Unshield from A to a Platform address owned by B | Shielded | Uncommon | ✅ | A unshields (`SH-06`) to a Platform address belonging to wallet B. |
| MW-08 | Platform balance sync is per-active-wallet, **not** concurrent | Platform | Thorough | ✅ | `PlatformBalanceSyncService` is configured for ONE wallet. Unlike Core SPV (`CORE-20`), wallet B's Platform balances can be **stale until you switch to B and Sync Now**. |
| MW-09 | Per-wallet Platform isolation (identities / usernames / tokens / contacts) | Platform | Thorough | ✅ | Wallet A's identities, DPNS names, token balances, and DashPay contacts must never surface under wallet B. |
| MW-10 | Same identity restored into two wallets (duplicate seed) | Platform | Uncommon | ✅ | Importing the same mnemonic as a second wallet derives the **same** identity; verify consistency. |
| MW-11 | Shielded withdraw from A to B's Core L1 address | Shielded | Uncommon | ✅ | Wallet A's pool → a Core L1 address owned by wallet B (`SH-08`). Verify B's Core balance rises after SPV sync. |

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
| Platform | ~71 |
| Cross | 6 |
| Shielded | 16 |

---

## 6. Category index

Membership of each feature category across **all** sections (primary section members + cross-cutting tests that live elsewhere).

- **Core / Wallet** — `CORE-01..23`
- **MultiWallet** — `CORE-14..23`, `MW-01..11`
- **Identity** — `ID-01..13`, `SH-11`, `MW-01`, `MW-08`, `MW-09`, `MW-10`
- **Address** (DIP-17 platform addresses) — `ADDR-01..04`, `ADDR-06`, `ID-06`, `ID-08`, `ID-11`
- **DPNS** — `DPNS-01..07`, `MW-05`
- **Voting** — `VOTE-01..07`, `DPNS-05`, `MW-05`
- **Contract** — `DC-01..04`
- **Document** — `DOC-01..14`, `MW-04`
- **Token** — `TOK-01..16`, `MW-02`
- **Shielded** — `SH-01..13`, `CORE-21`, `MW-06`, `MW-07`, `MW-11`
- **DashPay** — `DP-01..06`, `MW-03`
- **System / Diagnostics** — `SYS-01..06`
