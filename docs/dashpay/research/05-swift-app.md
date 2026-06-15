# 05 — Swift App (SwiftExampleApp + swift-sdk) Architecture & DashPay Surface

Research date: 2026-06-10. Worktree: `/Users/ivanshumkov/Projects/dashpay/platform.worktrees/dev`.
Base dir: `packages/swift-sdk/`.

Goal: map the iOS app architecture, the Swift→FFI call pattern, and the
**existing** DashPay/identity/profile/contact surface, so we can design a
polished full-DashPay UI (sync → contact request → approve → send money →
profile) and a test plan. **Key finding up front: a working DashPay surface
already exists end-to-end** (contacts list, incoming/outgoing requests,
accept/reject, send-money-to-contact, profile create/edit, DPNS lookup). It
is functional-but-utilitarian and buried under a per-identity drill-in, not a
first-class tab. The work is largely **UI polish + promotion + test
coverage**, not greenfield wiring.

---

## 1. App architecture

### 1.1 Entry point & app-state coordinators

`SwiftExampleApp/SwiftExampleApp/SwiftExampleAppApp.swift` (`@main`) builds one
`ModelContainer` (`DashModelContainer.create()`) and injects a set of
`@StateObject` coordinators into the environment. There is **no single
`UnifiedAppState`** anymore — the old `UnifiedAppState` / `WalletService` /
`CoreWalletManager` / `SPVClient` stack was removed (see
`SwiftExampleApp/CLAUDE.md` lines 67-91). Current coordinators:

| Object | Type | Role | File |
|---|---|---|---|
| `AppState` | `@StateObject platformState` | Platform SDK wrapper: identity/document/contract state, holds the `sdk` handle, `currentNetwork`. | `SwiftExampleApp/SwiftExampleApp/AppState.swift` |
| `WalletManagerStore` | `@StateObject` | Per-network store that lazy-creates + caches one `PlatformWalletManager` per network and republishes the active one. | `SwiftExampleApp/SwiftExampleApp/WalletManagerStore.swift` |
| `PlatformWalletManager` | env object (`walletManagerStore.activeManager`) | The workhorse. Holds N wallets keyed by walletId, drives SPV/BLAST/shielded sync, identity registration, **and all DashPay ops**. Publishes `spvProgress`, `wallets`, `lastError`. | `Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManager.swift` |
| `ShieldedService` | `@StateObject` | Orchard shielded-pool ops. | `Core/Services/ShieldedService.swift` |
| `PlatformBalanceSyncService` | `@StateObject` | Periodic BLAST platform-address balance sync. | `Core/Services/PlatformBalanceSyncService.swift` |
| `TransitionState` | `@StateObject` | Ephemeral state-transition flow state (pricing/eligibility). | `SwiftExampleApp/SwiftExampleApp/TransitionState.swift` |
| `AppUIState` | `@StateObject` | Tiny UI-only flag bag (e.g. `showWalletsSyncDetails`). | defined inline in `SwiftExampleAppApp.swift` |

There is **no dedicated `IdentityService`** — identity ops live on
`PlatformWalletManager` / `ManagedPlatformWallet` / `ManagedIdentity` and the
`Services/*Coordinator.swift` files (registration, asset-lock funding).

### 1.2 Navigation / tab structure

`ContentView.swift` is the root. A 5-tab `TabView` (enum `RootTab`):

```text
sync · wallets · identities · contracts · settings
```

Each tab wraps its content in its own `NavigationStack`
(`SyncStatusView`, `WalletsTabView`, `IdentitiesTabView`,
`ContractsTabView`, `SettingsView`). **There is no Contacts / DashPay
top-level tab.** DashPay lives under: `Identities` tab → `IdentityRow`
→ `IdentityDetailView` → `Section("DashPay")` → `FriendsView`, plus a
`Section("DashPay Profile")` on the same detail screen
(`ContentView.swift:99-111`, `IdentityDetailView.swift:184-204, 332-346`).

A global sync progress bar is rendered as a `.overlay(alignment: .top)` on the
TabView (`ContentView.swift:120-125`, `GlobalSyncIndicator`).

### 1.3 SwiftData models

Models live in `Sources/SwiftDashSDK/Persistence/Models/` and are registered in
`DashModelContainer.swift`. DashPay-relevant ones:

