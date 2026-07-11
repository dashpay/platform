# DashPay State Encapsulation — extract `ManagedIdentity`'s DashPay fields into `DashPayState`

Status: draft, rev 2 (review must-fixes folded)
Scope: `packages/rs-platform-wallet` (+ mechanical re-paths in `rs-platform-wallet-ffi`;
test-helper construction in `rs-platform-wallet-storage`). No on-disk format change, no FFI
ABI change, no Swift change. Follow-up to PR #3841 — lands as its own PR after #3841 merges.

Origin: review question on PR #3841 — "Having dashpay stuff right in identity wallet and
identity manager and managed identity is mixing too much different stuff… shall we have a
separate dashpaywallet and somehow encapsulate dashpay stuff from common identity things?"

> **Review outcome (rev 2).** Four independent reviewers (feasibility, scope, adversarial
> failure-modes, Rust/domain-fit) audited rev 1 against the code. Scope verdict:
> right-sized; two-tier design and two-commit staging earn their keep. The load-bearing
> corrections folded here: **(MF-1)** rev 1 undercounted the FFI cold-load restore path —
> it raw-writes **six** DashPay fields including all three relationship maps
> (`ffi/persistence.rs:4064/4067/4070`), so the relationship-map `apply_*` methods must be
> `pub`, not `pub(crate)`, and the boundary claim is recalibrated from "sealed" to
> "raw writes impossible; invariant-bypassing writes are named `apply_*` and auditable".
> **(MF-2)** a `pub dashpay` field is bypassable by whole-value replacement
> (`managed.dashpay = Default::default()` / `mem::take` compiles anywhere and silently wipes
> the high-water cursors) — the field itself is now private with a `dashpay()` borrow
> getter and per-field Tier B `_mut` accessors, and **no** whole-struct `dashpay_mut()`.
> **(MF-3)** rev 1 answered only one of the three sites the origin comment names; §0 now
> maps the design to all three honestly, and an optional facade-level `DashPayView`
> (zero-cost borrowing namespace — materially different from the twice-reverted owned
> facade) is added as decision point Q1. Plus: getters live on `DashPayState` itself;
> `dashpay_` field-name stutter dropped; `pub(super)` replaces the long scoped-visibility
> path; the in-crate test-fixture raw writes are inventoried and budgeted (§5); D4's method
> doc keeps the caller-side half of the cursor contract explicit.

---

## 0. What the origin comment names, and what this spec covers

The PR comment names three sites. Honest mapping:

1. **`ManagedIdentity` (state)** — the worst offender and **this spec's target**: 12 of 19
   fields are DashPay social state, all `pub`, invariants enforceable only by convention.
2. **`IdentityWallet` (network facade)** — the *largest* mixing by volume (~10.2k lines of
   DashPay ops vs ~6.6k identity-core across the `network/` impl files), but already
   file-split by concern with documented layering (`network/mod.rs`). Two owned-facade
   splits were tried and deliberately reverted on this branch (§4.1). What this spec offers
   there is the optional zero-cost `DashPayView` namespace (§3 D8, decision Q1) — call-site
   visibility without a second handle.
3. **`IdentityManager` / manager layer** — already clean: the manager holds buckets + a
   location index with no DashPay logic; DashPay sync orchestration is already its own
   coordinator (`manager/dashpay_sync.rs`). No change proposed.

## 1. Problem

`ManagedIdentity` (`src/wallet/identity/state/managed_identity/mod.rs`) mixes two concerns
in one flat, fully-`pub` struct:

- **Identity-core**: `identity`, `identity_index`, `wallet_id`, `status`, `dpns_names`,
  `contested_dpns_names`, two sync block-times.
- **DashPay social state — 12 fields**: `established_contacts`, `sent_contact_requests`,
  `incoming_contact_requests`, `ignored_senders`, `auto_accept_verify_failed`,
  `dashpay_rescan_triggered`, `dashpay_profile`, `dashpay_payments`, `contact_profiles`,
  `high_water_received_ms`, `high_water_sent_ms`, `pending_contact_crypto`.

Three concrete costs today (all verified against the code):

1. **No boundary.** Nothing in the type answers "what is DashPay vs identity-core"; the
   distinction lives in field-comment prose. Any future extraction (or reasoning about one)
   starts from zero.
