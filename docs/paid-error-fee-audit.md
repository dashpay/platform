# Paid-Error Fee Charging Audit

**Branch:** `fix/batch-paid-error-fee-charging`
**Context:** PR #3616 (commit `dfb6d846f9`) flipped invalid batch transitions from `UnpaidConsensusError` to `PaidConsensusError` on `PROTOCOL_VERSION_12+`. Under the new semantics, a failed batch lands as a `BumpIdentityDataContractNonce` action — the user pays for the validation work that ran. But validation does drive reads (contract fetches, document queries, identity lookups) whose cost is sometimes dropped on the floor. Result: the user is under-charged.

Non-batch state transitions already had paid-error semantics before #3616, so the same class of under-charging has been latent there for some time.

## ⚠️ CRITICAL CONSTRAINT — consensus-affecting; new function version required

**Changing the fee a user pays is consensus-affecting.** The chain must reproduce historical block validation bit-for-bit on every protocol version that has shipped. `LATEST_PLATFORM_VERSION = PLATFORM_V12` (`rs-platform-version/src/version/protocol_version.rs:68`) — meaning PV11 and PV12 are frozen.

**Rule:** every fix below must ship as a **new function version**, gated by a new field on `PlatformVersion`, with v0/v1 behavior preserved verbatim for replay. The target protocol version is **the next one** (current candidate: `PROTOCOL_VERSION_13` — confirm with platform-version policy before implementing).