- `PersistentIdentity` — owns optional `dashpayProfile` (cascade) and DashPay
  contact-request rows.
- `PersistentDashpayProfile` — `displayName`, `publicMessage`, `bio`,
  `avatarUrl`, `avatarHash`, `avatarFingerprint`, `network`, back-ref
  `identity`. (`Models/PersistentDashpayProfile.swift`)
- `PersistentDashpayContactRequest` — `ownerIdentityId`, `contactIdentityId`,
  `isOutgoing`, `senderKeyIndex`, `recipientKeyIndex`, `accountReference`,
  `encryptedPublicKey`, `encryptedAccountLabel`, `autoAcceptProof`,
  `coreHeightCreatedAt`, `createdAtMillis`, back-ref `owner`.
  (`Models/PersistentDashpayContactRequest.swift`)
- `PersistentDPNSName`, plus wallet/account/tx/token/shielded models.

Both DashPay models are **cascade-owned by `PersistentIdentity`** and written by
the Rust persister callback via
`PlatformWalletPersistenceHandler.upsertDashpayProfile` (see
`DashModelContainer.swift:141-157`). **NB:** `FriendsView` today does NOT
`@Query` these rows — it reads live state off the Rust `ManagedIdentity`
snapshot each load. So the persisted rows exist but the UI doesn't yet use them
reactively (a clean opportunity for the new UI — see §4).

### 1.4 FFI call pattern (view → service → FFI)

Architecture rule (`packages/swift-sdk/CLAUDE.md`): the Swift SDK only
**persists, loads, and bridges**. All orchestration (gap-limit scans, derivation
pipelines, DashPay payment address derivation) lives in the Rust
`platform-wallet` crate, reached via `rs-platform-wallet-ffi`. Swift wrappers
are thin: resolve handle → marshal in → call → marshal out.

The canonical async-FFI shape (from `ManagedPlatformWallet.sendContactRequest`,
`ManagedPlatformWallet.swift:1484`):

```swift
public func sendContactRequest(
    senderIdentityId: Identifier,
    recipientIdentityId: Identifier,
    accountLabel: String? = nil,
    autoAcceptProof: Data? = nil,
    signer: KeychainSigner
) async throws -> ContactRequest {
    let handle = self.handle               // UInt64 wallet handle
    let signerHandle = signer.handle
    let senderBytes  = senderIdentityId.withFFIBytes { Array(UnsafeBufferPointer(start: $0, count: 32)) }
    let recipientBytes = recipientIdentityId.withFFIBytes { Array(UnsafeBufferPointer(start: $0, count: 32)) }

    let requestHandle: Handle = try await Task.detached(priority: .userInitiated) {
        _ = signer                         // keepalive — ctx is passUnretained
        var outHandle: Handle = NULL_HANDLE
        let result = senderBytes.withUnsafeBufferPointer { s in
            recipientBytes.withUnsafeBufferPointer { r in
                platform_wallet_send_contact_request_with_signer(
                    handle, s.baseAddress!, r.baseAddress!, /*label*/nil,
                    /*proof*/nil, 0, signerHandle, &outHandle)
            }
        }
        try result.check()                 // PlatformWalletFFIResult → throws PlatformWalletError
        return outHandle
    }.value
    return ContactRequest(handle: requestHandle)
}
```

Pattern characteristics, repeated across the SDK:
- **Handles** are `UInt64` (`typealias Handle`); opaque Rust objects
  (`ManagedPlatformWallet`, `ContactRequest`, `EstablishedContact`,
  `ManagedIdentity`) are `final class … @unchecked Sendable` wrapping one
  handle, freeing it in `deinit`.
- **Blocking FFI runs in `Task.detached(priority: .userInitiated)`**; the
  wrapper method is `async throws`. Result codes come back as
  `PlatformWalletFFIResult` with a `.check()` that throws
  `PlatformWalletError`.
- **Signing** always goes through a `KeychainSigner` (FFI `_with_signer`
  variants) — never the wallet seed. `KeychainSigner`
  (`Sources/SwiftDashSDK/FFI/KeychainSigner.swift`) is the C-ABI signer
  trampoline: Rust calls back with raw pubkey bytes, Swift looks up the
  `PersistentPublicKey` row, pulls the 32-byte scalar from Keychain, signs,
  zeroes the buffer. Must be kept alive across the detached task (`_ = signer`).