2. **Invariants are bypassable — and one already lives outside the state layer.** All fields
   are `pub`, so the auto-establish invariant (reciprocal request ⇒ established contact), the
   `AUTO_ACCEPT_VERIFY_FAILED_CAP` eviction, and the ignore-emits-both changeset rule are
   upheld only by the convention of calling the right method. Worse, **high-water cursor
   monotonicity is enforced nowhere in the state layer**: the compare-and-advance rule
   (`advance_if_unchanged`, `network/contact_requests.rs:822`) is a free function in network
   code writing the fields raw (`:1363`, `:1370`). A miss reintroduces the lost-unignore bug
   that function's doc-comment describes.
3. **The FFI crate reads 9 public fields directly** inside handle closures
   (`ffi/src/dashpay_profile.rs:150` et al.) and **raw-writes six fields on the cold-load
   restore path**: `ignored_senders` (`ffi/persistence.rs:3758`), `dashpay_payments`
   (`:3806`), `contact_profiles` (`:3900`), and — via `apply_contact_rows`
   (`:3961-4077`, production load path) — all three relationship maps (`:4064/:4067/:4070`).
   The compiler offers no help distinguishing "legit restore write" from "invariant bypass".

## 2. Current architecture (facts the design relies on)

From a three-agent inventory of the state layer, all access sites, and the persistence
coupling, plus four review passes re-verifying the cites:

- **The core↔DashPay coupling is narrow.** No method in
  `state/managed_identity/{contacts,contact_requests,identity_ops,sync}.rs` mutates both an
  identity-core field and a DashPay field in the same call. Coupling is exactly: (a)
  `snapshot_changeset()` → `IdentityEntry::from_managed` (`changeset/changeset.rs:332-349`)
  reads 4 DashPay scalar fields on every core-field persist; (b) the two constructors
  initialize both groups; (c) `disable_keys` reads `wallet_id`/`identity_index` alongside a
  snapshot. DashPay mutators read only the immutable `self.id()`.
- **Mutation methods are already the norm.** Network code calls state-mutation methods 53×;
  direct production field writes outside the owner module: ~11 in `network/` (high-water ×2,
  `dashpay_rescan_triggered` ×1, `established_contacts.get_mut` ×3,
  `set_dashpay_profile`/`mark_auto_accept_verify_failed` pass-throughs, `contact_profiles`
  ×1), 12 in `state/manager/apply.rs` (changeset replay), 9 in `wallet/apply.rs` (changeset
  replay), and **9 in the FFI crate** (6 restore-path raw writes + 3 via methods). Test
  fixtures add raw writes in `wallet/apply.rs:562-567/:1348-1351`, `network/` test modules,
  `ffi/tests/test_data/mod.rs:283-286/:304`, and `contact_workflow_tests.rs:289`
  (`established_contacts.get_mut`).
- **Persistence never serializes `ManagedIdentity` itself** — it derives `Debug, Clone` only.
  Three persistence tiers cover the 12 fields:
  - `IdentityEntry` scalar snapshot (serde-derived flat struct, `changeset.rs:270-323`):
    `dashpay_profile` (merge: LWW), `dashpay_payments` (extend, LWW per txid),
    `contact_profiles` (extend, LWW per contact), `ignored_senders` (union).
  - `ContactChangeSet` (`changeset.rs:634-663`): the three relationship maps + the
    `ignored`/`unignored` tombstone pair.
  - `PlatformWalletChangeSet.pending_contact_crypto_{added,cleared}` (`changeset.rs:1189/1193`)
    for the deferred-crypto queue.
  - **In-memory only, never persisted**: `dashpay_rescan_triggered`,
    `auto_accept_verify_failed`, `high_water_received_ms`, `high_water_sent_ms`.
- **The FFI ABI is keyed off the flat entry types** (`IdentityEntryFFI::from_entry`,
  `ContactRequestFFI::from_*` — `#[repr(C)]` with pinned sizes), NOT off `ManagedIdentity`'s
  layout. Regrouping `ManagedIdentity` cannot move a single FFI byte as long as the entry
  types stay flat.
- **Two restore paths construct/populate `ManagedIdentity` outside its methods**: the
  boot/load path (in-crate: pre-built identities arrive via `IdentityManagerStartState`,
  consumed at `manager/load.rs:100` — no field-level construction; FFI loader:
  `ManagedIdentity::new` + restore writes, `ffi/persistence.rs:3692-3700`) and the
  changeset-replay path (`state/manager/apply.rs`, `wallet/apply.rs` contacts block,
  `apply_established_contact`).
