# SwiftExampleApp — iOS Test Plan

A catalog of **every action theoretically possible** on Dash via the Platform gRPC API + Dash Core (SPV) layer, each cross-referenced against **what is actually implemented in this iOS app today**, and — for the implemented ones — assigned a **frequency tier** so a QA agent can run a meaningful subset.

This file is meant to be read by an automated QA agent. A human or agent can say *"test the Essential, Platform-only actions"* and the agent filters the tables below by `Tier = Essential` and `Layer = Platform`, then drives each action in the booted simulator (see the `simulator-control` skill) and reports pass/fail.

> **Provenance & maintenance.** Generated from a full source scan of the `v3.1-dev` line (proto, `rs-dpp`, `rs-sdk`, `rs-sdk-ffi`, `rs-platform-wallet[-ffi]`, `swift-sdk`, `SwiftExampleApp`). It is a snapshot — when features land or move, update the affected rows (status, entry point) and re-tier if behavior changes. Treat the codebase as the source of truth if a row looks stale.

---

## 1. How to use this document (for the QA agent)

Every catalog row carries four orthogonal, machine-filterable fields. Select tests by intersecting them.

**Selection grammar** — canonical tokens (case-insensitive):

- **Tier** ∈ `Essential` · `Common` · `Thorough` · `Uncommon` · `Manual`
- **Layer** ∈ `Core` · `Platform` · `Cross` · `Shielded`
- **Status** ∈ `✅` · `🧪` · `⚠️` · `🔌` · `🚫` · `➖`
- **Category** ∈ `Core` · `Identity` · `Address` · `DPNS` · `Voting` · `Contract` · `Document` · `Token` · `Shielded` · `DashPay` · `Group` · `System` · `MultiWallet` (the feature area; shown as `Domain=…` on each §4 section header — "Category" and "Domain" are the same axis)

A test is **automatable now** only if Status is `✅`, `🧪`, or `⚠️` (reachable and drivable in the simulator) **and** `Tier ≠ Manual`. `Tier=Manual` marks implemented features that need a human on a physical device (e.g. a camera) — the automated QA agent must **skip and flag them for manual testing**, never mark them failed. `🔌`/`🚫` rows are listed for completeness — skip them unless asked to confirm absence.

A row's **primary** category is the §4 section it lives in. Some tests are **cross-cutting** — e.g. `MW-02` (token transfer between two wallets) lives in the MultiWallet section but is also a **Token** test, and `CORE-21` is also **Shielded**. Resolve any `Category=…` selection through **§6 Category index**, which lists every member ID per category (primary + cross-cutting), so "run all Token tests" catches `MW-02` and `GRP-03` too. This is the axis behind requests like *"run all non-Uncommon Token tests."*

**Worked examples of a request → selection:**

| Request | Filter | Resolves to |
|---|---|---|
| "test Essential, Platform-only" | `Tier=Essential AND Layer=Platform` | `ID-02, ID-03, ID-04, DPNS-01, DPNS-02, DPNS-03, DPNS-04` |
| "test all Essential" | `Tier=Essential` | the core experience: `CORE-01..07`, `ID-01/02/03/04`, `DPNS-01/02/03/04`, `SH-01..06` |
| "list the manual tests" | `Tier=Manual` | `CORE-08` (skip in automation; run on a physical device) |
| "smoke test the wallet" | `Category=Core AND Status=✅` | `CORE-01..CORE-09` |
| "test all non-Uncommon Token tests" | `Category=Token AND Tier≠Uncommon` | `TOK-01..07`, `MW-02` (via §6 index) |
| "exercise every token admin action" | `Category=Token AND Tier=Uncommon` | `TOK-08..TOK-16` |
| "run all Shielded tests" | `Category=Shielded` | `SH-01..13`, `CORE-21`, `MW-06/07/11` (via §6 index) |
| "run all read queries" | Appendix A | the gRPC read-RPC coverage table |

### Generic pass criteria (apply per action type unless a row overrides)

- **Write / state transition** (`Send`, `Create`, `Transfer`, `Mint`, …): the broadcast returns a successful state-transition result (no consensus error), **and** the resulting state is observable — balance changes, a new row appears in SwiftData, the object is fetchable from the network. A "submitted, then visible after the next block/sync" round-trip is the real pass; a UI spinner that never resolves is a fail.
- **Read / query**: returns a non-error response with the expected shape (and, for proof-backed reads, a verified proof). Empty-but-valid results are a pass when the queried object legitimately doesn't exist.
- **Wallet/local action** (backup phrase, alias, address derivation): the local state is produced and persists across an app relaunch.

### Test infrastructure note

Drive the app with the **`simulator-control`** skill: tap/type/screenshot via `idb`, read persisted state from the app's SwiftData store, and stream Rust logs (the agents found Rust logs land in the app container under `Library/Logs/SwiftDashSDK`). Verify writes against **both** the UI and the persisted/queried state — don't trust the UI alone.

---

## 2. Prerequisites & fixtures

