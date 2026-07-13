# SwiftExampleApp — iOS Test Plan

A catalog of **every action theoretically possible** on Dash via the Platform gRPC API + Dash Core (SPV) layer, each cross-referenced against **what is actually implemented in this iOS app today**, and — for the implemented ones — assigned a **frequency tier** so a QA agent can run a meaningful subset.

This file is meant to be read by an automated QA agent. A human or agent can say *"test the Essential, Platform-only actions"* and the agent filters the tables below by `Tier = Essential` and `Layer = Platform`, then drives each action in the booted simulator (see the `simulator-control` skill) and reports pass/fail.

> **📊 Live QA status dashboard.** A read-only web dashboard visualises these tests' on-chain results — the `dash-qa` data contract seeded from this plan: **<https://dashpay.github.io/qa-dashboard-site/>**. It renders a tier × category status matrix, per-test latest result + run history, summary counts, and filters (network / build / tier / category / result). Source: [`dashpay/qa-dashboard-site`](https://github.com/dashpay/qa-dashboard-site).

> **Provenance & maintenance.** Generated from a full source scan of the `v3.1-dev` line (proto, `rs-dpp`, `rs-sdk`, `rs-sdk-ffi`, `rs-platform-wallet[-ffi]`, `swift-sdk`, `SwiftExampleApp`). It is a snapshot — when features land or move, update the affected rows (status, entry point) and re-tier if behavior changes. Treat the codebase as the source of truth if a row looks stale.

---

## 1. How to use this document (for the QA agent)

Every catalog row carries four orthogonal, machine-filterable fields. Select tests by intersecting them.

**Selection grammar** — canonical tokens (case-insensitive):

- **Tier** ∈ `Essential` · `Common` · `Thorough` · `Uncommon` · `Manual`
- **Layer** ∈ `Core` · `Platform` · `Cross` · `Shielded`
- **Status** ∈ `✅` · `🧪` · `⚠️` · `🔌` · `🚫` · `➖`
- **Category** ∈ `Core` · `Identity` · `Address` · `DPNS` · `Voting` · `Contract` · `Document` · `Token` · `Shielded` · `DashPay` · `System` (the feature area; shown as `Domain=…` on each §4 section header — "Category" and "Domain" are the same axis)
- **Tags** ∈ `multiwallet` · `group` · `contested` · `withdrawal` · `distribution` · `aggregation` · `read-only` · `regression` · `proof` · `freeze` · `funding` · `masternode` (orthogonal cross-cutting labels in the **Tags** column; a row may carry several, comma-separated. `multiwallet` and `group` used to be categories — they are now tags, so a multi-wallet token test lives in **Token** and is found with `Tag=multiwallet`.)

A test is **automatable now** only if Status is `✅`, `🧪`, or `⚠️` (reachable and drivable in the simulator) **and** `Tier ≠ Manual`. `Tier=Manual` marks implemented features that need a human on a physical device (e.g. a camera) — the automated QA agent must **skip and flag them for manual testing**, never mark them failed. `🔌`/`🚫` rows are listed for completeness — skip them unless asked to confirm absence.

A row's **primary home** is the §4 section it lives in (its `Domain=…`). **Category** stays a distinct selection axis, though: a handful of rows are cross-cutting and belong to a category *other* than their section — e.g. `ID-06`/`ID-08`/`ID-11` are **Address** tests that live in §4.2 Identity, and `SH-11` is an **Identity** test in the Shielded section. Resolve any `Category=…` selection through the **§6 category index**, which lists every member per category (primary + cross-cutting). Separately, the old MultiWallet/Group *sections* are gone — those cross-cutting concerns are now **tags**: e.g. `TOK-17` (token transfer between two wallets) lives in **Token** and carries `Tag=multiwallet`. Select cross-cutting sets with the Tags column — `Tag=multiwallet`, `Tag=group`, `Tag=read-only`, etc. — intersected with `Tier`/`Layer`/`Status`/`Category` as needed. This is the axis behind requests like *"run all multi-wallet token tests"* (`Category=Token AND Tag=multiwallet`).

**Worked examples of a request → selection:**

| Request | Filter | Resolves to |
|---|---|---|
| "test Essential, Platform-only" | `Tier=Essential AND Layer=Platform` | `ID-02, ID-03, ID-04, ADDR-07, DPNS-01, DPNS-02, DPNS-03, DPNS-04` |
| "test all Essential" | `Tier=Essential` | the core experience: `CORE-01..07`, `ID-01/02/03/04`, `ADDR-07`, `DPNS-01/02/03/04`, `SH-01..06` |
| "list the manual tests" | `Tier=Manual` | `CORE-08`, `DP-10` (skip in automation; run on a physical device) |
| "smoke test the wallet" | `Category=Core AND Status=✅` | `CORE-01..CORE-10` |
| "test all non-Uncommon Token tests" | `Category=Token AND Tier≠Uncommon` | `TOK-01..07`, `TOK-17`, `TOK-18`, `TOK-19` |
| "exercise every token admin action" | `Category=Token AND Tier=Uncommon` | `TOK-08..TOK-16`, `TOK-20` |
| "run all Shielded tests" | `Category=Shielded` | `SH-01..17` |
| "run all multi-wallet tests" | `Tag=multiwallet` | `CORE-14..23`, `ID-14/15`, `TOK-17`, `DPNS-08`, `DP-11`, `DOC-15`, `SH-14/15/16`, `SYS-07/08` |
| "run all read queries" | `Tag=read-only` (or Appendix A) | the read-only rows + the gRPC read-RPC coverage table |

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
| **A loaded data contract** (with a token + a document type) | `Domain=Token`, `Domain=Document`, `Tag=group` | Token actions are gated by the contract's on-chain permission rules — a "coming soon" placeholder means *disallowed by rule*, not unimplemented. |
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

> **Entry-point reality check.** A few Platform write transitions (data-contract create/update, `DC-03`/`DC-04`) are reachable in the app **only through `Settings → Platform State Transitions` → `TransitionDetailView`** (marked 🧪). They broadcast for real, but there is no per-identity "happy path" button for them. The QA agent must navigate to the builder for those rows. The full **document** write family now has production UI: create (`DOC-02`) via Contracts → contract → document type → **New Document**, and replace/delete/transfer/set-price/purchase (`DOC-03`..`DOC-07`) via Contracts → **Browse Documents** (`contracts.browseDocuments`) → document → **⋯** action menu (ownership-gated) → `platform_wallet_document_*`. Identity credit *transfer* (`ID-04`), *withdrawal* (`ID-10`), and *key-disable* (`ID-12`) also have production buttons (`IdentityDetailView` / `KeyDetailView`). The DIP-17 platform-address *transfer* (`ADDR-02`) and *withdrawal* (`ADDR-04`) now have production sheets off the `WalletDetailView` Platform Balance row's ⋯ menu — see those rows. The builder and the read-only **Platform Queries** catalog both live under the **Settings** tab's **Platform** section (scroll past *Network* and *Data*).

---

## 4. Catalog