- **One external struct-literal construction** exists in `rs-platform-wallet-storage`
  (`schema/identities.rs:206-237`) — test-gated (`#[cfg(any(test, feature="__test-helpers"))]`).
  It defaults the three relationship maps (loaded separately from the contacts table) and
  populates `ignored_senders` by wholesale clone.
- **The WalletPersister is a method parameter**, not a `ManagedIdentity` field. Non-persisting
  mutators return a `ContactChangeSet` for the caller to store. A sub-struct inherits the same
  two patterns unchanged.
- History: a separate DashPay surface was tried and deliberately reverted twice on this
  branch — `914e244401` folded `wallet/dashpay/` under `identity/` (duplicate files,
  bidirectional refs, "where does this live?"), `cdd0da880e` merged the `DashPayWallet<B>`
  facade into `IdentityWallet<B>` (two FFI handles, two clones per op, straddling ops like
  `accept_contact_request`).

## 3. Design

### D1 — `DashPayState` struct, privately owned by `ManagedIdentity`

New file `state/managed_identity/dashpay.rs` (a child module of `managed_identity`, sibling
of the four impl files — verified module chain makes `pub(super)` fields visible to all of
them and to nothing outside `managed_identity/`):

```rust
/// Per-identity DashPay social state: the DashPay-contract layer
/// (contacts, requests, profile, payments, deferred crypto) carried by
/// a `ManagedIdentity` on top of its identity-core fields.
#[derive(Debug, Clone, Default)]
pub struct DashPayState {
    // -- Tier A: guarded (sibling-module fields, mutate via methods) --
    pub(super) established_contacts: BTreeMap<Identifier, EstablishedContact>,
    pub(super) sent_contact_requests: BTreeMap<Identifier, ContactRequest>,
    pub(super) incoming_contact_requests: BTreeMap<Identifier, ContactRequest>,
    pub(super) ignored_senders: BTreeSet<Identifier>,
    pub(super) auto_accept_verify_failed: BTreeSet<[u8; 32]>,
    pub(super) high_water_received_ms: Option<u64>,
    pub(super) high_water_sent_ms: Option<u64>,

    // -- Tier B: open (plain data / caches, no cross-field invariant) --
    pub profile: Option<DashPayProfile>,
    pub payments: BTreeMap<String, PaymentEntry>,
    pub contact_profiles: BTreeMap<Identifier, ContactProfileEntry>,
    pub rescan_triggered: BTreeSet<Identifier>,
    pub pending_contact_crypto: Vec<PendingContactCrypto>,
}
```

On `ManagedIdentity`, the 12 flat fields are replaced by a **private** field
`dashpay: DashPayState` (private-to-`managed_identity`: visible in `mod.rs` and all child
impl files, invisible outside — this also closes the whole-value-replacement bypass, see
D3). The existing field doc-comments (several are load-bearing, e.g. the in-memory-only
rationale on `rescan_triggered` and `auto_accept_verify_failed`) move verbatim. The
`dashpay_` name prefix is dropped inside the struct (no stutter behind `dashpay()`).