**What this looks like in practice** (modeled on the `failed_per_transition_action` versioning that #3616 introduced):

1. Bump (or add) a version field on the relevant struct in `rs-platform-version/src/version/dpp_versions/` or `drive_abci_versions/`. Existing pattern reuses the dedicated field that already gates the function — e.g. `batch_state_transition.transform_into_action: 0 → 1`.
2. In `v11.rs` (or earlier) the field stays at its old value; in `v12.rs` (or the targeted version) the field gets the new value.
3. **Add a new function version** `_v1` alongside the existing `_v0` with the new behavior. `_v0` stays **byte-identical** to the version that shipped.
4. Dispatch on the field at the call site — the version branch happens **outside** the function, not inside:
   ```rust
   match platform_version.drive_abci.validation_and_processing
       .state_transitions.batch_state_transition.transform_into_action
   {
       0 => self.transform_into_action_v0(/* old signature */),
       1 => self.transform_into_action_v1(/* new signature, threaded ctx */),
       v => return Err(UnknownVersionMismatch { ... }),
   }
   ```
5. **Never modify** an existing `_v0` (or any shipped `_vN`) function body. If a new field value needs different behavior, add `_v(N+1)` and route to it.

**Exception for the ~1100-line transformer body** (per the comment at `transformer/v0/mod.rs:1-22`): the `try_into_action_v0` / `transform_document_transition_v0` / etc. functions in that file are intentionally kept at `_v0` and gated at *finer* version-field granularity inside them (e.g. `failed_per_transition_action`, `flatten`, `merge_many`). Bumping their suffix would force copy-pasting the entire file as a v0 archive — exactly the regression #3616 set out to avoid. The B7 fix follows this pattern: `try_into_action_v0` is unchanged and continues to be the single transformer entry-point; only the *outer wrapper* `transform_into_action_v0` got a `_v1` sibling to thread the ctx through.

## Fee plumbing — how a drive read becomes a charge

1. Validator calls `execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result))` after each billable drive op.
2. After validation, `execute_event/v0/mod.rs:61` calls `ValidationOperation::add_many_to_fee_result(&execution_operations, &mut individual_fee_result, ...)`, adding accumulated validation cost on top of the action's drive-op fee.
3. The combined fee is what the user pays — works the same for successful actions and `BumpIdentityDataContractNonce` paid-error actions.

**Implication:** the canonical fix for every site below is "compute the FeeResult, push it into `execution_context`" — except for the data-trigger sites where the context ref is immutable (architectural blocker — see Tier 4).

## Severity scale

- **HIGH** — runs on every transition of its kind, discards measurable cost, hits the paid-error or successful-action path
- **MEDIUM** — runs in many but not all transitions OR smaller per-op cost
- **LOW** — dead code / billed elsewhere via a sibling path
- **NONE** — stale TODO, not actually a fee gap

---

## Tier 0 — Pipeline-level fee leak (discovered while verifying B1/B2)

### B7 — Transformer's execution_context is a dropped local

**Severity: HIGH+ (likely dwarfs all individual leaks below)**

The outer `state_transition_execution_context` created by the processor (`processor/v0/mod.rs:43`, threaded as `&mut` into every other phase) **is not threaded into the batch transformer**. Instead:

**`batch/mod.rs:57-84`** — `transform_into_action`:
```rust
fn transform_into_action<C: CoreRPCLike>(
    &self, /* ... */
    _execution_context: &mut StateTransitionExecutionContext,  // <-- parameter ignored (underscore-prefixed)
    tx: TransactionArg,
) -> Result<...> {
    /* ... */
    0 => self.transform_into_action_v0(&platform.into(), block_info, validation_mode, tx),
    //                                                                                 ^^^ outer ctx NOT passed through
}
```

**`state/v0/mod.rs:323-344`** — `transform_into_action_v0`:
```rust
fn transform_into_action_v0(&self, /* ... no ctx param ... */) -> Result<...> {
    let mut execution_context =                                            // <-- LOCAL ctx created
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    let validation_result = self.try_into_action_v0(/* ... */, &mut execution_context)?;
    //                                                          ^^^ transformer add_operation calls go here

    Ok(validation_result.map(Into::into))                                  // <-- LOCAL ctx dropped, all ops discarded
}
```

**Consequence:** every `execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result))` inside the batch transformer is silently discarded. That includes:

- **Token base transition group-action lookups** in `try_from_borrowed_base_transition_with_contract_lookup` (`rs-drive/src/state_transition_action/batch/batched_transition/token_transition/token_base_transition_action/v0/transformer.rs:289-340`): `fetch_action_is_closed`, `fetch_action_id_signers_power_and_add_operations`, `fetch_active_action_info_and_add_operations`. These accumulate real `LowLevelDriveOperation` entries that get converted via `Drive::calculate_fee(...)` into the returned `FeeResult` — which is then added to the dropped local ctx.
- **All `try_from_borrowed_token_*_with_contract_lookup`** fee_results: `transformer/v0/mod.rs:586, 596, 606, 619, 629, 639, 649, 659, 669, 679, 689`.
- **All `try_from_document_borrowed_*_transition_with_contract_lookup`** fee_results: `transformer/v0/mod.rs:748, 819, 829, 894, 959`.

**Severity assessment:**
- For token group actions: confirmed real cost loss — `validate_state` performs different reads (`fetch_identity_token_balance`) than the transformer's group-action reads, so the costs don't overlap.
- For non-group tokens: the `None` arm at `token_base_transition_action/v0/transformer.rs:244-245` skips the group reads — actual loss is minimal.
- For document creates/replaces: depends on what `try_from_document_borrowed_*` actually reads vs. what `validate_state` re-reads. Requires per-transition-kind audit.

**B1, B2, B3 are subsumed by B7.** Even if you switched `epoch=None` → `epoch=Some(...)` to make those fetches produce a `FeeResult`, that result would still flow into the local ctx and be dropped.

**Fix approach** (gated by new protocol version per the constraint above):
- Thread `&mut execution_context` from `batch/mod.rs:65` through `transform_into_action_v0` down into `try_into_action_v0`. The outer ctx already exists at the processor; the local-ctx wrapper just needs to be removed (or repurposed as a no-op pre-PV-bump for replay).
- Verify the existing v0 behavior is preserved by reading via the version field — current behavior is "discard ops", new behavior is "thread through".

**Open question for archaeology pass:** is the local-ctx pattern intentional ("transformer reads are free by design") or a latent bug? Initial git-blame shows the pattern has existed since `8884df7f58` (the squashed commit that introduced batch state validation). No comment justifying it. Probably an oversight, but worth a definitive answer before treating as a bug.

---

## Tier 1 — Batch path (new bugs surfaced by #3616)

### Contract fetches in batch transformer

| # | file:line | what's discarded | severity |
|---|---|---|---|
| B1 | `state_transitions/batch/transformer/v0/mod.rs:365` | Token contract fetch cost — `get_contract_with_fetch_info_and_fee(...)` called with `epoch=None`, so `FeeResult` is `None` by design. Runs on every batch with token transitions. | **SUBSUMED BY B7** — even with `epoch=Some(...)`, the resulting fee would flow into the dropped local ctx. Verified: not billed upstream (basic structure does no reads; nonce validation bills its own reads only; signature/advanced-structure phases don't refetch the contract; per-action `validate_state` reads contract from `data_contract_fetch_info_ref()` of the action, not a fresh fetch). |
| B2 | `state_transitions/batch/transformer/v0/mod.rs:418` | Document contract fetch cost — same pattern as B1. Runs on every batch with document transitions. | **SUBSUMED BY B7** |
| B3 | `state_transitions/batch/state/v0/fetch_documents.rs:91` | Internal contract fetch cost in `fetch_documents_for_transitions_knowing_contract_id_and_document_type_name`. Function is `#[allow(dead_code)]` but reachable via other paths to audit. | LOW |

**B1/B2 resolved:** verified there is no upstream billing site. The contract is fetched exactly once during the batch pipeline (in the transformer), with `epoch=None` deliberately suppressing fee computation, into a context that gets dropped. Three independent reasons it's unbilled — fixing any one of them is necessary but not sufficient without also fixing B7.

### Document query in batch transformer

| # | file:line | what's discarded | severity |
|---|---|---|---|
| B4 | `state_transitions/batch/state/v0/fetch_documents.rs:168` | `drive.query_documents(...)` cost from `fetch_documents_for_transitions_knowing_contract_and_document_type`. Documented as `// todo: deal with cost of this operation`. Used by transformer/v0/mod.rs:511 on every batch with replace/transfer/purchase/update-price. | **HIGH** — this is the headliner |
| B5 | `state_transitions/batch/state/v0/fetch_documents.rs:211` | `drive.query_documents(...)` cost from `fetch_document_with_id` — function returns `FeeResult` separately, billing is caller-dependent. | MEDIUM |

### Failed-batch path

| # | file:line | what's discarded | severity |
|---|---|---|---|
| B6 | `transformer/v0/mod.rs:1119` (`failed_per_transition_action`) | When validation fails after the document query at line 511 succeeded, the query cost was never added to `execution_context` (see B4). User pays only `BumpIdentityDataContractNonce` cost. | HIGH (duplicate of B4 — fixing B4 fixes this) |

---

## Tier 2 — Data triggers (new bugs surfaced by #3616 + architectural blocker)

**Architectural blocker:** `DataTriggerExecutionContext` holds an immutable `&'a StateTransitionExecutionContext`. Triggers cannot call `add_operation()` because that method requires `&mut self`. Fixing the trigger fee leaks requires either:
- Refactoring `DataTriggerExecutionContext` to hold `&'a mut StateTransitionExecutionContext`, OR
- Returning a `FeeResult` from each trigger fn and having the caller bill it.

The second option is less invasive but spreads fee accounting across the trigger dispatcher.

| # | file:line | trigger | what's discarded | severity |
|---|---|---|---|---|
| T1 | `data_triggers/triggers/dpns/v0/mod.rs:246` | dpns: `create_domain_data_trigger_v0` | `drive.query_documents(...)` cost — parent domain lookup. Runs when creating a subdomain. | HIGH |
| T2 | `data_triggers/triggers/dpns/v0/mod.rs:337` | dpns: `create_domain_data_trigger_v0` | `drive.query_documents(...)` cost — preorder lookup. Runs on every DPNS domain create. | HIGH |
| T3 | `data_triggers/triggers/dashpay/v0/mod.rs:77` | dashpay: `create_contact_request_data_trigger_v0` | `drive.fetch_identity_balance(...)` cost — recipient identity check. Runs on every contact request. (Line 74 has `// TODO: Calculate fee operations` placeholder.) | HIGH |
| T4 | `data_triggers/triggers/withdrawals/v0/mod.rs:79` | withdrawals: `delete_withdrawal_data_trigger_v0` | `drive.query_documents(...)` cost — withdrawal document fetch. Runs on every withdrawal delete. | HIGH |
| T5 | `data_triggers/triggers/reject/v0/mod.rs` | reject | (no drive ops — pure validation) | NONE |

---

## Tier 3 — Non-batch state transitions (pre-existing bugs, predate #3616)

### identity_credit_transfer

| # | file:line | what's discarded | severity |
|---|---|---|---|
| N1 | `state_transitions/identity_credit_transfer/state/v0/mod.rs:38` | `drive.fetch_identity_balance(self.identity_id(), ...)` — sender balance fetch. Runs every transition. | HIGH |
| N2 | `state_transitions/identity_credit_transfer/state/v0/mod.rs:61` | `drive.fetch_identity_balance(self.recipient_id(), ...)` — recipient balance fetch. Runs every transition. | HIGH |

### identity_create / identity_create_from_addresses

| # | file:line | what's discarded | severity |
|---|---|---|---|
| N3 | `state_transitions/identity_create/state/v0/mod.rs:75` | `drive.fetch_identity_balance(identity_id, ...)` — existence check. | HIGH |
| N4 | `state_transitions/identity_create_from_addresses/state/v0/mod.rs:56` | `drive.fetch_identity_balance(identity_id, ...)` — existence check. | HIGH |

### masternode_vote

| # | file:line | what's discarded | severity |
|---|---|---|---|
| N5 | `state_transitions/masternode_vote/transform_into_action/v0/mod.rs:53` | `drive.fetch_identity_contested_resource_vote(...)` — existing vote lookup. | MEDIUM |

### Shared validators in `common/`

These are called from multiple transition kinds, so a single fix benefits many sites.

| # | file:line | what's discarded | severity |
|---|---|---|---|
| N6 | `common/validate_identity_public_key_contract_bounds/v0/mod.rs:61` | Has explicit `//todo: we should add to the execution context the cost of fetching contracts`. All contract fetches and key lookups inside this function are unbilled. Called on every identity_update with contract-bounded keys. | HIGH |
| N7 | `common/validate_identity_public_key_contract_bounds/v0/mod.rs:67,111,154,185,234,269` | Six internal `get_contract_with_fetch_info` / `fetch_identity_keys` calls — each runs on different bounds patterns. | MEDIUM (covered by N6 root-cause fix) |
| N8 | `common/validate_identity_public_key_ids_dont_exist_in_state/v0/mod.rs:39` | `drive.fetch_identity_keys::<KeyIDVec>(...)` — key-ID existence check. Called on identity_update adding keys and identity_create. | HIGH |
| N9 | `common/validate_identity_public_key_ids_exist_in_state/v0/mod.rs:34` | `drive.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(...)` — key fetch for disable/remove. Called on identity_update disabling keys. | HIGH |
| N10 | `common/validate_unique_identity_public_key_hashes_in_state/v0/mod.rs:35` | `drive.has_any_of_unique_public_key_hashes(...)` — global key-hash uniqueness. Called on identity_create, identity_create_from_addresses, identity_update with added keys. | HIGH |
| N11 | `common/asset_lock/proof/verify_is_not_spent/v0/mod.rs:30` | `drive.fetch_asset_lock_outpoint_info(...)` — asset-lock spent check. Called on identity_create, identity_top_up, address_funding_from_asset_lock, shield_from_asset_lock. | HIGH |
| N12 | `common/asset_lock/proof/validate/instant/mod.rs:57` | Local instant-lock signature verification — has `// TODO: Shouldn't we add an operation for fees?` | MEDIUM |

---

## Severity rollup

| Tier | HIGH count | MEDIUM count | LOW/NONE count |
|---|---|---|---|
| **Tier 0 (pipeline)** | **1 (B7 — likely dominates all batch leaks)** | | |
| Tier 1 (batch) | 1 standalone (B4) — B1/B2 subsumed by B7 | 1 (B5) | 1 (B3) |
| Tier 2 (triggers) | 4 (T1, T2, T3, T4) | 0 | 1 (T5) |
| Tier 3 (non-batch) | 7 (N1–N4, N6, N8–N11) | 4 (N5, N7, N12, partially N6 sub-sites) | 0 |
| **Total HIGH** | **13 + B7 (pipeline-level)** | | |

## Open questions to resolve in implementation phase

1. ~~**B1/B2 — is the batch transformer's contract fetch already billed upstream?**~~ **RESOLVED** — no, and B1/B2 are subsumed by B7. Fixing B7 (threading the outer ctx through the transformer) is necessary but not sufficient to bill the contract fetch — the `epoch=None` parameter must also become `Some(...)` so a `FeeResult` is actually computed.
2. **B7 — confirm the local-ctx drop is unintentional, not a deliberate "transformer reads are free" policy.** Git-blame to `8884df7f58` (squashed commit) shows no comment justifying it. Recommend posting to PR description as a discovery rather than treating as confirmed bug until reviewed.
3. **Trigger architecture (T1–T4) — refactor context to `&mut` vs. return FeeResult?** The first is cleaner but touches more code.
4. **PR splitting:** B7 + Tier 1 + Tier 2 ship together (one PV13 gate covering all batch-path billing). Tier 3 is pre-existing under-billing in other transition kinds — could be the same PV13 bump or a separate PR. Decision affects diff size and review surface.
5. **Test strategy:** for each HIGH site, write a regression test that runs the same input under both `PLATFORM_V12` (old: under-billed) and `PLATFORM_V13` (new: correctly billed), asserting the fee delta. Pattern after the existing `replayed_failed_replace_with_consumed_nonce_must_be_rejected_at_check_tx` test in `batch/tests/document/replacement.rs`, which already uses the dual-version pattern from #3616.

## Next steps

Walk through each entry, in this order:
1. **B7 first** (it dominates the batch leak and gates how everything else gets fixed) — write a regression test that captures the dropped transformer-phase fee, confirm red on V12, design the version-field plumbing, implement.
2. Then **T1–T4** (triggers) — needs the architectural decision on plumbing first.
3. Then **N1–N12** (non-batch) — can proceed in parallel once the version-gate pattern is established.

Per CLAUDE.md TDD discipline: every fix gets a test that goes red on V12 → green on V13 in the same commit.

---

## Implementation approach (B7 + B4)

**User directives (2026-05-19):**
- Target = **PV12 re-cut** (V8 amended in place; not a new platform version). Confirms PV12 hasn't shipped to mainnet yet.
- **Reuse existing version field** `batch_state_transition.transform_into_action` (bump 0 → 1) rather than adding a new field.
- **Separate tests and separate commits** for B7 and B4 — so each leak is *independently observable*. If B4 were fixed first or bundled, fixing B7 wouldn't move any fee number ("B4 can hide B7"). Order matters: B7 first, then B4.

### Two-commit shape

| Commit | Code change | Test added | Demonstrates |
|---|---|---|---|
| 1 — **B7 fix** | Thread outer `execution_context` through `batch/mod.rs` and `state/v0/mod.rs`. Gate behavior on `transform_into_action: 0 / 1`. Bump V8's value to 1. | New test using a scenario with non-zero transformer-phase reads (token group action OR contested document create). Asserts the post-fix fee value. | B7 is real and isolatable: only group-action / contested-doc scenarios change fee. Non-group simple replaces (B4-affected) remain unchanged. |
| 2 — **B4 fix** | Change `fetch_documents_for_transitions_knowing_contract_and_document_type` to return cost (or accept ctx); bill at callsite in transformer/v0/mod.rs:511. Same `transform_into_action: 1` gate. | New test using a simple successful document replace. Asserts the post-fix fee value, with the delta = query_documents cost. | B4 is a SEPARATE bug from B7 — even after B7's fix, simple replaces still leaked the query cost. |

**Why this order matters:** B4's fix has no user-visible effect without B7 — `fetch_documents.rs:168` is called from inside the transformer, whose `execution_context` is the dropped local. If we did B4 first, fixing the query cost would add it to a context that gets discarded → no fee change → can't tell whether B4 is real or fixed. So **B7 must land first, then B4 builds on top**.

### Original combined approach (kept for reference)

These two fix together because they touch the same call chain:
- **B7** is the structural fix — thread the outer `execution_context` through `batch/mod.rs:57-84` and `state/v0/mod.rs:323-344` so the transformer's `add_operation` calls actually land in the per-transition fee.
- **B4** is the leaf fix — `fetch_documents_for_transitions_knowing_contract_and_document_type` at `state/v0/fetch_documents.rs:128` calls `drive.query_documents(...)` and discards `documents_outcome.cost()`. After B7, this cost has a home; before B7, fixing B4 in isolation does nothing.

### Version-field plumbing

Following the `failed_per_transition_action` model from #3616:

1. Add a new field on `DriveAbciDocumentsStateTransitionValidationVersions` (in `rs-platform-version/src/version/drive_abci_versions/drive_abci_validation_versions/mod.rs`):
   ```rust
   pub batch_transformer_billing: FeatureVersion,
   ```
2. Existing v1.rs through v8.rs (covering PV1–PV12): explicitly `batch_transformer_billing: 0`.
3. New v9.rs (for PV13): `batch_transformer_billing: 1`.
4. New v13.rs in `rs-platform-version/src/version/`: wires `DRIVE_ABCI_VALIDATION_VERSIONS_V9` into `PLATFORM_V13`.
5. `protocol_version.rs:68` `LATEST_PLATFORM_VERSION = &PLATFORM_V13`.

### Code-level branch points (B7, as shipped)

Two function versions side-by-side; dispatch outside.

| Branch site | v0 behavior (PV11, frozen — `transform_into_action: 0`) | v1 behavior (PV12, new — `transform_into_action: 1`) |
|---|---|---|
| `batch/mod.rs:57-84` `transform_into_action` | Match arm `0` → calls `transform_into_action_v0(...)` (no ctx) | Match arm `1` → calls `transform_into_action_v1(..., execution_context, ...)` |
| `state/v0/mod.rs::transform_into_action_v0` | **Byte-identical to v3.1-dev original.** Creates local ctx, calls `try_into_action_v0`, drops local on return | Untouched |
| `state/v0/mod.rs::transform_into_action_v1` | N/A | New function. Takes the outer `execution_context` and threads it into `try_into_action_v0` |
| Transformer add_operation calls inside `try_into_action_v0` (lines 586, 596, 606, 619, 629, 639, 649, 659, 669, 679, 689, 748, 819, 829, 894, 959) | Land in the local ctx that `_v0` drops | Land in the outer ctx that `_v1` threads → billed to the user |

The transformer body (`try_into_action_v0` and all its helpers in `transformer/v0/mod.rs`) is **unchanged** — same single function, called by both wrappers. Only the wrapper's choice of which ctx to pass differs.

### Code-level branch points (B4, next commit)

| Branch site | v0 behavior (PV11, frozen) | v1 behavior (PV12, new) |
|---|---|---|
| `state/v0/fetch_documents.rs::fetch_documents_for_transitions_knowing_contract_and_document_type` | Discards `documents_outcome.cost()` (keeps current signature) | New function variant returns `(Vec<Document>, FeeResult)` |
| `transformer/v0/mod.rs:511` (callsite for ↑) | No fee accounting (kept for v0 transformer wrapper, called only from `_v0` path) | After call, `execution_context.add_operation(PrecalculatedOperation(fee))` |

To preserve the `try_into_action_v0` single-entry-point invariant, B4 will likely thread cost via a return-value channel that the existing function can ignore — see B4 implementation notes when that commit lands.

### Test scenarios

Two test scenarios cover both leaks:

1. **`test_successful_document_replace_fee_protocol_version_12` (new, pins V12 baseline)** — exercises B4. Successful Replace of a mutable profile document. V12 fee captured as the under-billing baseline. Asserts current `aggregated_fees().processing_fee` value.
2. **`test_successful_document_replace_fee_protocol_version_13` (new, pins V13 fix)** — same scenario, but on PV13. Asserts the fee is higher by exactly the `query_documents` cost. Δ ≈ the cost of fetching one document from the documents subtree.

Optional follow-up tests for B7-only (token-group-action and contested-document-create scenarios) can land in the same PR if time permits.

### Files to touch (B7 + B4 first PR)

```
packages/rs-platform-version/src/version/
  drive_abci_versions/drive_abci_validation_versions/mod.rs        # +field on struct
  drive_abci_versions/drive_abci_validation_versions/v1.rs..v8.rs  # +field = 0 (8 files)
  drive_abci_versions/drive_abci_validation_versions/v9.rs         # NEW: field = 1
  v13.rs                                                            # NEW
  protocol_version.rs                                              # +PLATFORM_V13 to list, bump LATEST

packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/batch/
  mod.rs                                                           # thread ctx in transform_into_action
  state/v0/mod.rs                                                  # thread ctx in transform_into_action_v0; gate local-ctx creation
  state/v0/fetch_documents.rs                                      # add fee return + version gate
  transformer/v0/mod.rs                                            # bill the query cost at callsite (gated)
  tests/document/replacement.rs                                    # add 2 new tests (V12 baseline + V13 fix)
```

### Pre-flight checks

Before starting code: confirm `LATEST_PLATFORM_VERSION` is V12, no test pins fees on V13 (none exist yet), and `protocol_version.rs` is the only place that needs `LATEST_PLATFORM_VERSION` updated. Also: search for any explicit `.latest()` / `PV13` references that would need staging.