- **Out-params** are inline C tuples (32-byte ids, etc.) copied into owned
  `Data`; arrays come back as FFI structs with a paired `_free` function called
  via `defer`.
- Optional C-strings marshalled via the local `invokeWithOptionalCStrings`
  helper (in `ManagedPlatformWallet.swift`).

View-side dispatch (from `FriendsView.acceptRequest`,
`FriendsView.swift:244`): a `@State` var holds UI data; the action wraps the
async call in `Task { @MainActor in … }`, resolves the wallet via
`walletManager.wallet(for: walletId)`, constructs a `KeychainSigner`, calls the
wrapper, sets `errorMessage` on throw, and re-runs `loadFriends()` on success.

---

## 2. Existing identity / wallet / DashPay surface

### 2.1 DashPay UI — **already exists** (the big finding)

`SwiftExampleApp/SwiftExampleApp/Views/FriendsView.swift` (917 lines) is a
complete, working DashPay contacts screen. It contains these views:

| View (in FriendsView.swift) | What it does | Status |
|---|---|---|
| `FriendsView` | List of established contacts + incoming-requests section; toolbar "add friend"; per-contact tap → send-money sheet. Loads via `wallet.syncContactRequests()` then reads `ManagedIdentity` snapshot ids. | Working, utilitarian |
| `ContactRowView` | Avatar (initial circle), display name, dpns/hex subtitle, note. | Working |
| `ContactRequestRow` | Incoming request with Accept/Reject buttons; relative timestamp. | Working |
| `AddFriendView` | Send contact request by **DPNS name** or **Identity ID** (segmented picker); resolves via `wallet.resolveDpnsName` / base58 parse; calls `wallet.sendContactRequest(…signer:)`. | Working |
| `SendDashPayPaymentSheet` | Send Dash to a contact: amount in DASH→duffs, shows sender balance, recipient profile/avatar/DPNS, over-spend validation, calls `wallet.sendDashPayPayment(…)`, shows txid. | Working, fairly polished |
| value types `DashPayContact`, `DashPayContactRequest` | Lightweight UI models. | — |

Profile UI is on `IdentityDetailView.swift`:
- `Section("DashPay Profile")` + `dashPayProfileCard(identity:)` — three states
  (populated / empty placeholder / loading), "Edit Profile" / "Set up profile"
  button (`IdentityDetailView.swift:332-346, 748-835`).
- `DashPayProfileEditorView` (`IdentityDetailView.swift:1169-1410`) — Form with
  display name / public message / avatar URL; on save fetches avatar bytes
  (for DIP-15 hash/fingerprint), calls `wallet.createDashPayProfile` /
  `updateDashPayProfile(…signer:)`.
- `loadCachedDashPayProfile` / `refreshDashPayProfilesFromPlatform` — cached read
  + `syncDashPayProfiles()` network refresh.
- `EditAliasView` (`IdentityDetailView.swift:1411`) — local alias editing.

DPNS UI: `Section("DPNS Names")` in `IdentityDetailView` (register/contested/
select-main), plus `RegisterNameView.swift`, `SelectMainNameView.swift`,
`DPNSTestView.swift`. Recipient selection helper:
`Views/Components/RecipientPickerView.swift` (pick a local identity / paste a
base58 id — used by credit-transfer flows, reusable for DashPay).

### 2.2 Identity & wallet surface

Extensive. Identities: `IdentitiesContentView` (list, swipe-to-remove),
`IdentityDetailView` (the hub), `CreateIdentityView`, `LoadIdentityView`,
`TopUpIdentityView`, `AddIdentityKeyView`, `KeysListView`/`KeyDetailView`,
`SearchWalletsForIdentitiesView`, registration progress views, and the
`Services/*RegistrationCoordinator*` / `AddressFundFromAssetLock*` controllers.
Wallets/Core: `Core/Views/` — `WalletsContentView`, `WalletDetailView`,
`AccountListView`/`AccountDetailView`, `CreateWalletView`, `SeedBackupView`,
`SendTransactionView`, `ReceiveAddressView`, `TransactionListView`. Plus the
shielded-pool funding views. So DashPay sits inside a mature, busy demo app.

---

## 3. Swift SDK wrapper status — DashPay FFI coverage

**All core DashPay FFI functions are already wrapped.** From
`Sources/SwiftDashSDK/PlatformWallet/`:

| Capability | Swift method | FFI symbol | File:line |
|---|---|---|---|
| Sync incoming contact requests | `ManagedPlatformWallet.syncContactRequests()` | `platform_wallet_sync_contact_requests` | `ManagedPlatformWallet.swift:1452` |
| Send contact request | `.sendContactRequest(…signer:)` | `platform_wallet_send_contact_request_with_signer` | `:1484` |
| Accept contact request | `.acceptContactRequest(_:signer:)` → `EstablishedContact` | `platform_wallet_accept_contact_request_with_signer` | `:1556` |
| Reject contact request | `.rejectContactRequest(ourIdentityId:contactIdentityId:)` | `platform_wallet_reject_contact_request` | `:1585` |
| Fetch sent requests | `.fetchSentContactRequests(identityId:)` | `platform_wallet_fetch_sent_contact_requests` | `:1612` |
| Send money to contact | `.sendDashPayPayment(from:to:amountDuffs:memo:)` → txid | `platform_wallet_send_dashpay_payment` | `:1651` |
| Read cached profile | `.getDashPayProfile(identityId:)` | `platform_wallet_get_dashpay_profile` | `:1714` |
| Sync all profiles | `.syncDashPayProfiles()` | `platform_wallet_sync_dashpay_profiles` | `:1744` |
| Create profile | `.createDashPayProfile(…signer:)` | `platform_wallet_create_or_update_dashpay_profile_with_signer` | `:1763` |
| Update profile | `.updateDashPayProfile(…signer:)` | same (`doCreate:false`) | `:1779` |
| Resolve DPNS name | `.resolveDpnsName(_:)` | `platform_wallet_resolve_dpns_name` | `:1261` |
| Register DPNS name | `.registerDpnsName(…signer:)` | `platform_wallet_register_dpns_name_with_signer` | `:1215` |
| Search DPNS | `.searchDpnsNames(prefix:limit:)` | `platform_wallet_search_dpns_names` | `:1285` |
| Sync DPNS names | `.syncDpnsNames(identityId:)` | `platform_wallet_sync_dpns_names` | `:1335` |

Supporting wrapper types/objects (all in `Sources/SwiftDashSDK/PlatformWallet/`):
- `ContactRequest.swift` — opaque handle; getters for sender/recipient id, key
  indices, account ref, encrypted pubkey, createdAt; `create(…)` builder.
- `EstablishedContact.swift` — `getContactIdentityId`, `getAlias`/`setAlias`/
  `clearAlias`, `getNote`/`setNote`/`clearNote`, `isHidden`/`hide`/`unhide`.
- `DashPayProfile.swift` — value struct (`displayName`, `publicMessage`,
  `avatarUrl`, `avatarHash`, `avatarFingerprint`) + `DashPayProfileUpdate`
  input struct (+ `avatarBytes` for DIP-15 hashing).
- `ManagedIdentity.swift` — local-cache accessors:
  `getSentContactRequestIds` / `getIncomingContactRequestIds` /
  `getEstablishedContactIds` (`:240-307`), single-item
  `getSentContactRequest` / `getIncomingContact` / `getEstablishedContact`
  (`:307-360`), `isContactEstablished` (`:360`), `getDashPayProfile()`
  (`:444`), `getDpnsNames` / `getContestedDpnsNames` (`:399-407`).

**Known gaps / caveats (from docstrings):**
1. **ECDH on watch-only wallets** — `sendContactRequest` still derives the
   sender's ECDH key Rust-side from the wallet seed; watch-only wallets fail at
   the encryption step (`ManagedPlatformWallet.swift:1480-1483`). A future FFI
   must push ECDH across.
2. **Reject is local-only** — `rejectContactRequest` only drops the local entry;
   no `display_hidden` contactInfo doc is written yet (`:1580-1584`).
3. **Payment memo** is recorded on the Rust `PaymentEntry` but not embedded
   on-chain; the payment sheet deliberately omits a memo field
   (`FriendsView.swift:765-772, 890-893`).
4. `DashPayProfileEditorView` falls back to `firstWallet` when `walletId` is nil
   — needs tightening for multi-wallet (`IdentityDetailView.swift:1171-1175`).
5. **Persisted DashPay rows are not yet `@Query`-driven in the UI** — FriendsView
   reads the live Rust snapshot; the `PersistentDashpay*` rows exist but aren't
   the reactive source of truth.