**Tier assignment rationale.** Tier A = every field with a cross-field or temporal invariant:
the three relationship maps (auto-establish; rotation-supersede; both-exist precheck),
`ignored_senders` (ignore must emit `removed_incoming` + `ignored` together),
`auto_accept_verify_failed` (CAP eviction — already method-only today), the two high-water
cursors (compare-and-advance; only writer besides the sweep is `unignore_sender`'s rewind).
Tier B = independent per-key caches where a raw insert cannot corrupt sibling state, and where
the replay/restore paths and profile/payment recorders already write directly today.
`pending_contact_crypto` stays Tier B: its dedup invariant lives in the free function
`upsert_pending_contact_crypto` shared with the changeset apply path, and its drain uses
owned snapshots — capturing that in methods is real scope with no bypass bug on record
(possible follow-up, out of scope here).

### D2 — mutation methods stay on `ManagedIdentity`; signatures unchanged

Every existing mutation/query method (`add_sent_contact_request`,
`add_incoming_contact_request`, `accept_incoming_request`, `ignore_sender` /
`unignore_sender`, `set_contact_metadata`, `apply_rotated_incoming_request`,
`mark_auto_accept_verify_failed`, `should_enqueue_auto_accept`, `set_dashpay_profile`,
`record_dashpay_payment`, …) keeps its receiver, name, signature, and persister-threading
pattern; bodies reach through `self.dashpay.*`. The 53 existing method call sites don't
change. This is deliberately NOT a `DashPayState`-methods design for mutations: they need
`self.id()` and snapshot access, and moving them would churn every call site for zero
invariant gain.

### D3 — read access: one `dashpay()` borrow + getters on `DashPayState`

`ManagedIdentity` gains exactly one read accessor:

```rust
pub fn dashpay(&self) -> &DashPayState
```

Tier B fields are `pub`, so all reads flow `managed.dashpay().payments`,
`managed.dashpay().contact_profiles`, … Tier A fields get borrow getters **on
`DashPayState` itself** (next to the fields): `established_contacts()`,
`sent_contact_requests()`, `incoming_contact_requests()`, `ignored_senders()`, and by-value
`high_water_received_ms()` / `high_water_sent_ms()` (`Option<u64>` is `Copy`).
`auto_accept_verify_failed` gets NO getter — the two existing query methods on
`ManagedIdentity` (`is_auto_accept_verify_failed`, `should_enqueue_auto_accept`) cover every
reader. Existing query helpers (`is_sender_ignored`, `established_contact(&id)`,
`prior_sent_account_reference`, …) stay on `ManagedIdentity` unchanged.

Read sites (~30 network, ~18 FFI, ~55 test assertions) re-path mechanically
(`managed.established_contacts` → `managed.dashpay().established_contacts()`). Verified: no
name collisions with existing `ManagedIdentity` methods; the same-named methods on
`IdentityWallet<B>` are a different type.

Tier B **writes** get per-field mut accessors on `ManagedIdentity`: `payments_mut()`,
`contact_profiles_mut()`, `rescan_triggered_mut()`, `pending_contact_crypto_mut()`, and
`set_profile_raw` is unnecessary (`set_dashpay_profile` already exists; the replay paths use
the mut accessors). There is deliberately **no whole-struct `dashpay_mut()`** and the
`dashpay` field is private: `managed.dashpay = DashPayState::default()`, `mem::take`, and
`mem::swap` — whole-value replacements that would silently wipe Tier A state including the
cursors — do not compile outside `managed_identity/`.

`established_contact_mut(&id) -> Option<&mut EstablishedContact>` **stays, promoted to
`pub`** (documented escape hatch; `contact_workflow_tests.rs:289` — an external compilation
unit — needs it, as do three in-crate network sites that mutate contact sub-fields then
persist a hand-built `ContactChangeSet`). Sealing per-contact sub-field mutation is
follow-up scope; the boundary claim here is deliberately modest — see D5.

### D4 — capture the high-water invariant (the one real behavior-adjacent move)

`advance_if_unchanged` + `advance_high_water` (free fns,
`network/contact_requests.rs:814-832`) move onto `ManagedIdentity` as the ONLY write path
for the cursors:

```rust
/// Compare-and-advance: advance the received-direction cursor to
/// `max_fetched` (never below its current value) ONLY if the cursor
/// still holds `snapshot` — the value read at sweep start. A mid-sweep
/// `unignore_sender` rewind (reset to None) must not be clobbered by a
/// stale sweep max, or the un-ignored sender stays invisible until a
/// cold restart.
///
/// Caller contract (unchanged from the free fn): invoke only when the
/// paginate exhausted without error AND every ingest reached disk —
/// fetch/persist-success gating stays at the call site.
pub fn advance_high_water_received(&mut self, snapshot: Option<u64>, max_fetched: Option<u64>);
pub fn advance_high_water_sent(&mut self, snapshot: Option<u64>, max_fetched: Option<u64>);
```

The two raw network writes (`:1363`, `:1370`) become calls; `unignore_sender`'s rewind stays
internal to the state layer. The moved invariant is CAS + monotonicity **only**; the
fetch-succeeded/persist-succeeded gating remains caller-side convention, stated in the doc.
Semantics bit-identical: the snapshot stays a caller-supplied param, so the two-guard
interleaving with a concurrent un-ignore is unchanged.

### D5 — replay/restore writes become named `apply_*` methods

The replay and cold-load paths currently write Tier A fields raw. They get intent-named
methods that skip business invariants **by design** (establishment/ignore decisions were made
before persist; replay must reproduce state, not re-decide it). Visibility follows the
callers — the FFI crate restores relationship maps in production, so these are `pub`:

- `pub fn apply_sent_contact_request(&mut self, ContactRequest)` /
  `apply_incoming_contact_request` — for `wallet/apply.rs:197-222` and the FFI loader
  (`ffi/persistence.rs:4067/:4070`) + FFI fixtures (`ffi/tests/test_data/mod.rs:304`).
- `apply_established_contact` — exists (`contact_requests.rs:621`), promoted
  `pub(crate)` → `pub` (FFI loader `:4064`, fixtures `:283-286`). Parity note: its
  remove-both-pending-sides is a provable no-op on the cold-load path — `apply_contact_rows`'
  match arms emit exactly one of {established, sent, incoming} per contact into fresh maps.
- `pub fn apply_ignored_sender(&mut self, Identifier)` / `apply_unignored_sender` — for
  `wallet/apply.rs:255/265`, the FFI restore write (`ffi/persistence.rs:3758`), and the
  storage-crate test helper. **Implementer note:** `state/manager/apply.rs:71/:115/:143` and
  the storage helper write the *whole set* (`.extend()` union / fresh-object assign) — loop
  `apply_ignored_sender` per element. Equivalent because `:115/:143` sit on fresh-insert
  branches where the set is constructor-empty (assign ≡ insert-loop) and `:71` is already a
  union; pin this equivalence with a test.
- `pub(crate) fn apply_removed_sent(&mut self, &Identifier)` / `apply_removed_incoming` —
  only `wallet/apply.rs:225/:230` removes.
- `state/manager/apply.rs`'s remaining writes touch Tier B only (`dashpay_profile` →
  `profile`, `dashpay_payments` → `payments`, `contact_profiles`) — they use the Tier B mut
  accessors, as do the FFI restore writes to `payments`/`contact_profiles`.