Most Platform actions have hard preconditions. Establish these fixtures before selecting tests, and skip (don't fail) rows whose preconditions can't be met in the environment.

| Fixture | Needed for | Notes |
|---|---|---|
| **Network selected** (testnet / devnet) | everything | Devnet has the format/version caveats noted in memory; mainnet blocks the pool-seeding util. Confirm the SDK protocol-version floor is applied. |
| **Funded Core wallet** | all `Layer=Core` and `Layer=Cross` | A wallet with confirmed, mature, spendable UTXOs. Asset-lock funding needs InstantSend/ChainLock, so masternode sync must complete. |
| **A registered identity with credit balance** | almost all `Layer=Platform` | Created via `ID-01`. Many transitions also need a specific **key purpose/security level** present on the identity. |
| **A loaded data contract** (with a token + a document type) | `Domain=Token`, `Domain=Document`, group | Token actions are gated by the contract's on-chain permission rules — a "coming soon" placeholder means *disallowed by rule*, not unimplemented. |
| **A contested-name scenario** | `DPNS-05`, `VOTE-*` | Register a premium/contested name to create a live vote poll. |
| **Masternode / evonode voting key** | `VOTE-01` | Casting a contested-resource vote requires masternode voting credentials. Standard app QA on a non-masternode identity **cannot** exercise the actual vote broadcast — treat as environment-limited unless a masternode identity is configured. |
| **Shielded pool: configured + bound + prover warmed + synced** | `Domain=Shielded` | `SH-01` sync + `SH-09` prover warm-up are preconditions for any shielded spend. The pool also needs a non-trivial anonymity set (`SH-10` on devnet/testnet) for realistic spends. |
| **A second identity / contact** | credit transfer, token transfer, document transfer, DashPay | Needed as the counterparty/recipient. |

---

## 3. Legend

**Tiers** (how often a real user performs the action → how central it is to QA):

| Tier | Meaning |
|---|---|
| **Essential** | The core happy-path experience every user exercises: the headline value transactions (Core→Core send, identity→identity credit transfer, shield / transfer / unshield) plus everything needed to perform and verify them — create/restore wallet, sync, receive, view balances & history, back up phrase, create & view/discover identity, register/check/resolve usernames, view shielded activity. QA must always run these. |
| **Common** | Frequent actions beyond the core experience — identity top-ups, contested usernames, identity key management, platform-address credit ops, contracts, token transfer/view, DashPay, secondary shielded flows (asset-lock shield, withdraw, prover warm-up). |
| **Thorough** | Occasional, or tied to a specialized role (contract author, voter, contact-graph user, multi-wallet power user) — voting, contract update, document edit/delete, mint/burn/claim, group reads, multi-wallet management. |
| **Uncommon** | Rare / exotic / administrative edge cases (most token governance, marketplace, emergency, group, raw-protocol). |
| **Manual** | Not a frequency — a special bucket for implemented features that **can't be driven in the simulator** and need a human on a physical device (e.g. camera, biometrics, NFC). The automated agent **skips and flags** these for a person; it never fails them. Select with `Tier=Manual`. |

**Layers** (for the "X-only" filter):

| Layer | Meaning |
|---|---|
| **Core** | L1 transparent SPV wallet only. |
| **Platform** | Pure L2: identity, contracts, documents, tokens, DPNS, voting, groups. |
| **Cross** | Bridges the two layers (asset-lock funding, credit withdrawal back to L1). |
| **Shielded** | Orchard private pool (often also crosses layers, but tagged distinctly for selection). |

**Status:**

| Status | Meaning | Runnable now? |
|---|---|---|
| ✅ | Implemented and reachable in the app UI. | Yes |
| 🧪 | Reachable **only** via *Settings → Platform State Transitions* (the demo builder uses a test signer but broadcasts for real). | Yes (builder) |
| ⚠️ | UI exists but is **local-only / mock** — does not broadcast. | Partially (UI only) |
| 🔌 | FFI and/or Swift wrapper exists, but **no UI** to trigger it. | No (SDK only) |
| 🚫 | Not implemented anywhere (no FFI, no UI). | No |
| ➖ | Retired — the thing this row tracked was removed or folded into another row. | n/a |

> **Entry-point reality check.** A set of Platform write transitions (identity credit withdrawal, document create/replace/delete/transfer/price/purchase, data-contract create/update, identity key-disable) are reachable in the app **only through `Settings → Platform State Transitions` → `TransitionDetailView`** (marked 🧪). They broadcast for real, but there is no per-identity "happy path" button for them. The QA agent must navigate to the builder for those rows. (Identity credit *transfer*, `ID-04`, now has a production button in `IdentityDetailView` — see that row.) The builder and the read-only **Platform Queries** catalog both live under the **Settings** tab's **Platform** section (scroll past *Network* and *Data*).

---

## 4. Catalog

### 4.1 Core / Wallet — `Domain=Core`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| CORE-01 | Create wallet (new mnemonic) | Core | Essential | ✅ | `CreateWalletView`. New 12/24-word phrase shown; wallet appears in Wallets tab. |
| CORE-02 | Restore wallet (existing mnemonic) | Core | Essential | ✅ | `CreateWalletView` (Import Existing toggle). After sync, derived addresses + balance populate. |
| CORE-03 | Backup / view recovery phrase | Core | Essential | ✅ | `SeedBackupView`. Phrase matches creation. |
| CORE-04 | Receive (derive address + QR) | Core | Essential | ✅ | `ReceiveAddressView` → `core_wallet_next_receive_address`. Fresh external address + scannable QR. |
| CORE-05 | Send Core L1 transaction | Core | Essential | ✅ | Send flow (`SendTransactionView`, mode Core→Core) → `core_wallet_send_to_addresses`. Tx broadcasts; balance drops; appears in history. *Anchor: the canonical Essential action.* |
| CORE-06 | View balance / tx history / UTXOs | Core | Essential | ✅ | `WalletDetailView`, `TransactionListView`, `AccountDetailView` (SwiftData). |
| CORE-07 | SPV sync (start / stop / progress) | Core | Essential | ✅ | Global sync indicator (`ContentView`) → `platform_wallet_manager_spv_*`. Headers/filters/masternodes advance to tip. |
| CORE-08 | QR scan recipient | Core | Manual | ✅ | `QRScannerView`, reachable in the Send flow — but scanning needs a real camera the simulator doesn't have, so it can't be automated (`Tier=Manual`). On a device: Send → QR-scan button → point at a Dash address QR → recipient field populates. |
| CORE-09 | Multiple HD accounts (within one wallet) | Core | Common | ✅ | Account selection / `AccountDetailView`; balances per `account_index`. Distinct from holding multiple *wallets* — see CORE-14+. |
| CORE-10 | Multi-recipient Core send | Core | Common | ✅ | Send flow (`SendTransactionView`, Core→Core) → "Add recipient" appends extra address/amount rows → `SendViewModel.coreRecipients` → `core_wallet_send_to_addresses` (parallel arrays; Rust coin-selects + builds the multi-output tx). One tx with N outputs; balance drops by sum+fee. Verified: 2-output testnet send (txid `30010050…17f840fc`, txlock, 3 vouts) credited both recipients. |
| CORE-11 | Custom fee on transparent send | Core | Uncommon | 🚫 | Not exposed on the transparent send path (custom Core fee only on shielded withdraw `SH-08` and platform-address funding). |
| CORE-12 | CoinJoin / mixing | Core | Uncommon | 🚫 | Not implemented anywhere (SPV crate or FFI). |
| CORE-13 | Send explicitly via InstantSend | Core | Uncommon | 🚫 | IS is observe-only (used to obtain asset-lock proofs); no user-facing send toggle. |

#### Multiple wallets on one device

The app is a full multi-wallet client: `PlatformWalletManager` holds N wallets concurrently (keyed by `wallet_id`), one SPV runtime per network. Most rows elsewhere in this plan are written for a single active wallet — these rows cover the multi-wallet dimension explicitly. (Distinct from CORE-09, which is multiple *accounts* inside one wallet.)

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| CORE-14 | Hold multiple wallets at once (wallet list) | Core | Thorough | ✅ | `WalletsContentView` lists every wallet for the current network; `PlatformWalletManager.wallets` holds N keyed by `wallet_id`. |
| CORE-15 | Create / import a second wallet (alongside existing) | Core | Thorough | ✅ | Wallets tab → "Add Wallet" → `CreateWalletView`. New wallet coexists; must not replace or corrupt the first. |
| CORE-16 | Switch active wallet | Core | Thorough | ✅ | Tap a wallet row → `WalletDetailView` scopes all `@Query`s to that `walletId`. Navigation-based — there is **no** global wallet picker, so "switching" = opening another wallet's detail. |
| CORE-17 | Remove / delete a wallet | Core | Uncommon | ✅ | `WalletDetailView` → Delete Wallet → `platform_wallet_manager_remove_wallet`; cascades Keychain mnemonic + that wallet's identities + SwiftData rows. Verify the other wallets are untouched. |
| CORE-18 | Per-wallet isolation (identities / addresses / balances / shielded) | Core | Thorough | ✅ | Confirm wallet A's identities, addresses, Core/Platform balances and shielded state never surface under wallet B (`@Query` predicates filtered by `walletId`). Key correctness check for multi-wallet. |
| CORE-19 | Send between two on-device wallets | Core | Thorough | ✅ | Normal send from wallet A to wallet B's receive address (no intra-app picker — you must paste/scan B's address). B's balance increases after sync. Variants: identity→identity (`ID-04`) or shielded between two local wallets. |
| CORE-20 | Concurrent SPV sync across all wallets | Core | Thorough | ✅ | One SPV runtime per network filters every wallet's addresses; `spvProgress` is manager-global, not per-wallet. With 2+ wallets, confirm each reaches the tip and detects its own funds. |
| CORE-21 | Multiple wallets bound to the shielded pool concurrently | Shielded | Uncommon | ✅ | `platform_wallet_manager_bind_shielded` is per `wallet_id`; the manager syncs all bound wallets. UI (`ShieldedService.boundWalletId`) displays one wallet's shielded state at a time — switching should swap cleanly, not merge balances. |
| CORE-22 | Re-add a previously deleted wallet (same network) | Core | Uncommon | ✅ | After `CORE-17`, re-import the same mnemonic on the same network. Re-derives the same (network-scoped) `wallet_id`, re-creates the wallet, and must re-discover identities/addresses/balances cleanly — no stale Keychain keys or orphaned SwiftData rows left over from the delete. Verify the wallet is fully functional again, not a half-restored duplicate. |
| CORE-23 | Re-add a deleted wallet that also exists on another network | Core | Uncommon | ✅ | Same mnemonic present as a wallet on two networks (e.g. testnet + devnet) → **distinct** network-scoped `wallet_id`s, each with its own Keychain mnemonic copy. Delete it on network X (`CORE-17`) and verify the network-Y wallet is untouched (still listed, mnemonic intact, functional); then re-add on X and confirm both coexist. Exercises the `walletRowCountAcrossNetworks` cross-network mnemonic-purge guard in `PlatformWalletManager.deleteWallet`. |