### 4.1 Core / Wallet — `Domain=Core`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| CORE-01 | Create wallet (new mnemonic) | Core | Essential | ✅ |  | `CreateWalletView`. New 12/24-word phrase shown; wallet appears in Wallets tab. |
| CORE-02 | Restore wallet (existing mnemonic) | Core | Essential | ✅ |  | `CreateWalletView` (Import Existing toggle). After sync, derived addresses + balance populate. |
| CORE-03 | Backup / view recovery phrase | Core | Essential | ✅ |  | `SeedBackupView`. Phrase matches creation. |
| CORE-04 | Receive (derive address + QR) | Core | Essential | ✅ |  | `ReceiveAddressView` → `core_wallet_next_receive_address`. Fresh external address + scannable QR. |
| CORE-05 | Send Core L1 transaction | Core | Essential | ✅ |  | Send flow (`SendTransactionView`, mode Core→Core) → `core_wallet_send_to_addresses`. Tx broadcasts; balance drops; appears in history. *Anchor: the canonical Essential action.* |
| CORE-06 | View balance / tx history / UTXOs | Core | Essential | ✅ | read-only | `WalletDetailView`, `TransactionListView`, `AccountDetailView` (SwiftData). |
| CORE-07 | SPV sync (start / stop / progress) | Core | Essential | ✅ |  | Global sync indicator (`ContentView`) → `platform_wallet_manager_spv_*`. Headers/filters/masternodes advance to tip. |
| CORE-08 | QR scan recipient | Core | Manual | ✅ |  | `QRScannerView`, reachable in the Send flow — but scanning needs a real camera the simulator doesn't have, so it can't be automated (`Tier=Manual`). On a device: Send → QR-scan button → point at a Dash address QR → recipient field populates. |
| CORE-09 | Multiple HD accounts (within one wallet) | Core | Common | ✅ |  | Account selection / `AccountDetailView`; balances per `account_index`. Distinct from holding multiple *wallets* — see CORE-14+. |
| CORE-10 | Multi-recipient Core send | Core | Common | ✅ |  | Send flow (`SendTransactionView`, Core→Core) → "Add recipient" appends extra address/amount rows → `SendViewModel.coreRecipients` → `core_wallet_send_to_addresses` (parallel arrays; Rust coin-selects + builds the multi-output tx). One tx with N outputs; balance drops by sum+fee. Verified: 2-output testnet send (txid `30010050…17f840fc`, txlock, 3 vouts) credited both recipients. |

#### Multiple wallets on one device

The app is a full multi-wallet client: `PlatformWalletManager` holds N wallets concurrently (keyed by `wallet_id`), one SPV runtime per network. Most rows elsewhere in this plan are written for a single active wallet — these rows cover the multi-wallet dimension explicitly. (Distinct from CORE-09, which is multiple *accounts* inside one wallet.)

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| CORE-14 | Hold multiple wallets at once (wallet list) | Core | Thorough | ✅ | multiwallet | `WalletsContentView` lists every wallet for the current network; `PlatformWalletManager.wallets` holds N keyed by `wallet_id`. |
| CORE-15 | Create / import a second wallet (alongside existing) | Core | Thorough | ✅ | multiwallet | Wallets tab → "Add Wallet" → `CreateWalletView`. New wallet coexists; must not replace or corrupt the first. |
| CORE-16 | Switch active wallet | Core | Thorough | ✅ | multiwallet | Tap a wallet row → `WalletDetailView` scopes all `@Query`s to that `walletId`. Navigation-based — there is **no** global wallet picker, so "switching" = opening another wallet's detail. |
| CORE-17 | Remove / delete a wallet | Core | Uncommon | ✅ | multiwallet | `WalletDetailView` → Delete Wallet → `platform_wallet_manager_remove_wallet`; cascades Keychain mnemonic + that wallet's identities + SwiftData rows. Verify the other wallets are untouched. |
| CORE-18 | Per-wallet isolation (identities / addresses / balances / shielded) | Core | Thorough | ✅ | multiwallet | Confirm wallet A's identities, addresses, Core/Platform balances and shielded state never surface under wallet B (`@Query` predicates filtered by `walletId`). Key correctness check for multi-wallet. |
| CORE-19 | Send between two on-device wallets | Core | Thorough | ✅ | multiwallet | Normal send from wallet A to wallet B's receive address (no intra-app picker — you must paste/scan B's address). B's balance increases after sync. Variants: identity→identity (`ID-04`) or shielded between two local wallets. |
| CORE-20 | Concurrent SPV sync across all wallets | Core | Thorough | ✅ | multiwallet | One SPV runtime per network filters every wallet's addresses; `spvProgress` is manager-global, not per-wallet. With 2+ wallets, confirm each reaches the tip and detects its own funds. |
| CORE-22 | Re-add a previously deleted wallet (same network) | Core | Uncommon | ✅ | multiwallet | After `CORE-17`, re-import the same mnemonic on the same network. Re-derives the same (network-scoped) `wallet_id`, re-creates the wallet, and must re-discover identities/addresses/balances cleanly — no stale Keychain keys or orphaned SwiftData rows left over from the delete. Verify the wallet is fully functional again, not a half-restored duplicate. |
| CORE-23 | Re-add a deleted wallet that also exists on another network | Core | Uncommon | ✅ | multiwallet | Same mnemonic present as a wallet on two networks (e.g. testnet + devnet) → **distinct** network-scoped `wallet_id`s, each with its own Keychain mnemonic copy. Delete it on network X (`CORE-17`) and verify the network-Y wallet is untouched (still listed, mnemonic intact, functional); then re-add on X and confirm both coexist. Exercises the `walletRowCountAcrossNetworks` cross-network mnemonic-purge guard in `PlatformWalletManager.deleteWallet`. |