Each `apply_*` doc-comment states why it bypasses the invariant and who may call it.

**Boundary claim, calibrated** (this is what the refactor actually buys): raw *field* writes
to Tier A are compile errors outside `managed_identity/`; invariant-*bypassing* writes still
exist but are named `apply_*`, greppable, and auditable — the compiler cannot distinguish a
new illegitimate `apply_*` caller from a restore path. The capability-level seal is real for
exactly two things: the high-water cursors (D4 — the only public write path enforces CAS)
and whole-value replacement of the DashPay group (D3). Everything else is
naming-and-audit, which is the honest, proportionate win.

### D6 — construction

`DashPayState` derives `Default` (every field defaults empty/None — true today for both
constructors and the cold-load path; all 12 field types are `Default`-able).
`ManagedIdentity::new` / `new_out_of_wallet` set `dashpay: DashPayState::default()`. The FFI
loader keeps `ManagedIdentity::new` + `apply_*`/Tier-B-mut writes. The storage test helper
(`storage/schema/identities.rs:206-237`) switches its literal to
`dashpay: DashPayState::default()` semantics via constructor + an `apply_ignored_sender`
loop (its relationship maps are already defaulted there; contacts load separately).

### D7 — persistence mapping updates (no wire change)

`IdentityEntry::from_managed` (lives in `crate::changeset` — reads Tier A through the `pub`
getters, Tier B through `dashpay()`). `IdentityEntry`, `ContactChangeSet`, all merge
functions, the SQLite schema, and every `#[repr(C)]` FFI mirror are **untouched**. The
on-disk format and FFI ABI provably cannot change: nothing serializes `ManagedIdentity`
(derives `Debug, Clone` only), and no FFI struct or `const` size assert is edited. Only
observable representation delta: `Debug` output nests the group under `dashpay:` — no test
parses `Debug` output (verified).

### D8 — OPTIONAL: facade-level `DashPayView` namespace (decision Q1)

The origin comment's eye was on `IdentityWallet` — where DashPay ops outnumber identity-core
ops by lines ~10.2k to ~6.6k. The twice-reverted design was a second **owned** facade (two
handles through FFI, two clones per op). A **borrowing view** has none of those costs:

```rust
pub struct DashPayView<'a, B: TransactionBroadcaster + ?Sized>(&'a IdentityWallet<B>);

impl<B: …> IdentityWallet<B> {
    pub fn dashpay(&self) -> DashPayView<'_, B> { DashPayView(self) }
}
```