### 4.2 Identity — `Domain=Identity`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| ID-01 | Create identity (Core-funded asset lock) | Cross | Essential | ✅ | `CreateIdentityView` / `IdentityRegistrationController` → `platform_wallet_register_identity_with_signer`. New identity + credit balance appear. *Gateway to all Platform tests.* |
| ID-02 | Load / discover identity from wallet | Platform | Essential | ✅ | `LoadIdentityView` / `SearchWalletsForIdentitiesView` → `platform_wallet_discover_identities`. |
| ID-03 | View identity (info / balance / revision / keys) | Platform | Essential | ✅ | `IdentityDetailView`, `KeysListView`, `KeyDetailView`. |
| ID-04 | Transfer credits identity → identity | Platform | Essential | ✅ | `IdentityDetailView` → **Transfer Credits** (sheet, `TransferCreditsView`) → `wallet.transferCredits` → `platform_wallet_transfer_credits_with_signer` (keychain-signed). Recipient entered via `RecipientPickerView` (local identity / paste base58 id / DPNS name). *Anchor: the "platform-to-platform" Essential action.* Recipient balance increases; sender's drops. (Also reachable via the *Settings → Platform State Transitions → Identity Credit Transfer* builder → `dash_sdk_identity_transfer_credits`.) |
| ID-05 | Top up identity (asset lock) | Cross | Common | ✅ | `TopUpIdentityView` (sheet from `IdentityDetailView`). *Anchor: top-up = Common.* |
| ID-06 | Top up identity (from Platform addresses) | Cross | Common | ✅ | `AddressQueriesView` → TopUpIdentityFromAddresses → `dash_sdk_identity_top_up_from_addresses`. |
| ID-07 | Update identity — add public key | Platform | Common | ✅ | `AddIdentityKeyView` (from `KeysListView`) → `updateIdentity(addPublicKeys:)`. |
| ID-08 | Create identity (from Platform addresses) | Cross | Common | ✅ | `AddressQueriesView` → CreateIdentityFromAddresses → `dash_sdk_identity_create_from_addresses`. |
| ID-09 | Set / edit local alias | Platform | Common | ✅ | `IdentityDetailView` (Add Alias). Local only — persists across relaunch; no broadcast. |
| ID-10 | Withdraw credits → Dash L1 address | Cross | Common | 🧪 | *Settings builder → Identity Credit Withdrawal* → `dash_sdk_identity_withdraw`. Credits burned; L1 payout observed. |
| ID-11 | Transfer credits → Platform addresses | Platform | Common | ✅ | `AddressQueriesView` → TransferIdentityToAddresses → `dash_sdk_identity_transfer_credits_to_addresses`. |
| ID-12 | Update identity — disable key | Platform | Thorough | 🧪 | *Settings builder → Identity Update* (disable path) → `executeIdentityUpdate`. |
| ID-13 | Top up identity (builder path) | Cross | — | 🧪 | Builder entry is a stub (`notImplemented`). Use `ID-05`/`ID-06`. Listed so QA doesn't mistake the stub for a defect. |