---

## 4. Where new DashPay screens slot in

The cleanest design move is to **promote DashPay from a per-identity drill-in to
a first-class experience** while reusing the already-wrapped FFI. Two viable
shapes:

**Option A (lower risk, matches house style): keep it per-identity, polish in
place.** Extend `IdentityDetailView`'s `Section("DashPay")` + the existing
`FriendsView` / `DashPayProfileEditorView`. No new tab; the identity is the
account context, which is architecturally honest (contacts belong to an
identity).

**Option B (nicer UX, more work): add a top-level `RootTab.dashpay` tab.** Add a
case to `RootTab` (`ContentView.swift:6-7`), a `DashPayTabView { NavigationStack
{ … } }` wrapper, and an identity picker at the top (most DashPay UIs assume one
active identity). This is where a "nice UI" lives. Insertion points:

| New screen | Slots into | Service/method to call (already wrapped) |
|---|---|---|
| **Contacts list** | New `ContactsView` (tab root) or replace `FriendsView` body; drive off `@Query [PersistentDashpayContactRequest]`/`[PersistentDashpayProfile]` for reactivity, refreshed by `syncContactRequests()` + `syncDashPayProfiles()`. | `getEstablishedContactIds`, `getDashPayProfile`, `EstablishedContact` |
| **Contact requests (incoming/outgoing)** | A `Section`/segmented sub-view; today incoming is inline in `FriendsView`, outgoing (`sentRequests`) is loaded but **not rendered** — add the outgoing section. | `getIncomingContactRequestIds`, `getSentContactRequestIds`, `fetchSentContactRequests` |
| **Send contact request by username** | Replace/restyle `AddFriendView`; add live DPNS prefix search. | `searchDpnsNames`, `resolveDpnsName`, `sendContactRequest(…signer:)` |
| **Approve / reject requests** | Already in `ContactRequestRow`; just restyle + add toast/animation. | `acceptContactRequest(…signer:)`, `rejectContactRequest` |
| **Profile view/edit** | Promote `DashPayProfileEditorView`; add avatar image preview + DPNS handle. Move it out of `IdentityDetailView` into a standalone `ProfileView`. | `getDashPayProfile`, `createDashPayProfile`/`updateDashPayProfile(…signer:)`, `syncDashPayProfiles` |
| **Send money to contact** | Restyle `SendDashPayPaymentSheet`; it's already the most polished piece. | `sendDashPayPayment`, `wallet.balance()` |
| **Initial sync** | Wire DashPay sync into the existing `Sync` tab / `GlobalSyncIndicator`, or run `syncContactRequests()`+`syncDashPayProfiles()` on the DashPay tab `.task`. | existing sync coordinators on `PlatformWalletManager` |

**Service to extend:** all of it lands on `ManagedPlatformWallet` (resolved via
`walletManager.wallet(for: identity.wallet?.walletId)`). No new FFI is required
for the happy path — only the watch-only ECDH gap (caveat 1) and the
persistent-reject doc (caveat 2) need Rust-side follow-ups, and those are
**platform-wallet crate** changes per `swift-sdk/CLAUDE.md`, not Swift.

---

## 5. House-style conventions a new screen must follow

From the existing DashPay/identity views (`FriendsView`, `IdentityDetailView`,
`RecipientPickerView`) and `SwiftExampleApp/CLAUDE.md`:

- **View naming:** `*View` for screens/rows, `*Sheet` for modal sheets,
  `*EditorView` for forms. One file may hold several related views
  (`FriendsView.swift` holds 6).
- **State:** `@EnvironmentObject var walletManager: PlatformWalletManager` +
  `@EnvironmentObject var appState: AppState` (a.k.a. `platformState`);
  `@Environment(\.modelContext)`; local `@State` for view data, `isLoading` /
  `isSending` flags, and a `String? errorMessage` surfaced as a red caption.
- **Reactivity:** prefer SwiftUI `@Query` on `Persistent*` models for lists
  (CLAUDE.md "Use `@Query` for reactive data"). The new DashPay UI should move
  off the snapshot-read pattern onto `@Query [PersistentDashpay*]`.
- **Async FFI dispatch:** wrap calls in `Task { @MainActor in … }`; `defer {
  isLoading = false }`; resolve wallet via `walletManager.wallet(for:)`;
  construct `KeychainSigner(modelContainer: modelContext.container)` per submit;
  set `errorMessage = error.localizedDescription` on `catch`.