The DashPay op definitions (already file-split: `contact_requests.rs`, `contacts.rs`,
`contact_info.rs`, `payments.rs`, `profile.rs`) move their `impl IdentityWallet<B>` blocks to
`impl DashPayView<'_, B>` — one route per op, no forwarding shims. Call sites become
`wallet.identity().dashpay().send_contact_request(…)`. FFI **function signatures are
unchanged** (they re-path internally); Swift is untouched. Cost: a mechanical re-path of the
FFI + internal DashPay call sites and the sync coordinator; zero new state, zero clones.
This is the piece that makes the DashPay/identity boundary visible at every call site — but
it is severable: commits 1–2 stand alone if this is declined.

## 4. Alternatives rejected

1. **Separate owned `DashPayWallet` facade (the PR comment's literal suggestion).** Tried and
   reverted twice on this very branch (§2 history). The ops straddle the boundary
   (`accept_contact_request` = identity signing + DashPay docs; payments = core broadcaster;
   contact crypto = identity DIP-9/14 keys), so a second owned facade re-creates the same
   handle with a different name, re-splits ops that straddle, and re-introduces FFI
   handle-juggling. DashPay is a *layer on the identity aggregate*, not a sibling domain.
   Rejected on evidence, not taste. (The borrowing view in D8 is the surviving kernel of
   this idea — namespace without ownership.)
2. **Separate DashPay store keyed by identity id** (e.g. `IdentityManager.dashpay:
   BTreeMap<Identifier, DashPayState>`). Splits one aggregate into two maps that must stay
   key-synchronized through add/remove/apply/load; every combined read becomes a two-map
   join; the changeset apply and both restore paths get a second lookup + orphan mode. All
   cost, and the "is it one thing?" answer is unchanged — a DashPay state without its
   identity is meaningless.
3. **Extension-trait split of `IdentityWallet`** (`DashPayOps` trait). Pure cosmetics: state
   stays mixed, callers add trait imports, and the network layer is already file-split by
   concern. The borrowing view (D8) achieves the namespacing without the trait ceremony.
4. **Full lockdown (all 12 fields private, no `_mut` escape hatch).** Forces dedicated
   methods for per-contact sub-field mutation (3 network sites), the profile fetch-cache
   writer, payments recorder internals, and the pending-crypto upsert/drain — roughly
   doubles the new-method surface to protect fields with no cross-field invariants and no
   observed bypass bugs. Poor cost/benefit now; Tier A→B promotion later is cheap.
5. **Stop after commit 1 (all-`pub` regroup, no encapsulation).** Fixes cost 1 of §1
   (boundary/naming) but leaves costs 2–3 untouched: the high-water cursors stay raw
   network writes guarding a documented lost-unignore bug, and the FFI keeps 6
   indistinguishable raw restore writes. The ~14 new methods in commit 2 are earned by
   exactly those two costs.
6. **Docs only (comment banner grouping the fields).** Zero enforcement; the next reviewer
   asks the same question.

## 5. Migration & staging

Own PR, based on `v4.1-dev` **after #3841 merges** (this touches the same files as #3841's
tail; doing it inside would bloat an already-huge diff and re-trigger full re-review).

Commits, each independently green:

1. **Mechanical regroup.** Introduce `DashPayState` with ALL fields temporarily `pub` (and
   the `dashpay` field `pub`); move the 12 fields (renaming the three `dashpay_`-prefixed
   ones); re-path every access (`managed.X` → `managed.dashpay.X`). No visibility change, no
   method change. Compile-error-driven; behavior-identical by construction. Also deletes
   the orphaned 0-byte `state/managed_identity/tests.rs` left by `914e244401`.
2. **Encapsulate.** Apply tier visibilities + private `dashpay` field; add `dashpay()`,
   Tier A getters, Tier B mut accessors, `advance_high_water_*`, the `apply_*` family;
   convert the network + FFI + replay sites; move the two high-water free fns into the
   state layer with their tests. **Test-fixture conversions budgeted here** (not
   "mechanical re-paths"): `wallet/apply.rs:562-567/:1348-1351` (insert → `apply_*`),
   `network/contact_requests.rs` fixture sites (~3411-3418 flag write →
   `established_contact_mut`; ~3560-3564 `ignored_senders.clear()` — no direct equivalent,
   becomes clone-keys + `apply_unignored_sender` loop), `network/payments.rs:1535/1610/2537`
   + `network/contact_info.rs` helper (insert → `apply_*`), FFI fixtures
   (`ffi/tests/test_data/mod.rs`), `contact_workflow_tests.rs:289`
   (→ `established_contact_mut`).
3. **(Optional, decision Q1) `DashPayView` facade namespace** per D8.