### 4.2 Identity — `Domain=Identity`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| ID-01 | Create identity (Core-funded asset lock) | Cross | Essential | ✅ |  | `CreateIdentityView` / `IdentityRegistrationController` → `platform_wallet_register_identity_with_signer`. New identity + credit balance appear. *Gateway to all Platform tests.* |
| ID-02 | Load / discover identity from wallet | Platform | Essential | ✅ |  | `LoadIdentityView` / `SearchWalletsForIdentitiesView` → `platform_wallet_discover_identities`. |
| ID-03 | View identity (info / balance / revision / keys) | Platform | Essential | ✅ | read-only | `IdentityDetailView`, `KeysListView`, `KeyDetailView`. |
| ID-04 | Transfer credits identity → identity | Platform | Essential | ✅ |  | `IdentityDetailView` → **Transfer Credits** (sheet, `TransferCreditsView`) → `wallet.transferCredits` → `platform_wallet_transfer_credits_with_signer` (keychain-signed). Recipient entered via `RecipientPickerView` (local identity / paste base58 id / DPNS name). *Anchor: the "platform-to-platform" Essential action.* Recipient balance increases; sender's drops. (Also reachable via the *Settings → Platform State Transitions → Identity Credit Transfer* builder → `dash_sdk_identity_transfer_credits`.) |
| ID-05 | Top up identity (asset lock) | Cross | Common | ✅ |  | `TopUpIdentityView` (sheet from `IdentityDetailView`). *Anchor: top-up = Common.* |
| ID-06 | Top up identity (from Platform addresses) | Cross | Common | ✅ |  | `AddressQueriesView` → TopUpIdentityFromAddresses → `dash_sdk_identity_top_up_from_addresses`. |
| ID-07 | Update identity — add public key | Platform | Common | ✅ |  | `AddIdentityKeyView` (from `KeysListView`) → `updateIdentity(addPublicKeys:)`. |
| ID-08 | Create identity (from Platform addresses) | Cross | Common | ✅ |  | `AddressQueriesView` → CreateIdentityFromAddresses → `dash_sdk_identity_create_from_addresses`. |
| ID-09 | Set / edit local alias | Platform | Common | ✅ |  | `IdentityDetailView` (Add Alias). Local only — persists across relaunch; no broadcast. |
| ID-10 | Withdraw credits → Dash L1 address | Cross | Common | ✅ | withdrawal | `IdentityDetailView` → **Withdraw Credits** (sheet, `WithdrawCreditsView`) → `wallet.withdrawCredits` → `platform_wallet_withdraw_credits_with_signer` (keychain-signed). Destination L1 address typed in + validated against the wallet's network; amount validated against balance. Identity credit balance drops by amount + fee; L1 payout is pooled and processed asynchronously by the network (no immediate txid). Requires the identity to have a TRANSFER/CRITICAL key — newly-derived identities get one (keyId 3); older identities may need one added first via `ID-07`. (Also reachable via the *Settings → Platform State Transitions → Identity Credit Withdrawal* builder → `dash_sdk_identity_withdraw` with a test signer.) |
| ID-11 | Transfer credits → Platform addresses | Platform | Common | ✅ |  | `AddressQueriesView` → TransferIdentityToAddresses → `dash_sdk_identity_transfer_credits_to_addresses`. |
| ID-12 | Update identity — disable key | Platform | Thorough | ✅ |  | `KeyDetailView` (drill into a key from `KeysListView`) → **Key Status → Disable Key** → confirm (permanent / irreversible) → `wallet.updateIdentity(disablePublicKeyIds:)` → `platform_wallet_update_identity_with_signer` (keychain-signed). The button is gated to match consensus: it's hidden/disabled for master-level keys, the last enabled authentication key, and the last enabled transfer key (each shows an inline reason), and already-disabled keys show a read-only "Disabled" row. On success the identity's keys are re-fetched so the disabled badge appears, then the view pops back. A swipe-to-Disable shortcut on each eligible row in `KeysListView` routes into the same confirm + submit (reaches keys whose row tap opens `PrivateKeyView` instead of the detail). (Also reachable via *Settings → Platform State Transitions → Identity Update* (disable path) → `executeIdentityUpdate` with a test signer.) |
| ID-13 | Top up identity (builder path) | Cross | — | ➖ |  | Retired — builder entry is a stub (`notImplemented`); identity top-up is covered by `ID-05`/`ID-06`. Kept here to document the stub; not seeded to the QA catalog. |
| ID-14 | Credit transfer between two on-device identities (A → B) | Platform | Thorough | ✅ | multiwallet | `IdentityDetailView` → **Transfer Credits** (`ID-04`), recipient = wallet B's identity (via `RecipientPickerView` — local / paste id / DPNS). Switch to B; verify its credit balance rose and A's dropped. Fully local round-trip. |
| ID-15 | Same identity restored into two wallets (duplicate seed) | Platform | Uncommon | ✅ | multiwallet | Importing the same mnemonic as a second wallet derives the **same** identity; verify state stays consistent and balances are not double-counted or conflicting across the two wallets. |