### 4.3 Platform Addresses (DIP-17 credit addresses) — `Domain=Address`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| ADDR-01 | Query address info / multiple infos | Platform | Common | ✅ | `GetAddressInfoViewModel` / `GetAddressesInfosViewModel` → `dash_sdk_address_fetch_info(s)`. |
| ADDR-02 | Transfer credits address → address | Platform | Thorough | ✅ | `AddressQueriesView` → TransferAddressFunds → `dash_sdk_address_transfer_funds`. |
| ADDR-03 | Top up address from asset lock | Cross | Thorough | ✅ | `FundFromAssetLockPlatformAddressView` → `dash_sdk_address_top_up_from_asset_lock`. |
| ADDR-04 | Withdraw address credits → Core L1 | Cross | Thorough | ✅ | `AddressQueriesView` → WithdrawAddressFunds → `dash_sdk_address_withdraw_funds`. |
| ADDR-05 | Address balance-change history (recent / compacted / branch / trunk) | Platform | Uncommon | 🔌 | FFI `dash_sdk_address_fetch_recent_balance_changes` / `_compacted_balance_changes` / `_branch_state` / `_trunk_state`; no UI. |
| ADDR-06 | Display / share your Platform receive address | Platform | Common | ✅ | "Receive Dash" sheet → **Platform** tab (`ReceiveAddressView`, `ReceiveAddressTab.platform`, "Your Platform Address"): QR + bech32m DIP-17 address + Copy. The receive counterpart to the credit-transfer / top-up funding paths. |

### 4.4 DPNS (usernames) — `Domain=DPNS`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DPNS-01 | Register username (normal) | Platform | Essential | ✅ | `RegisterNameView` → `platform_wallet_register_dpns_name_with_signer`. Name resolves to the identity afterward. |
| DPNS-02 | Check availability / validate / normalize | Platform | Essential | ✅ | `RegisterNameView` / `DPNSTestView`. |
| DPNS-03 | Resolve name → identity | Platform | Essential | ✅ | `PlatformQueriesView` (dpnsResolve) / `DPNSTestView`. |
| DPNS-04 | Get usernames for an identity | Platform | Essential | ✅ | `IdentityDetailView` DPNS section. |
| DPNS-05 | Register username (contested / premium) | Platform | Common | ✅ | `RegisterNameView` (auto-detects contested via `dash_sdk_dpns_is_contested_username`). *Anchor: contested name = Common.* Creates a live vote poll. |
| DPNS-06 | Select main / primary name | Platform | Common | ✅ | `SelectMainNameView` (sheet from `IdentityDetailView`). |
| DPNS-07 | Search names by prefix | Platform | Common | ✅ | `PlatformQueriesView` (dpnsSearch) / `DPNSTestView`. |

### 4.5 Voting / Contested Resources — `Domain=Voting`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| VOTE-01 | Vote on contested DPNS username (masternode vote) | Platform | Thorough | ✅ | `ContestDetailView` cast-vote (from per-name link in `IdentityDetailView`) → `dash_sdk_contested_resource_cast_vote`. *Anchor: voting = Thorough.* **Requires masternode voting credentials** — environment-limited otherwise. |
| VOTE-02 | Query contested resources | Platform | Thorough | ✅ | `PlatformQueriesView` (getContestedResources). |
| VOTE-03 | Query contested-resource vote state | Platform | Thorough | ✅ | `PlatformQueriesView` (getContestedResourceVoteState) — contenders + abstain/lock tallies. |
| VOTE-04 | Query voters for a contestant identity | Platform | Thorough | ✅ | `PlatformQueriesView` (getContestedResourceVotersForIdentity). |
| VOTE-05 | Query an identity's votes | Platform | Thorough | ✅ | `PlatformQueriesView` (getContestedResourceIdentityVotes). |
| VOTE-06 | Query vote polls by end date | Platform | Thorough | ✅ | `PlatformQueriesView` (getVotePollsByEndDate). |
| VOTE-07 | Masternode vote (generic builder entry) | Platform | — | 🧪 | Builder entry is a stub (`default → notImplemented`). Use `VOTE-01`. |

### 4.6 Data Contracts — `Domain=Contract`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DC-01 | Register / load contract from network | Platform | Common | ✅ | `RegisterContractSourceView` / `LocalDataContractsView` / `ContractsTabView`. |
| DC-02 | View contract / schema / doc types / history | Platform | Common | ✅ | `DataContractDetailsView`, `DocumentTypeDetailsView`. |
| DC-03 | Create data contract | Platform | Common | ✅ | `QuickBasicTokenView` and *Settings builder → Data Contract Create* → `platform_wallet_create_data_contract_with_signer`. |
| DC-04 | Update data contract | Platform | Thorough | 🧪 | *Settings builder → Data Contract Update* → `platform_wallet_update_data_contract_with_signer`. (Note the consensus-sensitive byteArray-widening case in memory when authoring updates.) |