Rollback story: each commit reverts independently of the ones before it.

## 6. Failure modes & risks

- **Missed access site** → compile error (loud, the mechanism working as intended). Zero
  runtime discovery.
- **High-water semantics drift** (the only logic that *moves*): mitigated by porting the
  existing free-fn unit tests unchanged, plus new method-level tests written to pass against
  the free fn's behavior BEFORE the move (kill-the-mutant check: `snapshot != current` must
  leave the field untouched).
- **Replay-path behavior change**: `apply_*` methods must reproduce today's raw writes
  exactly. Verified caller-side semantics that must NOT move into the methods:
  `wallet/apply.rs` keys inserts off `entry.request.recipient_id`/`sender_id`, warns on
  orphan inserts but is silent on orphan removes, orders inserts-before-removes and
  unignore-after-ignore (un-ignore wins) — all stay in `wallet/apply.rs`. The
  `ignored_senders` assign-vs-loop equivalence (D5) gets a pinning test.
- **Borrow-checker fallout**: adversarial pass verified all existing sites compile under the
  new surface (the three `get_mut` sites touch only the contact + persister while the `&mut`
  is live; loops over Tier A maps are read-only in-body; mutating sites collect inputs before
  taking `&mut`). New sites holding a getter-returned borrow across a `&mut` call will fail
  to compile — clone first in tests.
- **FFI restore path**: `apply_*` parity is exact (raw inserts have no side effects;
  `apply_established_contact`'s extra removes are a no-op on fresh maps — D5).
- **Merge risk with in-flight DashPay work**: pure mechanics; land in a quiet window.
  Orthogonal to the pending-contact-crypto follow-ups (that spec moved the field *onto* the
  identity; this one only re-paths it).
- **Residual escape hatches — the honest list**: `established_contact_mut` (`pub`), Tier B
  `pub` fields + mut accessors, and the **`pub apply_*` family itself** (any crate can call
  `apply_ignored_sender` instead of `ignore_sender`, skipping the tombstone contract — same
  exposure as today's `pub` fields, but now named and greppable). The boundary claim is
  D5's calibrated version, not "all DashPay state is sealed".

## 7. Test & verification plan

- **Existing suites are the harness** (behavior-preserving refactor): full
  `rs-platform-wallet` lib tests (the auto-establish, rotation, ignore/unignore, CAP-eviction,
  persist-before-commit pins all keep passing untouched), `contact_workflow_tests`,
  `rs-platform-wallet-ffi` lib + integration tests.
- **New unit tests** (written against the CURRENT free-fn behavior first, then the method):
  `advance_high_water_{received,sent}` — advance, never-below, `None`-snapshot,
  mid-sweep-rewind-preserved; `apply_ignored_sender` loop ≡ wholesale-assign parity;
  `apply_sent/incoming_contact_request` parity with today's `wallet/apply.rs` raw inserts
  (including the no-auto-establish property of the replay path).
- **Visibility is its own test — for what it actually seals**: after commit 2, a Tier A raw
  *field* write or a whole-`dashpay` replacement outside `managed_identity/` is a compile
  error. (`apply_*` misuse is not compiler-catchable — that's the calibrated D5 claim.)
- **Local CI mirror**: `cargo clippy --workspace --all-features` + `cargo fmt --check --all`
  (targeted `-p` builds miss feature-gated callers).
- **iOS**: rebuild the xcframework + run the existing FFI persistence round-trip tests; no
  Swift source change expected (assert: `git diff --stat` on `packages/swift-sdk` is empty).

## 8. Open questions (decision points for the PR author)

1. **Include commit 3 (`DashPayView` facade namespace, D8)?** Recommendation: yes — it is
   the only part of this spec that changes what the origin comment's author *sees* at the
   `IdentityWallet` layer, and it is zero-cost at runtime. But commits 1–2 deliver the
   state-layer value standalone; declining Q1 drops D8 with no other edits.
2. Should `payments` be Tier A? `record_dashpay_payment` has rollback-on-persist semantics,
   but the FFI restore + overlay paths write it raw; sealing it means two more `apply_*`
   methods. Proposed: keep Tier B now.
3. DPNS fields (`dpns_names`, `contested_dpns_names`): once `DashPayState` lands, their
   loose placement becomes the next obvious question. Position: deliberately-separate
   follow-up using the identical pattern ("dashpay first, dpns next"), not scope here.