### 4.3 Platform Addresses (DIP-17 credit addresses) — `Domain=Address`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| ADDR-01 | Query address info / multiple infos | Platform | Common | ✅ | read-only | `GetAddressInfoViewModel` / `GetAddressesInfosViewModel` → `dash_sdk_address_fetch_info(s)`. |
| ADDR-02 | Transfer credits address → address | Platform | Thorough | ✅ |  | `WalletDetailView` → Platform Balance row **⋯ menu → Transfer Credits** (sheet, `TransferPlatformAddressView`) → `ManagedPlatformAddressWallet.transfer` → `platform_address_wallet_transfer` (keychain-signed). Source = DIP-17 platform-payment account picker; destination = own-wallet address picker or pasted 20-byte P2PKH hash. Input selection (Auto), the `Σ inputs == Σ outputs` balancing, fee strategy, and nonce all happen Rust-side — surplus stays on the source addresses (credit-balance model), so there's no change address to pick, and no private-key entry. Submit gated on amount + fee ≤ account balance and recipient ∉ funded source inputs. On success a DIP-17 resync runs. (Also reachable via the 🧪 debug builder *Settings → Platform State Transitions → Address → Transfer Address Funds (raw)* → `dash_sdk_address_transfer_funds`, which pastes a raw 64-char private key.) |
| ADDR-03 | Top up address from asset lock | Cross | Thorough | ✅ |  | `FundFromAssetLockPlatformAddressView` → `dash_sdk_address_top_up_from_asset_lock`. |
| ADDR-04 | Withdraw address credits → Core L1 | Cross | Thorough | ✅ | withdrawal | `WalletDetailView` → Platform Balance row **⋯ menu → Withdraw to Core** (sheet, `WithdrawPlatformAddressView`) → `ManagedPlatformAddressWallet.withdraw` → `platform_address_wallet_withdraw_to_address` (keychain-signed). Source = DIP-17 platform-payment account picker; the **full** account balance is withdrawn (no per-address amount, no change). Core L1 destination = own wallet (`core_wallet_next_receive_address`) or pasted external address, network-checked Rust-side. `coreFeePerByte` defaults to 1. Gated on the Core (SPV) wallet being initialized — shows a "Core not ready" state otherwise. Identity/address credit balance drops; L1 payout is pooled and processed asynchronously (no immediate txid). On success a DIP-17 resync runs. (Also reachable via the 🧪 debug builder *Settings → Platform State Transitions → Address → Withdraw Address Funds (raw)* → `dash_sdk_address_withdraw_funds`, which pastes a raw 64-char private key.) |
| ADDR-06 | Display / share your Platform receive address | Platform | Common | ✅ | read-only | "Receive Dash" sheet → **Platform** tab (`ReceiveAddressView`, `ReceiveAddressTab.platform`, "Your Platform Address"): QR + bech32m DIP-17 address + Copy. The receive counterpart to the credit-transfer / top-up funding paths. |
| ADDR-07 | Platform address balance sync (BLAST) — start / progress; address balances populate to tip | Platform | Essential | ✅ |  | Sync tab → **Platform Sync Status**: `PlatformBalanceSyncService` runs the BLAST platform-address balance sync (`sync_address_balances` — trunk/branch tree scan + per-block catch-up). After a sync, the platform address balances + Platform Balance + Chain Tip Height populate to chain tip. The Platform-layer analogue of `CORE-07` (Core SPV sync) and `SH-01` (Shielded sync) — the core "platform balances sync to tip" happy path every user relies on. |
| ADDR-08 | Clear & resync platform address balances | Platform | Common | ✅ |  | Sync tab → **Platform Sync Status** → **Clear**, then **Sync Now**: drops the cached platform-address balance state and re-runs the BLAST sync (`sync_address_balances`) from scratch. Verifies balances repopulate correctly after a clear — the recovery path for stale / wrong cached platform balances (e.g. the stale-balance display seen during heavy funding). |
| ADDR-09 | Top-up balance reflects exactly once (no double-credit) | Cross | Thorough | ✅ | regression | Regression guard for the top-up double-credit bug. Run `ADDR-03` ("Top Up from Core", `FundFromAssetLockPlatformAddressView`, `dash_sdk_address_top_up_from_asset_lock`) on a Core-funded wallet, then **wait through at least one automatic BLAST platform-address sync (~15s)** and re-read the `WalletDetailView` Platform Balance. **Pass:** the balance increases by the topped-up amount **exactly once** and stays there — through further automatic syncs, a manual Sync-tab "Clear"+"Sync Now", and an app restart. Cross-check the topped-up address against on-chain truth (`sdk.addresses.getWithProof`) when in doubt. **Fail (the bug):** the topped-up address shows ~2× its on-chain balance. Root cause: the funding credit is recorded on-chain as an `AddBalanceToAddress` **delta** (`AddToCredits`) in Drive's recent-address-balance-changes tree, and the sync replayed that delta **on top of** an absolute balance that already included it (the ST-proof reconcile write, or a full scan's trunk absolute). An earlier watermark-invalidation fix (#4004/#4005) forced a full rescan but could not stop the rescan itself from re-applying the delta — behavioral QA 2026-07-06 showed a durable 2× that survived Clear+resync and restart. Fixed properly by the **balance height pin**: every absolute carries `AddressFunds::as_of_height` (the proof/scan height it is current *as of*); the sync's recent/compacted apply loops drop any delta at or below the pin, and reconcile freshness is decided by pin ordering (a later pin wins even when it revises the balance downward — which also self-heals rows poisoned by the old bug). The pin round-trips through persistence as `PersistentPlatformAddress.lastSeenHeight`. Needs a **Funded Core wallet** fixture; the wallet's Core (SPV) balance must be non-zero to build the asset lock. |

### 4.4 DPNS (usernames) — `Domain=DPNS`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DPNS-01 | Register username (normal) | Platform | Essential | ✅ |  | `RegisterNameView` → `platform_wallet_register_dpns_name_with_signer`. Name resolves to the identity afterward. |
| DPNS-02 | Check availability / validate / normalize | Platform | Essential | ✅ | read-only | `RegisterNameView` / `DPNSTestView`. |
| DPNS-03 | Resolve name → identity | Platform | Essential | ✅ | read-only | `PlatformQueriesView` (dpnsResolve) / `DPNSTestView`. |
| DPNS-04 | Get usernames for an identity | Platform | Essential | ✅ | read-only | `IdentityDetailView` DPNS section. |
| DPNS-05 | Register username (contested / premium) | Platform | Common | ✅ | contested | `RegisterNameView` (auto-detects contested via `dash_sdk_dpns_is_contested_username`). *Anchor: contested name = Common.* Creates a live vote poll. |
| DPNS-06 | Select main / primary name | Platform | Common | ✅ |  | `SelectMainNameView` (sheet from `IdentityDetailView`). |
| DPNS-07 | Search names by prefix | Platform | Common | ✅ | read-only | `PlatformQueriesView` (dpnsSearch) / `DPNSTestView`. |
| DPNS-08 | Contested DPNS race between two on-device identities | Platform | Uncommon | ✅ | multiwallet, contested | A and B (different wallets) both register the same premium/contested name (`DPNS-05`) → produces a contest observable end-to-end on-device via `VOTE-02`/`VOTE-03`. |

### 4.5 Voting / Contested Resources — `Domain=Voting`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| VOTE-01 | Vote on contested DPNS username (masternode vote) | Platform | Thorough | ✅ | contested, masternode | `ContestDetailView` cast-vote (from per-name link in `IdentityDetailView`) → `dash_sdk_contested_resource_cast_vote`. *Anchor: voting = Thorough.* **Requires masternode voting credentials** — environment-limited otherwise. |
| VOTE-02 | Query contested resources | Platform | Thorough | ✅ | contested, read-only | `PlatformQueriesView` (getContestedResources). |
| VOTE-03 | Query contested-resource vote state | Platform | Thorough | ✅ | contested, read-only | `PlatformQueriesView` (getContestedResourceVoteState) — contenders + abstain/lock tallies. |
| VOTE-04 | Query voters for a contestant identity | Platform | Thorough | ✅ | contested, read-only | `PlatformQueriesView` (getContestedResourceVotersForIdentity). |
| VOTE-05 | Query an identity's votes | Platform | Thorough | ✅ | contested, read-only | `PlatformQueriesView` (getContestedResourceIdentityVotes). |
| VOTE-06 | Query vote polls by end date | Platform | Thorough | ✅ | contested, read-only | `PlatformQueriesView` (getVotePollsByEndDate). |
| VOTE-07 | Masternode vote (generic builder entry) | Platform | — | ➖ | contested | Retired — builder entry is a stub (`default → notImplemented`); masternode voting is covered by `VOTE-01`. Kept here to document the stub; not seeded to the QA catalog. |

### 4.6 Data Contracts — `Domain=Contract`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DC-01 | Register / load contract from network | Platform | Common | ✅ |  | `RegisterContractSourceView` / `LocalDataContractsView` / `ContractsTabView`. |
| DC-02 | View contract / schema / doc types / history | Platform | Common | ✅ | read-only | `DataContractDetailsView`, `DocumentTypeDetailsView`. |
| DC-03 | Create data contract | Platform | Common | ✅ |  | `QuickBasicTokenView` and *Settings builder → Data Contract Create* → `platform_wallet_create_data_contract_with_signer`. |
| DC-04 | Update data contract | Platform | Thorough | 🧪 |  | *Settings builder → Data Contract Update* → `platform_wallet_update_data_contract_with_signer`. (Note the consensus-sensitive byteArray-widening case in memory when authoring updates.) |

### 4.7 Documents — `Domain=Document`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DOC-01 | Query documents / single document | Platform | Common | ✅ | read-only | `DocumentsView` / `PlatformQueriesView` → `dash_sdk_document_search` / `_fetch`. |
| DOC-02 | Create document (broadcast) | Platform | Common | ✅ |  | Production UI: Contracts → contract → document type → **New Document** (`DocumentTypeDetailsView` / schema-driven `CreateDocumentView`) → `platform_wallet_create_document_with_signer` (routes through `rs-platform-wallet` `IdentityWallet::create_document_with_signer` → SDK `put_to_platform_and_wait_for_response`, signed by the wallet's keychain signer). Driven end-to-end: created a `preorder` doc (`saltedDomainHash`) on `GWRSAV…S31Ec` from funded idx1 — network-confirmed, doc id `7i1hJgvVt8fJms26kGwkEZ6jVZxrfd3BrqfmAfpqXMoG`, persisted & appears in the documents list. *(Settings builder → Document Create / `dash_sdk_document_create` remains as a test-signer alternative.)* |
| DOC-03 | Replace document | Platform | Thorough | ✅ |  | Production UI: Contracts → **Browse Documents** (`contracts.browseDocuments`) → document → **⋯** action menu (`documentAction.menu`, ownership-gated) → **Replace…** → `ReplaceDocumentView` → `platform_wallet_document_replace` (routes through `rs-platform-wallet` `IdentityWallet::replace_document_with_signer`: schema-sanitizes the new properties, bumps the revision, signs with the wallet keychain signer on the 8 MB worker stack). Driven end-to-end on testnet: replaced the `card` doc `FgVSYG6sTZZ9…` on `5jpKat9U82PG` (`attack` 7→42, `name`→"As-replaced"), revision 1→2, broadcast confirmed + canonical JSON persisted. *(Settings builder → `dash_sdk_document_replace_on_platform` remains as a test-signer alternative.)* |
| DOC-04 | Delete document | Platform | Thorough | ✅ |  | Production UI: **Browse Documents** → document → **⋯** → **Delete…** → `DeleteDocumentView` → `platform_wallet_document_delete` (`IdentityWallet::delete_document_with_signer`, keychain signer, 8 MB worker stack). Driven end-to-end on testnet: deleted the `card` doc `FgVSYG6sTZZ9…`, broadcast confirmed, local row removed. *(Settings builder → `dash_sdk_document_delete` remains as a test-signer alternative.)* |
| DOC-05 | Transfer document | Platform | Uncommon | ✅ |  | Production UI: **Browse Documents** → document → **⋯** → **Transfer…** (shown when the doc type is `documentsTransferable`) → `TransferDocumentView` (recipient base58) → `platform_wallet_document_transfer` (`IdentityWallet::transfer_document_with_signer`, revision bumped, keychain signer, 8 MB worker stack). Driven end-to-end on testnet: transferred the `card` doc `FgVSYG6sTZZ9…` from `BjJz3hdmg5Ec…` → `8267geu4…` (QA2), revision →4, owner changed + persisted. *(Settings builder → `dash_sdk_document_transfer_to_identity` remains as a test-signer alternative.)* |
| DOC-06 | Update document price | Platform | Uncommon | ✅ |  | Production UI: **Browse Documents** → document → **⋯** → **Set Price…** (shown when the doc type has a `tradeMode`) → `SetDocumentPriceView` (price in credits) → `platform_wallet_document_set_price` (`IdentityWallet::set_document_price_with_signer`, revision bumped, keychain signer, 8 MB worker stack). Driven end-to-end on testnet: priced the `card` doc `FgVSYG6sTZZ9…` at 1,000,000 credits, revision →3, `$price` present in persisted JSON. *(Settings builder / `DocumentWithPriceView` → `dash_sdk_document_update_price_of_document` remains as a test-signer alternative.)* |
| DOC-07 | Purchase document | Platform | Uncommon | ✅ |  | Production UI: **Browse Documents** → document → **⋯** → **Purchase…** → `PurchaseDocumentView` → `platform_wallet_document_purchase` (`IdentityWallet::purchase_document_with_signer`; the **purchaser** signs, revision bumped, 8 MB worker stack). Gating verified: Purchase is surfaced **only** when the owner is *not* a wallet-controlled identity **and** the doc type has a `tradeMode` — i.e. buyer ≠ owner, which consensus requires (it rejects self-purchase). Shares the identical broadcast/persist path proven this session by DOC-03/04/05/06 on `FgVSYG6sTZZ9…`; the menu correctly **omitted** Purchase for every wallet-owned doc. A fresh end-to-end buy needs a for-sale doc owned by a counterparty the wallet doesn't control (can't self-buy); the on-chain purchase path was previously confirmed on testnet. *(Settings builder → `dash_sdk_document_purchase` remains as a test-signer alternative.)* |
| DOC-08 | Document aggregation (umbrella) | Platform | Uncommon | ➖ | aggregation | Split into the rows below — `DOC-10` (count total), `DOC-11` (count filtered), `DOC-12` (count grouped), `DOC-13` (sum), `DOC-14` (average). Kept as a pointer only; select the specific row. |
| DOC-09 | Create document (local demo) | Platform | — | ➖ |  | Retired. The old `DocumentsView` local-only mock was replaced by the real broadcast flow (`CreateDocumentView`); see `DOC-02`. |
| DOC-10 | Aggregation — count documents (total) | Platform | Uncommon | 🧪 | aggregation, read-only | **Count Documents** read view → Swift wrapper over FFI `dash_sdk_document_count` (proof-verified). Total count is `counts[""]` in the `{counts:{hexKey:u64}}` result. Requires a contract whose doc type sets `documentsCountable: true` (e.g. the `countable` QA fixture). |
| DOC-11 | Aggregation — count documents, filtered (`where`) | Platform | Uncommon | 🧪 | aggregation, read-only | Same Count view with a `where` clause → `dash_sdk_document_count(where_json=…)`. The filtered field must be a `countable` index. |
| DOC-12 | Aggregation — count documents, grouped (`group_by`) | Platform | Uncommon | 🧪 | aggregation, read-only | Same Count view with a `group_by` field → `dash_sdk_document_count(group_by_json=…)`; returns one count per group (hex-encoded group key → `u64`). |
| DOC-13 | Aggregation — sum of a numeric property | Platform | Uncommon | 🧪 | aggregation, read-only | **Sum / Average Documents** read view (op selector → **Sum**) → Swift wrapper over FFI `dash_sdk_document_sum` (proof-verified). Total sum is `sums[""]` in the `{sums:{hexKey:i64}}` result; a `where`/`group_by` filter and the required numeric `sum property` are entered in the same view. Needs a contract doc type with a `summable` index on the numeric property. |
| DOC-14 | Aggregation — average of a numeric property | Platform | Uncommon | 🧪 | aggregation, read-only | Same **Sum / Average Documents** read view (op selector → **Average**) → Swift wrapper over FFI `dash_sdk_document_average` (proof-verified) → `{averages:{hexKey:{count,sum}}}`; the view divides `sum/count` for display. Needs a doc type with a `summable` index on the numeric property. |
| DOC-15 | Document transfer / purchase across wallets | Platform | Uncommon | ✅ | multiwallet | A creates + lists a document (`DOC-02`/`DOC-06`); B transfers/purchases it (`DOC-05`/`DOC-07`). Ownership and credits move between A and B. Transfer half driven end-to-end this session via the production UI: the `card` doc `FgVSYG6sTZZ9…` moved from identity `BjJz3hdmg5Ec…` to a different seed-controlled identity `8267geu4…` (QA2), with ownership re-persisted. Purchase half shares `DOC-07`'s status (a fresh buy needs a counterparty-owned for-sale listing). |

### 4.8 Tokens — `Domain=Token`

All token actions support single-signer **and** group (propose / co-sign) modes via `platform-wallet`. Reachability is gated by the contract's on-chain permission rules (a "coming soon" placeholder = disallowed by rule, not unimplemented).

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| TOK-01 | View token balances / details / search | Platform | Common | ✅ | read-only | `TokenDetailsView`, `TokensView`, `TokenSearchView`. |
| TOK-02 | Transfer token | Platform | Common | ✅ |  | `TokenTransferActionView` → `wallet.tokenTransfer`. |
| TOK-03 | Direct purchase token | Platform | Common | ✅ |  | `TokenPurchaseActionView` → `wallet.tokenPurchase`. Fetches the configured price via `getTokenDirectPurchasePrices` (canonical id from `calculateTokenId`), models it as `TokenDirectPurchasePricing`, and computes `expectedTotalCost` client-side with Drive's tier rule so Buy enables only for a purchasable amount. |
| TOK-04 | Token queries (statuses / prices / contract info / supply / distributions) | Platform | Common | ✅ | read-only | `PlatformQueriesView` token category → `dash_sdk_token_get_*`. |
| TOK-05 | Mint (issuance) | Platform | Thorough | ✅ |  | `TokenMintActionView` → `wallet.tokenMint`. |
| TOK-06 | Burn | Platform | Thorough | ✅ |  | `TokenBurnActionView` → `wallet.tokenBurn`. |
| TOK-07 | Claim distribution (perpetual / pre-programmed) | Platform | Thorough | ✅ | distribution | `TokenClaimActionView` → `wallet.tokenClaim`. |
| TOK-08 | Freeze an identity's balance | Platform | Uncommon | ✅ | freeze | `TokenFreezeActionView` → `wallet.tokenFreeze`. |
| TOK-09 | Unfreeze a balance | Platform | Uncommon | ✅ | freeze | `TokenUnfreezeActionView` → `wallet.tokenUnfreeze`. |
| TOK-10 | Destroy frozen funds | Platform | Uncommon | ✅ |  | `TokenDestroyFrozenFundsActionView`. |
| TOK-11 | Set / clear direct-purchase price | Platform | Uncommon | ✅ |  | `TokenSetPriceActionView` → `wallet.tokenSetPrice`. |
| TOK-12 | Emergency action — Pause | Platform | Uncommon | ✅ |  | `TokenPauseActionView` → `platform_wallet_token_pause`. |
| TOK-13 | Emergency action — Resume | Platform | Uncommon | ✅ |  | `TokenResumeActionView` → `platform_wallet_token_resume`. |
| TOK-14 | Config update / max supply | Platform | Uncommon | ✅ |  | `TokenUpdateMaxSupplyActionView` → `wallet.tokenUpdateConfig` (one `TokenConfigurationChangeItem` per tx). |
| TOK-15 | Group action — propose | Platform | Uncommon | ✅ | group | Token action in `.propose` mode (`CoSignProposalView`). Applies to Mint/Burn/Freeze/Unfreeze/DestroyFrozen/Emergency/Config/SetPrice. (Folds in GRP-03: token group propose/co-sign.) |
| TOK-16 | Group action — co-sign existing | Platform | Uncommon | ✅ | group | `PendingGroupActionsView` / `CoSignProposalView`. Action executes when accumulated signer power ≥ required. |
| TOK-17 | Token transfer between two on-device identities | Platform | Thorough | ✅ | multiwallet, regression | `TOK-02`, recipient = wallet B's identity. Switch to B; verify the token balance arrived. |
| TOK-18 | View group info / members | Platform | Thorough | ✅ | group, read-only | `GroupDetailView` (drill into member identities). |
| TOK-19 | Group queries (info / infos / actions / signers) | Platform | Thorough | ✅ | group, read-only | `PlatformQueriesView` group category → `dash_sdk_group_get_*`. |
| TOK-20 | Standalone group lifecycle management | Platform | — | ➖ | group | Retired from the catalog — not implemented anywhere and never will be a standalone test: there is no group-create/membership transition; groups exist only as a token access-control construct (read queries `TOK-18`/`TOK-19`; actions fold into `TOK-15`/`TOK-16`). Kept here to document the absence; not seeded to the QA catalog. |

### 4.9 Shielded Pool (Orchard) — `Domain=Shielded`

Shielded notes/balance/activity have **no read-side FFI** by design — Rust pushes them to SwiftData via `on_persist_shielded_*` callbacks; the app reads SwiftData. Verify shielded reads against SwiftData, not a query.

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| SH-01 | Shielded sync (start / stop / now) | Shielded | Essential | ✅ |  | `PlatformWalletManagerShieldedSync` → `platform_wallet_manager_shielded_sync_*`. Precondition for any shielded balance/spend. |
| SH-02 | View shielded activity / notes / balance | Shielded | Essential | ✅ | read-only | `ShieldedActivityView` (SwiftData @Query). |
| SH-03 | Shield from Platform balance (Type 15) | Shielded | Essential | ✅ |  | Send flow (Platform→Shielded) → `walletManager.shieldedShield`. *Anchor: shielded tx = Essential.* |
| SH-04 | Shield from Core L1 balance | Shielded | Essential | ✅ |  | Send flow (Core→Shielded). |
| SH-05 | Shielded → shielded transfer (Type 16) | Shielded | Essential | ✅ |  | Send flow (Shielded→Shielded) → `walletManager.shieldedTransfer`. Optional ≤32-byte memo. |
| SH-06 | Unshield → Platform address (Type 17) | Shielded | Essential | ✅ |  | Send flow (Shielded→Platform) → `walletManager.shieldedUnshield`. |
| SH-07 | Shield from asset lock (Type 18) | Cross | Common | ✅ |  | `ShieldedFundFromAssetLockView` (from `WalletDetailView`) → `platform_wallet_manager_shielded_fund_from_asset_lock`. |
| SH-08 | Shielded withdraw → Core L1 (Type 19) | Cross | Common | ✅ | withdrawal | Send flow (Shielded→Core) → `walletManager.shieldedWithdraw` (custom `core_fee_per_byte`). |
| SH-09 | Prover warm-up / readiness | Shielded | Common | ✅ |  | `warmUpShieldedProver` / `shieldedProverIsReady` (~30s Halo2 key build; precondition for spends). |
| SH-10 | Seed shielded pool (anonymity set) | Shielded | Uncommon | ✅ |  | `SeedShieldedPoolView` → `platform_wallet_manager_shielded_seed_pool_notes`. **Devnet/testnet only** — hard-errors on mainnet. |
| SH-11 | Create identity from shielded pool (Type 20) | Cross | Common | ✅ |  | `CreateIdentityView` → funding source **Shielded balance** (fixed denominations 0.1 / 0.3 / 0.5 / 1.0 DASH, gated on the bound pool's balance) → `IdentityRegistrationController` (`.shieldedPool`) → `shieldedIdentityCreateFromPool` → `platform_wallet_manager_shielded_identity_create_from_pool`. Requires a synced shielded pool with sufficient balance. |
| SH-12 | Clear shielded state (wipe notes + re-sync) | Shielded | Uncommon | ✅ |  | "Clear" button on the Sync tab (`CoreContentView` → `ShieldedService.clearLocalState` → `clearShielded`). Stops sync, wipes every wallet's shielded notes + sync state, zeroes the Swift mirror; bind credentials are kept so "Sync Now" rebinds and re-scans. "Sync Now" after Clear now re-binds EVERY loaded wallet (the mirror via `bind`, others via `engineBindOtherWallets`), so cross-wallet rows (`SH-14/15/16`) work right after an SH-12 run without an app restart. (On-disk SQLite tree is intentionally retained.) Verify balance/activity reset, then restore after Sync Now. |
| SH-13 | Display / share your shielded receive address | Shielded | Common | ✅ | read-only | "Receive Dash" sheet → **Shielded** tab (`ReceiveAddressView`, `ReceiveAddressTab.shielded`): QR + full `tdash1…`/`dash1…` bech32m address + Copy Address. Hand your shielded address to a payer, or grab wallet B's address for `SH-14`. |
| SH-14 | Shielded transfer between two on-device wallets | Shielded | Thorough | ✅ | multiwallet | Wallet A's pool → wallet B's shielded address (`SH-05`); copy B's address from its Receive → Shielded tab (`SH-13`, now resolved per-wallet). Both wallets are bound automatically at rebind (no wallet-swap needed); B's shielded balance rises on the next sync pass. The global Sync tab still mirrors one wallet, but per-wallet Receive/Balance surfaces read B directly. |
| SH-15 | Unshield from A to a Platform address owned by B | Shielded | Uncommon | ✅ | multiwallet | A unshields (`SH-06`) to a Platform address belonging to wallet B; verify B receives the credits (subject to the `SYS-07` sync caveat). Both wallets are engine-bound automatically at rebind, so A can spend from its own pool without a wallet-swap. |
| SH-16 | Shielded withdraw from A to B's Core L1 address | Shielded | Uncommon | ✅ | multiwallet, withdrawal | Wallet A's pool → a Core L1 address owned by wallet B (`SH-08`). Completes the cross-wallet shielded exit set (→ shielded `SH-14`, → Platform `SH-15`, → Core `SH-16`). Both wallets are engine-bound automatically at rebind (no wallet-swap needed). Verify B's Core balance rises after SPV sync. |
| SH-17 | Multiple wallets bound to the shielded pool concurrently | Shielded | Uncommon | ✅ | multiwallet | `platform_wallet_manager_bind_shielded` is per `wallet_id`; the manager syncs all bound wallets. UI (`ShieldedService.boundWalletId`) displays one wallet's shielded state at a time — switching should swap cleanly, not merge balances. |

### 4.10 DashPay — `Domain=DashPay`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| DP-01 | Send contact request | Platform | Common | ✅ |  | DashPay tab → **Add Contact** (toolbar button `dashpay.addContact`) → `AddContactView` (mode toggle `dashpay.addContact.mode`): by Identity ID (base58, 32-byte gated) or DPNS username (≥2-char live prefix search; not-found offers clear-and-retry `dashpay.addContact.retry`) → `platform_wallet_send_contact_request_with_signer`. Optimistic-send overlay until the request appears. |
| DP-02 | Accept contact request | Platform | Common | ✅ |  | `ContactRequestsView` (Incoming) → **Accept** (`dashpay.request.accept`) → `platform_wallet_accept_contact_request_with_signer`. Contact moves to `ContactsView`; the bidirectional contact persists (both request-direction rows present for the pair). |
| DP-03 | Send DashPay payment to a contact | Platform | Common | ✅ |  | `ContactDetailView` → **Send Dash** (`dashpay.detail.sendDash`) → `SendDashPayPaymentSheet` → `platform_wallet_send_dashpay_payment`. A DashPay payment is an **L1 transaction**, so it requires the **Core SPV client running** (`CORE-07`) — with SPV stopped the broadcast fails with "SPV error: SPV Client not started". Payment appears in the contact's Payments (txid). **Verify both directions** — once a contact is established the channel is symmetric, so each party can pay the other (A→B *and* B→A); the recipient derives the sender's payment address from the xpubs exchanged at establishment. Each sender needs its own funded Core wallet + running SPV, and both endpoints must be on the **same network**. Send is disabled while the payment channel is broken — flagged on a permanent channel failure (e.g. the contact rotated their payment keys/addresses, or a request decrypt/validation failure) — showing "ask the contact to send a new request"; it re-enables when a fresh request arrives. |
| DP-04 | Create / update DashPay profile | Platform | Common | ✅ |  | `DashPayProfileView` → **Edit** (`dashpay.profile.edit`) → `DashPayProfileEditorView` (`dashpay.profile.displayName` / `.publicMessage` / `.avatarUrl`) → `platform_wallet_create_or_update_dashpay_profile_with_signer`. Non-destructive update; avatar renders via `DashPayAvatarView` and the re-fetched profile carries the computed `avatarHash` + 8-byte dHash `avatarFingerprint`. |
| DP-05 | View profile / contacts / requests | Platform | Common | ✅ | read-only | DashPay tab: `ContactsView` (established + search `dashpay.search`), `ContactRequestsView` (incoming/outgoing), `ContactDetailView`, `DashPayProfileView` — backed by `PersistentDashpayContactRequest` (SwiftData); established contacts are derived in-memory by joining each pair's incoming + outgoing request rows. |
| DP-06 | Ignore a contact request (reversible local mute) | Platform | Thorough | ✅ |  | `ContactRequestsView` → **Ignore** (`dashpay.request.ignore`) → `wallet.ignoreContactSender`. The sender leaves the requests list and appears in `IgnoredContactsView` (un-ignore `dashpay.ignored.unignore` reverses it). Local-only, no on-chain artifact (R1 privacy); persists across relaunch. |
| DP-07 | Attach `encryptedAccountLabel`; see contact's "Their account" on receive | Platform | Common | ✅ |  | DIP-15 §8.5. Send: `AddContactView` → **Account label** (`dashpay.addContact.accountLabel`) carried into `sendContactRequest(…accountLabel:)`. Receive: the counterparty's `ContactDetailView` shows a read-only **"Their account"** block (assert on visible text — no a11y id yet). Verify on a two-wallet loop (cf. `DP-11`): the ingested request carries the encrypted bytes, but the plaintext is decrypted **on accept** (the signer-bearing register step) and shown on the **incoming row only** (direction-specific). |
| DP-08 | QR auto-accept (build "Add me" QR + add via pasted URI) | Platform | Thorough | ✅ |  | DIP-15 §8.13. Build: `DashPayProfileView` → **Add me (DIP-15 QR)** (`dashpay.profile.qrURI`, `du=…&dapk=…`, 1h validity — `AUTO_ACCEPT_TTL_SECS=3600`) via `buildAutoAcceptQR`. Add: DashPay tab → **Add via QR** (`dashpay.addViaQR`) → `AddViaQRSheet` (`dashpay.qr.uriField` / `dashpay.qr.send`) → `sendContactRequestFromQR`. Two-wallet: A builds the QR, B pastes the URI → the request is auto-accepted by A without A manually accepting (a distinct acceptance path from `DP-02`). **A's reciprocal is signer-backed**, so it only fires once A's wallet is **unlocked** (the "N contacts waiting to finish setup → Unlock" drain); the request + auto-accept proof reach A immediately, but the established reciprocal lands after unlock. The paste path is simulator-drivable; a camera scan yields the same string (Manual variant). |
| DP-09 | Publish encrypted on-chain `contactInfo` (private contact metadata) | Platform | Thorough | ✅ |  | DIP-15 §10. `ContactDetailView` → edit **Alias** / **Note** / **Hide contact** (`dashpay.detail.aliasEdit` / `dashpay.detail.noteEdit` / `dashpay.detail.hideToggle`) → `saveContactInfo` → `platform_wallet_set_dashpay_contact_info_with_signer` (ECB `encToUserId` + CBC `privateData`). These fields are locally cached **and** published encrypted to Platform once the identity has **≥2 established contacts** (stated in the in-app footer) → outcomes `.published` / `.deferredUntilTwoContacts` / `.skippedWatchOnly`. |
| DP-10 | Incoming-payment backfill rescan (restore-from-seed / pre-watch window) | Cross | Manual | ✅ | regression | DIP-15 §8.7 / §12.6 (on the DIP-16 SPV base). No UI trigger — automatic in DashPay sync: `reconcile_dashpay_rescan` lowers SPV `synced_height` to `min($coreHeightCreatedAt)` across new receival contacts so the filter manager backfills. Pass: a DashPay payment that landed on a contact's address **before** it was watched (restore-from-seed / second device / the offline-accept→pay window) appears after restore + SPV sync. Environment-limited (must construct the skew window); the regression pin for the §12.6 payment-loss gap. |
| DP-11 | DashPay request → accept → payment, both endpoints on device | Platform | Thorough | ✅ | multiwallet | A's identity sends a contact request (`DP-01`) to B's; switch to wallet B's identity and accept (`DP-02`); then pay (`DP-03`). Full bidirectional loop entirely local. |

### 4.11 System / Protocol / Diagnostics — `Domain=System`

| ID | Action | Layer | Tier | Status | Tags | Entry point & test notes |
|---|---|---|---|---|---|---|
| SYS-01 | Status / total credits / quorums / prefunded balance | Platform | Thorough | ✅ | read-only | `PlatformQueriesView` system category. |
| SYS-02 | Epochs info / current / finalized / proposed blocks | Platform | Thorough | ✅ | read-only | `PlatformQueriesView` epoch category. |
| SYS-03 | Protocol-version upgrade state / vote status | Platform | Uncommon | ✅ | read-only | `PlatformQueriesView` protocol category. |
| SYS-04 | Run-all-queries / DPNS test harness | Platform | Thorough | ✅ | read-only | `PlatformQueriesView` diagnostics (`runAllQueries`, `testDPNSQueries`), `DiagnosticsView`. |
| SYS-05 | Storage / Keychain / Wallet-memory explorers | — | Thorough | ✅ | read-only | `StorageExplorerView`, `KeychainExplorerView`, `WalletMemoryExplorerView` (Settings; debug tooling). |
| SYS-06 | Path elements (raw GroveDB) | Platform | Uncommon | 🧪 | read-only | **Get GroveDB Path Elements** read view (Platform Queries → System & Utility) → Swift wrapper over FFI `dash_sdk_system_get_path_elements` (proof-verified `Element::fetch_many` over `KeysInPath`). Enter a `path` + `keys` JSON array (hex bytes); returns `[{key, element, type}]`. Use a **bounded** path — root-level queries (`path=[]`) fail GroveDB proof verification ("Cannot verify lower bound"). The "DPNS contract example" preset fills `path=["40"]` (DataContractDocuments root) + the DPNS contract id → its subtree `tree` element. |
| SYS-07 | Platform balance sync is per-active-wallet, **not** concurrent | Platform | Thorough | ✅ | multiwallet | `PlatformBalanceSyncService` is configured for ONE wallet (`configure(...walletId:)`, re-run on switch). Unlike Core SPV (`CORE-20`, all wallets at once), wallet B's Platform address/credit balances can be **stale until you switch to B and Sync Now**. Verify this is the intended behavior, not a bug. |
| SYS-08 | Per-wallet Platform isolation (identities / usernames / tokens / contacts) | Platform | Thorough | ✅ | multiwallet | Extends `CORE-18` to Platform reads: wallet A's identities, DPNS names, token balances, and DashPay contacts must never surface under wallet B. |

---

## 5. Summary matrix

Counts are of rows reachable in the app (Status `✅`/`🧪`/`⚠️`); `🔌`/`🚫`/stub rows are excluded. `Tier=Manual` rows are reachable but **not automatable** (need a physical device) — counted on their own row below, excluded from the by-layer automatable totals. Each catalog row carries its own `Tier` + `Layer`, so any intersection (e.g. *Essential ∩ Platform*) is derivable directly from §4.

**By tier:**

| Tier | Count (approx.) | Automatable? |
|---|---|---|
| Essential | 22 | yes |
| Common | 35 | yes |
| Thorough | 38 | yes |
| Uncommon | 30 | yes |
| Manual | 2 (`CORE-08`, `DP-10`) | no — physical device |

**By layer (automatable only):**

| Layer | Count (approx.) |
|---|---|
| Core | 18 |
| Platform | ~81 |
| Cross | 11 |
| Shielded | 14 |

**Headline intersection — `Essential ∩ Platform` (the most common QA request):** `ID-02`, `ID-03`, `ID-04`, `ADDR-07`, `DPNS-01`, `DPNS-02`, `DPNS-03`, `DPNS-04`. Essential Core lives in §4.1 (`CORE-01..08`); Essential cross-layer identity creation is `ID-01`; Essential shielded is `SH-01..06`.

---

## 6. Category & tag index

Each row's **primary home** is its §4 section, but a few rows are cross-cutting and are listed under an additional **category** below (e.g. `ID-06`/`ID-08`/`ID-11` under Address, `SH-11` under Identity). To run a `Category=X` selection, take the category list below — it already includes those cross-cutting members. For cross-cutting *modalities* (multi-wallet, group, read-only, …) use the **tag index** that follows. Intersect either with `Tier` / `Layer` / `Status` as needed. `A-01..09` means every id in that span.

**By category (§4 section):**

- **Core / Wallet** — `CORE-01..23`
- **Identity** — `ID-01..15`, `SH-11`
- **Address** (DIP-17 platform addresses) — `ADDR-01..04`, `ADDR-06..09`, `ID-06`, `ID-08`, `ID-11`
- **DPNS** — `DPNS-01..08`
- **Voting** — `VOTE-01..07`, `DPNS-05`
- **Contract** — `DC-01..04`
- **Document** — `DOC-01..15`
- **Token** — `TOK-01..20`
- **Shielded** — `SH-01..17`
- **DashPay** — `DP-01..11`
- **System / Diagnostics** — `SYS-01..08`

**By tag (cross-cutting, the Tags column):**

- **multiwallet** — `CORE-14..23`, `ID-14`, `ID-15`, `TOK-17`, `DPNS-08`, `DP-11`, `DOC-15`, `SH-14`, `SH-15`, `SH-16`, `SYS-07`, `SYS-08`
- **group** — `TOK-15`, `TOK-16`, `TOK-18`, `TOK-19`, `TOK-20`
- **contested** — `DPNS-05`, `DPNS-08`, `VOTE-01..07`
- **withdrawal** — `ID-10`, `ADDR-04`, `SH-08`, `SH-16`
- **aggregation** — `DOC-08`, `DOC-10..14`
- **read-only** — the pure query / view rows (see the Tags column; also Appendix A)
- **freeze** — `TOK-08`, `TOK-09`
- **distribution** — `TOK-07`
- **masternode** — `VOTE-01`
- **regression** — `TOK-17`, `ADDR-09`, `DP-10`

Worked example — *"run all non-Uncommon Token tests"*: take **Token** = `TOK-01..20`; drop the `Uncommon`/stub ones (`TOK-08..16`, `TOK-20`) → run **`TOK-01..07`** plus the Thorough group / multi-wallet token rows `TOK-17`/`TOK-18`/`TOK-19`. Worked example — *"run all multi-wallet token tests"*: `Category=Token AND Tag=multiwallet` → `TOK-17`.

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
| getDocuments (incl. V1 COUNT/SUM/AVG, group_by, having) | Common | ✅ / 🧪 | `DocumentsView` / catalog. COUNT (total/`where`/`group_by`) has a **Count Documents** read view — `DOC-10/11/12`. SUM/AVG now have a **Sum / Average Documents** read view — `DOC-13/14`. `having` is not exposed by the FFI. |
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
| getPathElements | Uncommon | 🧪 | "Get GroveDB Path Elements" read view (`SYS-06`) |
| getConsensusParams | Uncommon | 🚫 | `@sdk-ignore` (served via Tenderdash RPC) |

### Address Sync (DIP-17)
| RPC | Tier | Status | Where |
|---|---|---|---|
| getAddressInfo | Common | ✅ | `ADDR-01` |
| getAddressesInfos | Common | ✅ | `ADDR-01` |
| getRecentAddressBalanceChanges | Uncommon | 🔌 | FFI only (no UI) |
| getRecentCompactedAddressBalanceChanges | Uncommon | 🔌 | FFI only (no UI) |
| getAddressesTrunkState | Uncommon | 🔌 | FFI only (no UI) |
| getAddressesBranchState | Uncommon | 🔌 | FFI only (no UI) |

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
- address balance-change history (recent / compacted / branch / trunk) — FFI only, no UI
- `SH-11` create identity from shielded pool (Type 20)

**🚫 Not implemented anywhere:**
- `TOK-20` standalone group lifecycle management
- `getConsensusParams` (served via Tenderdash RPC, not the SDK)

**Protocol-level write transitions present in DPP but not surfaced as distinct app actions** (the address/asset-lock family — `IdentityCreditTransferToAddresses`, `IdentityCreateFromAddresses`, `IdentityTopUpFromAddresses`, `AddressFundsTransfer`, `AddressFundingFromAssetLock`, `AddressCreditWithdrawal`) are largely covered by the `ID-*`/`ADDR-*` rows above; the shielded family (`Shield`, `ShieldedTransfer`, `Unshield`, `ShieldFromAssetLock`, `ShieldedWithdrawal`, `IdentityCreateFromShieldedPool`) maps to the `SH-*` rows. Anything not mapped is either internal or `🔌`/`🚫` above.