### 4.7 Documents — `Domain=Document`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DOC-01 | Query documents / single document | Platform | Common | ✅ | `DocumentsView` / `PlatformQueriesView` → `dash_sdk_document_search` / `_fetch`. |
| DOC-02 | Create document (broadcast) | Platform | Common | ✅ | Production UI: Contracts → contract → document type → **New Document** (`DocumentTypeDetailsView` / schema-driven `CreateDocumentView`) → `platform_wallet_create_document_with_signer` (routes through `rs-platform-wallet` `IdentityWallet::create_document_with_signer` → SDK `put_to_platform_and_wait_for_response`, signed by the wallet's keychain signer). Driven end-to-end: created a `preorder` doc (`saltedDomainHash`) on `GWRSAV…S31Ec` from funded idx1 — network-confirmed, doc id `7i1hJgvVt8fJms26kGwkEZ6jVZxrfd3BrqfmAfpqXMoG`, persisted & appears in the documents list. *(Settings builder → Document Create / `dash_sdk_document_create` remains as a test-signer alternative.)* |
| DOC-03 | Replace document | Platform | Thorough | 🧪 | *Settings builder* → `dash_sdk_document_replace_on_platform`. |
| DOC-04 | Delete document | Platform | Thorough | 🧪 | *Settings builder* → `dash_sdk_document_delete`. |
| DOC-05 | Transfer document | Platform | Uncommon | 🧪 | *Settings builder* → `dash_sdk_document_transfer_to_identity`. |
| DOC-06 | Update document price | Platform | Uncommon | 🧪 | *Settings builder* / `DocumentWithPriceView` → `dash_sdk_document_update_price_of_document`. |
| DOC-07 | Purchase document | Platform | Uncommon | 🧪 | *Settings builder* → `dash_sdk_document_purchase`. |
| DOC-08 | Document count / sum / average aggregation | Platform | Uncommon | 🔌 | FFI `dash_sdk_document_count` / `_sum` / `_average`; no UI. |
| DOC-09 | Create document (local demo) | Platform | — | ➖ | Retired. The old `DocumentsView` local-only mock was replaced by the real broadcast flow (`CreateDocumentView`); see `DOC-02`. |

### 4.8 Tokens — `Domain=Token`

All token actions support single-signer **and** group (propose / co-sign) modes via `platform-wallet`. Reachability is gated by the contract's on-chain permission rules (a "coming soon" placeholder = disallowed by rule, not unimplemented).

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| TOK-01 | View token balances / details / search | Platform | Common | ✅ | `TokenDetailsView`, `TokensView`, `TokenSearchView`. |
| TOK-02 | Transfer token | Platform | Common | ✅ | `TokenTransferActionView` → `wallet.tokenTransfer`. |
| TOK-03 | Direct purchase token | Platform | Common | ✅ | `TokenPurchaseActionView` → `wallet.tokenPurchase`. |
| TOK-04 | Token queries (statuses / prices / contract info / supply / distributions) | Platform | Common | ✅ | `PlatformQueriesView` token category → `dash_sdk_token_get_*`. |
| TOK-05 | Mint (issuance) | Platform | Thorough | ✅ | `TokenMintActionView` → `wallet.tokenMint`. |
| TOK-06 | Burn | Platform | Thorough | ✅ | `TokenBurnActionView` → `wallet.tokenBurn`. |
| TOK-07 | Claim distribution (perpetual / pre-programmed) | Platform | Thorough | ✅ | `TokenClaimActionView` → `wallet.tokenClaim`. |
| TOK-08 | Freeze an identity's balance | Platform | Uncommon | ✅ | `TokenFreezeActionView` → `wallet.tokenFreeze`. |
| TOK-09 | Unfreeze a balance | Platform | Uncommon | ✅ | `TokenUnfreezeActionView` → `wallet.tokenUnfreeze`. |
| TOK-10 | Destroy frozen funds | Platform | Uncommon | ✅ | `TokenDestroyFrozenFundsActionView`. |
| TOK-11 | Set / clear direct-purchase price | Platform | Uncommon | ✅ | `TokenSetPriceActionView` → `wallet.tokenSetPrice`. |
| TOK-12 | Emergency action — Pause | Platform | Uncommon | ✅ | `TokenPauseActionView` → `platform_wallet_token_pause`. |
| TOK-13 | Emergency action — Resume | Platform | Uncommon | ✅ | `TokenResumeActionView` → `platform_wallet_token_resume`. |
| TOK-14 | Config update / max supply | Platform | Uncommon | ✅ | `TokenUpdateMaxSupplyActionView` → `wallet.tokenUpdateConfig` (one `TokenConfigurationChangeItem` per tx). |
| TOK-15 | Group action — propose | Platform | Uncommon | ✅ | Token action in `.propose` mode (`CoSignProposalView`). Applies to Mint/Burn/Freeze/Unfreeze/DestroyFrozen/Emergency/Config/SetPrice. |
| TOK-16 | Group action — co-sign existing | Platform | Uncommon | ✅ | `PendingGroupActionsView` / `CoSignProposalView`. Action executes when accumulated signer power ≥ required. |
| TOK-17 | Calculate token ID (utility) | Platform | Uncommon | 🔌 | FFI `dash_sdk_calculate_token_id`; no dedicated UI. |

### 4.9 Shielded Pool (Orchard) — `Domain=Shielded`

Shielded notes/balance/activity have **no read-side FFI** by design — Rust pushes them to SwiftData via `on_persist_shielded_*` callbacks; the app reads SwiftData. Verify shielded reads against SwiftData, not a query.

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| SH-01 | Shielded sync (start / stop / now) | Shielded | Essential | ✅ | `PlatformWalletManagerShieldedSync` → `platform_wallet_manager_shielded_sync_*`. Precondition for any shielded balance/spend. |
| SH-02 | View shielded activity / notes / balance | Shielded | Essential | ✅ | `ShieldedActivityView` (SwiftData @Query). |
| SH-03 | Shield from Platform balance (Type 15) | Shielded | Essential | ✅ | Send flow (Platform→Shielded) → `walletManager.shieldedShield`. *Anchor: shielded tx = Essential.* |
| SH-04 | Shield from Core L1 balance | Shielded | Essential | ✅ | Send flow (Core→Shielded). |
| SH-05 | Shielded → shielded transfer (Type 16) | Shielded | Essential | ✅ | Send flow (Shielded→Shielded) → `walletManager.shieldedTransfer`. Optional ≤32-byte memo. |
| SH-06 | Unshield → Platform address (Type 17) | Shielded | Essential | ✅ | Send flow (Shielded→Platform) → `walletManager.shieldedUnshield`. |
| SH-07 | Shield from asset lock (Type 18) | Cross | Common | ✅ | `ShieldedFundFromAssetLockView` (from `WalletDetailView`) → `platform_wallet_manager_shielded_fund_from_asset_lock`. |
| SH-08 | Shielded withdraw → Core L1 (Type 19) | Cross | Common | ✅ | Send flow (Shielded→Core) → `walletManager.shieldedWithdraw` (custom `core_fee_per_byte`). |
| SH-09 | Prover warm-up / readiness | Shielded | Common | ✅ | `warmUpShieldedProver` / `shieldedProverIsReady` (~30s Halo2 key build; precondition for spends). |
| SH-10 | Seed shielded pool (anonymity set) | Shielded | Uncommon | ✅ | `SeedShieldedPoolView` → `platform_wallet_manager_shielded_seed_pool_notes`. **Devnet/testnet only** — hard-errors on mainnet. |
| SH-11 | Create identity from shielded pool (Type 20) | Cross | Uncommon | 🔌 | FFI `platform_wallet_manager_shielded_identity_create_from_pool`; no dedicated UI. |
| SH-12 | Clear shielded state (wipe notes + re-sync) | Shielded | Uncommon | ✅ | "Clear" button on the Sync tab (`CoreContentView` → `ShieldedService.clearLocalState` → `clearShielded`). Stops sync, wipes every wallet's shielded notes + sync state, zeroes the Swift mirror; bind credentials are kept so "Sync Now" rebinds and re-scans. (On-disk SQLite tree is intentionally retained.) Verify balance/activity reset, then restore after Sync Now. |
| SH-13 | Display / share your shielded receive address | Shielded | Common | ✅ | "Receive Dash" sheet → **Shielded** tab (`ReceiveAddressView`, `ReceiveAddressTab.shielded`): QR + full `tdash1…`/`dash1…` bech32m address + Copy Address. Hand your shielded address to a payer, or grab wallet B's address for `MW-06`. |

### 4.10 DashPay — `Domain=DashPay`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| DP-01 | Send contact request | Platform | Common | ✅ | `FriendsView` (AddFriendView) → `platform_wallet_send_contact_request_with_signer`. |
| DP-02 | Accept contact request | Platform | Common | ✅ | `FriendsView` → `platform_wallet_accept_contact_request_with_signer`. |
| DP-03 | Send DashPay payment to a contact | Platform | Common | ✅ | `FriendsView` → `platform_wallet_send_dashpay_payment`. |
| DP-04 | Create / update DashPay profile | Platform | Common | ✅ | `IdentityDetailView` profile editor → `platform_wallet_create_or_update_dashpay_profile_with_signer`. |
| DP-05 | View profile / contacts / requests | Platform | Common | ✅ | `FriendsView`, `EstablishedContact` (SwiftData). |
| DP-06 | Reject contact request | Platform | Thorough | ✅ | `FriendsView` → `wallet.rejectContactRequest`. |

### 4.11 Group — `Domain=Group`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| GRP-01 | View group info / members | Platform | Thorough | ✅ | `GroupDetailView` (drill into member identities). |
| GRP-02 | Group queries (info / infos / actions / signers) | Platform | Thorough | ✅ | `PlatformQueriesView` group category → `dash_sdk_group_get_*`. |
| GRP-03 | Token group action — propose / co-sign | Platform | Uncommon | ✅ | Same as `TOK-15` / `TOK-16`. |
| GRP-04 | Standalone group lifecycle management | Platform | Uncommon | 🚫 | Not implemented — groups exist only as a token access-control construct + read queries. There is no group-create/membership transition. |

### 4.12 System / Protocol / Diagnostics — `Domain=System`

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| SYS-01 | Status / total credits / quorums / prefunded balance | Platform | Thorough | ✅ | `PlatformQueriesView` system category. |
| SYS-02 | Epochs info / current / finalized / proposed blocks | Platform | Thorough | ✅ | `PlatformQueriesView` epoch category. |
| SYS-03 | Protocol-version upgrade state / vote status | Platform | Uncommon | ✅ | `PlatformQueriesView` protocol category. |
| SYS-04 | Run-all-queries / DPNS test harness | Platform | Thorough | ✅ | `PlatformQueriesView` diagnostics (`runAllQueries`, `testDPNSQueries`), `DiagnosticsView`. |
| SYS-05 | Storage / Keychain / Wallet-memory explorers | — | Thorough | ✅ | `StorageExplorerView`, `KeychainExplorerView`, `WalletMemoryExplorerView` (Settings; debug tooling). |
| SYS-06 | Path elements (raw GroveDB) | Platform | Uncommon | 🔌 | FFI `dash_sdk_system_get_path_elements`; no UI. |

### 4.13 Multi-wallet on-device Platform scenarios (same network) — `Domain=MultiWallet`

These compose base actions using **two (or more) wallets on the same device and the same network**, so both sides of a Platform/shielded interaction are local and end-to-end verifiable without an external counterparty. They reuse the underlying action (cited by ID) — the value is exercising the cross-wallet path and verifying both endpoints on one device.

Together with the wallet-lifecycle rows in §4.1 (`CORE-14..23`), these form the full multi-wallet test surface. None are Essential/Common — multi-wallet is a power-user / QA topology, not an everyday flow. "Act as wallet B" means navigating into wallet B (and its identity); there is **no** global wallet/identity selector (see `CORE-16`).

| ID | Action | Layer | Tier | Status | Entry point & test notes |
|---|---|---|---|---|---|
| MW-01 | Credit transfer between two on-device identities (A → B) | Platform | Thorough | ✅ | `IdentityDetailView` → **Transfer Credits** (`ID-04`), recipient = wallet B's identity (via `RecipientPickerView` — local / paste id / DPNS). Switch to B; verify its credit balance rose and A's dropped. Fully local round-trip. |
| MW-02 | Token transfer between two on-device identities | Platform | Thorough | ✅ | `TOK-02`, recipient = wallet B's identity. Switch to B; verify the token balance arrived. |
| MW-03 | DashPay request → accept → payment, both endpoints on device | Platform | Thorough | ✅ | A's identity sends a contact request (`DP-01`) to B's; switch to wallet B's identity and accept (`DP-02`); then pay (`DP-03`). Full bidirectional loop entirely local. |
| MW-04 | Document transfer / purchase across wallets | Platform | Uncommon | 🧪 | A creates + lists a document (`DOC-02`/`DOC-06`); B transfers/purchases it (`DOC-05`/`DOC-07`). Ownership and credits move between A and B. |
| MW-05 | Contested DPNS race between two on-device identities | Platform | Uncommon | ✅ | A and B (different wallets) both register the same premium/contested name (`DPNS-05`) → produces a contest observable end-to-end on-device via `VOTE-02`/`VOTE-03`. |
| MW-06 | Shielded transfer between two on-device wallets | Shielded | Thorough | ✅ | Wallet A's pool → wallet B's shielded address (`SH-05`); copy B's address from its Receive → Shielded tab (`SH-13`). Both wallets must be bound + synced (`CORE-21`, `SH-01`); after syncing B, its shielded balance rises. NB only one wallet's shielded state is displayed at a time. |
| MW-07 | Unshield from A to a Platform address owned by B | Shielded | Uncommon | ✅ | A unshields (`SH-06`) to a Platform address belonging to wallet B; verify B receives the credits (subject to the MW-08 sync caveat). |
| MW-08 | Platform balance sync is per-active-wallet, **not** concurrent | Platform | Thorough | ✅ | `PlatformBalanceSyncService` is configured for ONE wallet (`configure(...walletId:)`, re-run on switch). Unlike Core SPV (`CORE-20`, all wallets at once), wallet B's Platform address/credit balances can be **stale until you switch to B and Sync Now**. Verify this is the intended behavior, not a bug. |
| MW-09 | Per-wallet Platform isolation (identities / usernames / tokens / contacts) | Platform | Thorough | ✅ | Extends `CORE-18` to Platform reads: wallet A's identities, DPNS names, token balances, and DashPay contacts must never surface under wallet B. |
| MW-10 | Same identity restored into two wallets (duplicate seed) | Platform | Uncommon | ✅ | Importing the same mnemonic as a second wallet derives the **same** identity; verify state stays consistent and balances are not double-counted or conflicting across the two wallets. |
| MW-11 | Shielded withdraw from A to B's Core L1 address | Shielded | Uncommon | ✅ | Wallet A's pool → a Core L1 address owned by wallet B (`SH-08`). Completes the cross-wallet shielded exit set (→ shielded `MW-06`, → Platform `MW-07`, → Core `MW-11`). Verify B's Core balance rises after SPV sync. |

---

## 5. Summary matrix

Counts are of rows reachable in the app (Status `✅`/`🧪`/`⚠️`); `🔌`/`🚫`/stub rows are excluded. `Tier=Manual` rows are reachable but **not automatable** (need a physical device) — counted on their own row below, excluded from the by-layer automatable totals. Each catalog row carries its own `Tier` + `Layer`, so any intersection (e.g. *Essential ∩ Platform*) is derivable directly from §4.

**By tier:**

| Tier | Count (approx.) | Automatable? |
|---|---|---|
| Essential | 21 | yes |
| Common | 31 | yes |
| Thorough | 35 | yes |
| Uncommon | 25 | yes |
| Manual | 1 (`CORE-08`) | no — physical device |

**By layer (automatable only):**

| Layer | Count (approx.) |
|---|---|
| Core | 17 |
| Platform | ~72 |
| Cross | 7 |
| Shielded | 16 |

**Headline intersection — `Essential ∩ Platform` (the most common QA request):** `ID-02`, `ID-03`, `ID-04`, `DPNS-01`, `DPNS-02`, `DPNS-03`, `DPNS-04`. Essential Core lives in §4.1 (`CORE-01..08`); Essential cross-layer identity creation is `ID-01`; Essential shielded is `SH-01..06`.

---

## 6. Category index

Membership of each feature category across **all** sections (primary section members + cross-cutting tests that live elsewhere). To run a `Category=X` selection, take the list below and intersect with `Tier` / `Layer` / `Status` as needed. `A-01..09` means every id in that span.

- **Core / Wallet** — `CORE-01..23`
- **MultiWallet** — `CORE-14..23`, `MW-01..11`
- **Identity** — `ID-01..13`, `SH-11`, `MW-01`, `MW-08`, `MW-09`, `MW-10`
- **Address** (DIP-17 platform addresses) — `ADDR-01..06`, `ID-06`, `ID-08`, `ID-11`
- **DPNS** — `DPNS-01..07`, `MW-05`
- **Voting** — `VOTE-01..07`, `DPNS-05`, `MW-05`
- **Contract** — `DC-01..04`
- **Document** — `DOC-01..09`, `MW-04`
- **Token** — `TOK-01..17`, `MW-02`, `GRP-03`
- **Shielded** — `SH-01..13`, `CORE-21`, `MW-06`, `MW-07`, `MW-11`
- **DashPay** — `DP-01..06`, `MW-03`
- **Group** — `GRP-01..04`, `TOK-15`, `TOK-16`
- **System / Diagnostics** — `SYS-01..06`

Worked example — *"run all non-Uncommon Token tests"*: take **Token** = `TOK-01..17`, `MW-02`, `GRP-03`; drop the `Uncommon` ones (`TOK-08..17`, `GRP-03`) → run **`TOK-01..07` + `MW-02`**.

---

## Appendix A — gRPC read-RPC coverage

The complete Platform read surface, mapped to where each RPC is exercised in the app. Almost all are reachable through the **Platform Queries catalog** (`PlatformQueriesView`); exceptions are noted. Proof-backed reads should additionally verify the returned proof.

### Identity
| RPC | Tier | Status | Where |
|---|---|---|---|
| getIdentity | Essential | ✅ | `IdentityDetailView` / catalog |
| getIdentityBalance | Essential | ✅ | `IdentityDetailView` / catalog |
| getIdentityBalanceAndRevision | Common | ✅ | catalog |
| getIdentityKeys | Common | ✅ | `KeysListView` / catalog |
| getIdentityByPublicKeyHash | Common | ✅ | catalog |
| getIdentityByNonUniquePublicKeyHash | Thorough | ✅ | catalog |
| getIdentitiesBalances | Thorough | ✅ | catalog |
| getIdentitiesContractKeys | Thorough | ✅ | catalog |
| getIdentityNonce | Uncommon | ✅ | catalog (also used internally) |
| getIdentityContractNonce | Uncommon | ✅ | catalog (also used internally) |

### Data Contract
| RPC | Tier | Status | Where |
|---|---|---|---|
| getDataContract | Common | ✅ | `DataContractDetailsView` / catalog |
| getDataContracts | Thorough | ✅ | catalog |
| getDataContractHistory | Thorough | ✅ | catalog |

### Document
| RPC | Tier | Status | Where |
|---|---|---|---|
| getDocuments (incl. V1 COUNT/SUM/AVG, group_by, having) | Common | ✅ / 🔌 | `DocumentsView` / catalog; aggregation surface is FFI-only (`DOC-08`) |
| getDocumentHistory | Thorough | ✅ | catalog |

### Token
| RPC | Tier | Status | Where |
|---|---|---|---|
| getIdentityTokenBalances | Common | ✅ | `TokenDetailsView` / catalog |
| getIdentitiesTokenBalances | Thorough | ✅ | catalog |
| getIdentityTokenInfos | Thorough | ✅ | catalog |
| getIdentitiesTokenInfos | Thorough | ✅ | catalog |
| getTokenStatuses | Thorough | ✅ | catalog |
| getTokenDirectPurchasePrices | Thorough | ✅ | catalog |
| getTokenContractInfo | Thorough | ✅ | catalog |
| getTokenTotalSupply | Thorough | ✅ | catalog |
| getTokenPreProgrammedDistributions | Uncommon | ✅ | catalog |
| getTokenPerpetualDistributionLastClaim | Uncommon | ✅ | catalog |

### Voting / Contested Resources
| RPC | Tier | Status | Where |
|---|---|---|---|
| getContestedResources | Thorough | ✅ | catalog (`VOTE-02`) |
| getContestedResourceVoteState | Thorough | ✅ | catalog (`VOTE-03`) |
| getContestedResourceVotersForIdentity | Thorough | ✅ | catalog (`VOTE-04`) |
| getContestedResourceIdentityVotes | Thorough | ✅ | catalog (`VOTE-05`) |
| getVotePollsByEndDate | Thorough | ✅ | catalog (`VOTE-06`) |

### Group
| RPC | Tier | Status | Where |
|---|---|---|---|
| getGroupInfo | Thorough | ✅ | `GroupDetailView` / catalog |
| getGroupInfos | Thorough | ✅ | catalog |
| getGroupActions | Uncommon | ✅ | catalog / `PendingGroupActionsView` |
| getGroupActionSigners | Uncommon | ✅ | catalog |

### Epoch / Protocol / Quorums
| RPC | Tier | Status | Where |
|---|---|---|---|
| getEpochsInfo | Thorough | ✅ | catalog (`SYS-02`) |
| getFinalizedEpochInfos | Thorough | ✅ | catalog |
| getEvonodesProposedEpochBlocksByIds | Uncommon | ✅ | catalog |
| getEvonodesProposedEpochBlocksByRange | Uncommon | ✅ | catalog |
| getProtocolVersionUpgradeState | Uncommon | ✅ | catalog (`SYS-03`) |
| getProtocolVersionUpgradeVoteStatus | Uncommon | ✅ | catalog |
| getCurrentQuorumsInfo | Thorough | ✅ | catalog (unproved) |

### System / Status / Broadcast
| RPC | Tier | Status | Where |
|---|---|---|---|
| getStatus | Thorough | ✅ | catalog (`SYS-01`, unproved) |
| getTotalCreditsInPlatform | Thorough | ✅ | catalog |
| getPrefundedSpecializedBalance | Thorough | ✅ | catalog |
| waitForStateTransitionResult | Essential | ✅ | implicit in every write round-trip |
| broadcastStateTransition | Essential | ✅ | implicit in every write (`@sdk-ignore` RPC) |
| getPathElements | Uncommon | 🔌 | FFI only (`SYS-06`) |
| getConsensusParams | Uncommon | 🚫 | `@sdk-ignore` (served via Tenderdash RPC) |

### Address Sync (DIP-17)
| RPC | Tier | Status | Where |
|---|---|---|---|
| getAddressInfo | Common | ✅ | `ADDR-01` |
| getAddressesInfos | Common | ✅ | `ADDR-01` |
| getRecentAddressBalanceChanges | Uncommon | 🔌 | FFI only (`ADDR-05`) |
| getRecentCompactedAddressBalanceChanges | Uncommon | 🔌 | FFI only (`ADDR-05`) |
| getAddressesTrunkState | Uncommon | 🔌 | FFI only (`ADDR-05`) |
| getAddressesBranchState | Uncommon | 🔌 | FFI only (`ADDR-05`) |

### Shielded Pool
| RPC | Tier | Status | Where |
|---|---|---|---|
| getShieldedPoolState | Common | ✅ | drives shielded balance / sync |
| getShieldedNotesCount | Common | ✅ | sync progress denominator |
| getShieldedEncryptedNotes | Common | ✅ | consumed by shielded sync (`SH-01`) |
| getShieldedAnchors | Thorough | ✅ | consumed during spends |
| getMostRecentShieldedAnchor | Thorough | ✅ | consumed during spends |
| getShieldedNullifiers | Thorough | ✅ | consumed during sync/spends |

---

## Appendix B — Theoretically possible but not runnable in-app

For completeness (the "everything gRPC + Core can do" requirement), these exist at the protocol/FFI level but have **no app entry point** today:

**🔌 SDK-only (FFI/wrapper exists, no UI):**
- `ADDR-05` address balance-change history (recent / compacted / branch / trunk)
- `DOC-08` document count / sum / average aggregation
- `TOK-17` calculate token ID
- `SH-11` create identity from shielded pool (Type 20)
- `SYS-06` raw GroveDB path elements

**🚫 Not implemented anywhere:**
- `CORE-11` custom fee on transparent Core send
- `CORE-12` CoinJoin / mixing
- `CORE-13` explicit send-via-InstantSend
- `GRP-04` standalone group lifecycle management
- `getConsensusParams` (served via Tenderdash RPC, not the SDK)

**Protocol-level write transitions present in DPP but not surfaced as distinct app actions** (the address/asset-lock family — `IdentityCreditTransferToAddresses`, `IdentityCreateFromAddresses`, `IdentityTopUpFromAddresses`, `AddressFundsTransfer`, `AddressFundingFromAssetLock`, `AddressCreditWithdrawal`) are largely covered by the `ID-*`/`ADDR-*` rows above; the shielded family (`Shield`, `ShieldedTransfer`, `Unshield`, `ShieldFromAssetLock`, `ShieldedWithdrawal`, `IdentityCreateFromShieldedPool`) maps to the `SH-*` rows. Anything not mapped is either internal or `🔌`/`🚫` above.