- **Layout:** `Form` + `Section` for editors/detail; `List` + `Section` with
  `Text("… (\(count))")` headers for lists. `NavigationLink(destination:)` for
  drill-down; `.sheet(isPresented:)` / `.sheet(item:)` for modals (item-based
  uses an `Identifiable` value type). Toolbar Cancel (leading) / action
  (trailing, swaps to `ProgressView` while busy).
- **Theming:** SF Symbols everywhere (`person.2`, `person.badge.plus`,
  `paperplane`, `pencil`, `person.crop.circle`); avatar = blue
  `Circle().fill(.opacity(0.2))` with the first initial, upgraded to
  `AsyncImage` when `avatarUrl` is present (see `SendDashPayPaymentSheet`).
  Accent `Color.blue`; destructive `.tint(.red)`; success `.green` captions.
  `.buttonStyle(.borderedProminent)` for primary actions.
- **Amounts:** input in **DASH**, convert to **duffs** (`× 100_000_000`) before
  the FFI; show spendable balance and block over-spends
  (`SendDashPayPaymentSheet.amountDuffs`).
- **Identifiers vs display:** ids are `Data` (32 bytes); display via
  `toBase58String()` / truncated `toHexString().prefix(12) + "…"`; prefer
  DashPay `displayName` → DPNS label → truncated hex (the
  `recipientDisplayName` precedence in `SendDashPayPaymentSheet`).
- **Architecture guardrail:** never orchestrate derivation/iteration in Swift;
  every multi-step DashPay op must be one `platform-wallet` FFI call
  (`packages/swift-sdk/CLAUDE.md`). New screens only marshal + persist + render.
- **Accessibility:** add `.accessibilityIdentifier(...)` on tab/key controls
  (precedent: `rootTab.wallets`) — needed for the UI test plan.

### Test plan seed
- **Existing coverage is thin:** no DashPay-specific tests in
  `SwiftTests/SwiftDashSDKTests/` or `SwiftExampleApp/SwiftExampleAppTests/`
  (only `WalletDeletionTests` mentions profiles tangentially). `ContactRequest`,
  `EstablishedContact`, `DashPayProfile` round-trips and the wrapper marshalling
  are **untested**.
- Add: (a) Swift unit tests in `SwiftTests/SwiftDashSDKTests/` for
  `ContactRequest`/`EstablishedContact`/`DashPayProfile(ffi:)` round-trips and
  `DashPayProfileUpdate` marshalling; (b) flow tests for the
  send→sync→accept→established cycle against a regtest/devnet wallet
  (mirror `PlatformWalletIntegrationTests.swift`); (c) XCUITest in
  `SwiftExampleAppUITests/` driving the new tab (add request by DPNS → approve →
  send money), keyed on accessibility identifiers; (d) the
  `simulator-control` skill is available to drive SwiftData + screenshots for
  UAT verification.

---

## Appendix — key file paths

- App root / tabs: `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/ContentView.swift`, `SwiftExampleAppApp.swift`
- DashPay screens: `…/Views/FriendsView.swift`, `…/Views/IdentityDetailView.swift` (profile section + editor), `…/Views/Components/RecipientPickerView.swift`
- Identity/wallet hub: `…/Core/Views/IdentitiesContentView.swift`, `…/Views/IdentityDetailView.swift`, `…/Core/Views/WalletsContentView.swift`
- SDK DashPay wrappers: `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/{ManagedPlatformWallet,ContactRequest,EstablishedContact,DashPayProfile,ManagedIdentity}.swift`
- Signer / FFI plumbing: `…/Sources/SwiftDashSDK/FFI/KeychainSigner.swift`, `…/PlatformWallet/PlatformWalletFFI.swift`, `…/PlatformWallet/PlatformWalletResult.swift`
- SwiftData DashPay models: `…/Sources/SwiftDashSDK/Persistence/Models/{PersistentDashpayProfile,PersistentDashpayContactRequest,PersistentIdentity}.swift`, container `…/Persistence/DashModelContainer.swift`
- Persister callback: `…/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletPersistenceHandler.swift`
- Architecture rules: `packages/swift-sdk/CLAUDE.md` (SDK), `packages/swift-sdk/SwiftExampleApp/CLAUDE.md` (app)
