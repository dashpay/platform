# SPEC: Internal-Tagging Sweep for Externally-Tagged Enums (PR #3573)

> ## Outcome (COMPLETE)
> All **8 data-carrying enums** were converted to internal `type` tagging and
> merged into the branch (commits `51d31930d9`..`40fa0e9f5c`, plus the
> `bls_pubkey_serde` robustness fix `a7f3c60e38`). dpp lib **3812/0**, clippy clean.
> **Corrections the multi-agent review + implementation made to this draft:**
> - `DistributionFunction::Stepwise(BTreeMap)` **auto-derives fine** (a spike
>   refuted "needs custom impl"); both Tier-A enums are pure auto-derive.
> - The tuple/primitive enums use a clean `#[serde(into/from = "Repr")]` to a
>   struct-variant `Repr` — **not** hand-written visitors as drafted.
> - **u128 gotcha** (found during impl): serde's internal-tag `Content` buffer
>   can't hold a u128; encode Content-safe (u64-or-string, never `serialize_u128`).
> - `TokenConfigurationChangeItem` is a plain derive (not a custom `JsonSafeFields`)
>   and its `MaxSupply` had **no** json_safe — a latent bug, now fixed.
> - Consumer blast radius was overstated: wasm-dpp2 converters are hand-built
>   (serde-shape-independent); the only external serde consumer was 1 JS fixture.
> - `ArrayItemType` was PascalCase (`{"String":[..]}`), so it never matched
>   JSON-Schema's lowercase `type` vocabulary — converting it is safe.
>
> The draft below is preserved as the pre-implementation planning record.

**Status:** SUPERSEDED by the outcome above (was: DRAFT — pending multi-agent review)
**Branch:** `feat/json-convertible-address-transitions`
**Author:** Claude (Fable) for Ivan Shumkov
**Date:** 2026-06-17
**Related:** `docs/json-value-unification-plan.md` (§"Tag-key conventions"), commit `7b057b88d1` (StoredAssetLockInfo precedent)

---

## 1. Problem

The json/value unification convention states (`json-value-unification-plan.md`, codified this branch):

> **All sum types are internally tagged. No `data` wrapper.**
> **Discriminator key uses `$` prefix only when the wire-shape level has other `$`-prefixed fields. Plain key otherwise.**

Most enums on this branch were converted. PR review (shumkov) flagged that several **data-carrying enums in the PR's changed files remain externally tagged** — i.e. they serialize as `{"VariantName": <payload>}` (newtype/struct variants) instead of a flat `{"type": "variantName", …}`. This is inconsistent with the convention and is awkward for JS consumers (variant discovery requires `Object.keys(obj)[0]` and the shape flips between string and object across variants).

`StoredAssetLockInfo` was one such straggler, already fixed (`7b057b88d1`). Review comments now ask: *"how many more enums missing `$type`? fix them all?"*

This spec answers that with a verified inventory and a per-enum plan, and is the gate before any code is written.

## 2. Goal & success criteria

1. Every **in-scope** data-carrying sum type serializes with an **internal discriminator** per the codified rule (`type`, or `$type` only when `$`-fields coexist at the wire level).
2. JSON **and** `platform_value::Value` round-trips preserved; each gets a **static-fixture assertion** pinning the new shape (the `AddressWitness` standard).
3. **Zero consensus impact** — bincode `Encode`/`Decode` are independent of serde (per the plan doc); only the serde wire reshapes.
4. **No silent downstream breakage** — every consumer that depends on the old serde shape (notably `wasm-dpp2` manual converters and any JS/TS) is updated in lockstep, or explicitly confirmed shape-independent.
5. `cargo test -p dpp --all-features` green; `cargo clippy --workspace --all-features -- -D warnings` clean.

## 3. Non-goals / explicit exclusions

- **Unit-only enums (8):** `OrderBy`, `IndexCountability`, `TokenConfigurationPresetFeatures`, `TokenDistributionType`, `TokenTradeMode`, `ContestedIndexResolution`, `StorageKeyRequirements`, `GroupActionStatus`. A unit enum serializes as a bare string `"VariantName"` — the string *is* the discriminator. Internal tagging would only make them `{"type":"…"}`, which is strictly worse. **No change.**
- **`PlatformAddress` — EXCLUDED.** Despite my initial audit flagging it, its **manual** `Serialize`/`Deserialize` (`address_funds/platform_address.rs:127,142`) emit a **compact 21-byte version-prefixed address** (`Value::Bytes`; the recent "21-byte guard" commit `c9f7f3d433`), not a `{"P2pkh":…}` object. The leading version byte already discriminates p2pkh/p2sh. Adding a `type` key would bloat an address into an object and break 64 downstream files. This is an intentional address encoding, not a JSON-object union. **Out of scope.**
- **Bincode / consensus wire** — unchanged by construction.
- **Enums outside the PR's changed files** — this sweep is scoped to PR #3573's diff.

## 4. Convention applied

| Decision | Rule |
|---|---|
| Tag mechanism | `#[serde(tag = "type")]` auto-derive when **every** variant is a struct or wraps a struct/map; **hand-written `Serialize`/`Deserialize`** when any variant wraps a primitive/tuple (serde cannot auto-internal-tag those). Precedents: `AddressWitness`, `ResourceVoteChoice`, `TokenEvent`. |
| Tag key | Plain **`type`** — none of the in-scope enums have `$`-prefixed fields at the variant wire level (no flattened `$formatVersion` neighbor), so `$type` does **not** apply (contrast `StoredAssetLockInfo`). |
| Variant value casing | `rename_all = "camelCase"` on the discriminator value (matches `AssetLockProof` "instant"/"chain"). |
| Payload field name (custom impls) | Single unnamed payload → **`value`**. Multi-field tuple → **named per field** (documented per enum below). |

## 5. Scope — inventory (8 enums, tiered)

### Tier A — clean auto-derive (`#[serde(tag = "type", rename_all = "camelCase")]`)

| Enum | File | Current shape | Proposed shape | Consumers | Risk |
|---|---|---|---|---|---|
| `RewardDistributionType` | `…/reward_distribution_type/mod.rs` | `{"BlockBasedDistribution":{"interval":N,"function":{…}}}` | `{"type":"blockBasedDistribution","interval":N,"function":{…}}` | 15 files (wasm-dpp2 heavy) | low |
| `DistributionFunction` | `…/distribution_function/mod.rs` | `{"FixedAmount":{"amount":N}}`, `{"Stepwise":{<map>}}` | `{"type":"fixedAmount","amount":N}`, `{"type":"stepwise",<map>}` | 16 files (wasm-dpp2 heavy) | **med** — `Stepwise(BTreeMap<u64,TokenAmount>)` is a newtype-of-map; internal tag must flatten cleanly with numeric-string keys coexisting with `"type"`. Verify round-trip. |

### Tier B — hand-written `Serialize`/`Deserialize` (primitive/tuple variants)

| Enum | File | Current shape | Proposed shape | Consumers | Notes |
|---|---|---|---|---|---|
| `RewardDistributionMoment` | `…/reward_distribution_moment/mod.rs` | `{"BlockBasedMoment":N}` | `{"type":"blockBasedMoment","value":N}` | 11 (rs-sdk, proof-verifier, js-evo-sdk) | Must preserve `json_safe_u64` (string above 2^53) in custom impl; Value path keeps `U64`/`U16`. |
| `TokenConfigurationChangeItem` | `…/token_configuration_item.rs` | `{"maxSupply":N}` (camelCase, custom `JsonSafeFields`) | `{"type":"maxSupply","value":N}` | 31 (wasm-dpp2 heavy) | Many variants; must preserve per-variant `json_safe` u64/u128. |
| `ContestedIndexFieldMatch` | `…/document_type/index/mod.rs` | `{"regex":"…"}`, `{"positiveIntegerMatch":N}` | `{"type":"regex","value":"…"}`, `{"type":"positiveIntegerMatch","value":N}` | 0 external | **Recently refactored** to external camelCase on this branch (custom impl already exists). Reopening a deliberate decision. |
| `TokenDistributionInfo` | `…/token_distribution_key.rs` | `{"PreProgrammed":[ts,id]}`, `{"Perpetual":[moment,recipient]}` | `{"type":"preProgrammed","timestamp":N,"identity":id}`, `{"type":"perpetual","moment":{…},"recipient":{…}}` | 0 external | Multi-field tuples → named fields. |
| `TokenDistributionTypeWithResolvedRecipient` | `…/token_distribution_key.rs` | tuple variants | `{"type":"…",<named fields>}` | 0 external | Multi-field tuples → named fields. |
| `ArrayItemType` | `…/document_type/property/array.rs` | mixed: bare `"Integer"`, `{"String":[min,max]}`, `{"ByteArray":[min,max]}` | `{"type":"integer"}`, `{"type":"string","minLength":N,"maxLength":N}`, … | 0 external | **Mixed unit + tuple variants.** Internal tag forces unit variants to `{"type":"…"}` too. This is a document-schema type — wire shape may have schema-validation meaning; needs domain check. |

## 6. Failure modes & mitigations

1. **serde `Content`-buffer drops typed bytes.** Internal tagging buffers the map into `serde::__private::Content`; `Value::Bytes`/`Bytes32` can degrade to `Array[U8]` on the way back. *Mitigation:* every enum gets a Value round-trip fixture asserting the typed-bytes variant survives (this is why `StoredAssetLockInfo` worked but had to be tested, and why `ValidatorSet`'s BLS value-test is `#[ignore]`d). Tier-A enums carry no bytes; Tier-B `TokenDistributionInfo` carries `Identifier`.
2. **`DistributionFunction::Stepwise` map+tag collision.** Numeric-string map keys coexisting with `"type"`. *Mitigation:* dedicated round-trip test; if it fails, fall back to a custom impl for that enum.
3. **`json_safe_u64`/`u128` loss in custom impls.** Hand-written `Serialize` must replicate the large-int-as-string HR behavior or JS precision breaks silently. *Mitigation:* port the `serde(with=…)` logic into the custom impl; add an above-2^53 fixture per affected enum (the `AssetLockValue` precedent).
4. **`ArrayItemType` mixed variants.** Custom impl must handle unit + tuple uniformly. *Mitigation:* explicit per-variant Serialize arms + round-trip across all 7 variants.
5. **Downstream wasm-dpp2 manual converters.** Several enums have hand-built JS-object converters (the `StoredAssetLockInfo`/`convert.rs` pattern) that may encode the old shape independently of serde. *Mitigation:* §7 consumer audit must enumerate and update each.
6. **Reopening recently-refactored enums** (`ContestedIndexFieldMatch`, `TokenConfigurationChangeItem`). *Mitigation:* confirm with the existing per-enum tests + note the deliberate-change history; don't churn unless the convention genuinely requires it.

## 7. Consensus & consumer safety

- **Consensus:** none of these derive bincode through serde for the wire; `platform_serialize`/bincode paths are independent (plan doc §"Tag-key conventions"). Reshaping serde JSON/Value does not touch state or proofs. **To be re-verified per enum** (grep for `Encode`/`Decode` + `platform_serialize` on each).
- **Consumers:** counts above are *all* Rust references, most of which only use the type, not its serde shape. The shape-sensitive consumers are: (a) `wasm-dpp2` manual `convert.rs`/DTO builders, (b) any `.ts`/`.js` that parses the JSON. **Open item:** a consumer agent must classify each of the 16/15/31/11 references as shape-dependent or not.

## 8. Test / verification plan

- Per enum: JSON round-trip + Value round-trip, **each with a static fixture** asserting the new flat `{"type":…}` shape (not just round-trip).
- Above-2^53 fixture for every enum carrying `json_safe` integers.
- All-variant coverage for multi-variant enums (the `_all_variants` loop pattern already in the PR).
- `cargo test -p dpp --all-features` + targeted wasm-dpp2 tests if converters change.
- Red→green discipline where a custom impl fixes a previously-wrong asymmetry.

## 9. Alternatives considered

1. **Leave single-payload enums external** (`{"maxSupply":N}`). *Pro:* arguably cleaner/more JS-friendly for one-field variants (`obj.maxSupply`). *Con:* violates the uniform "always `obj.type`" convention; variant discovery still needs key inspection. **Tension flagged for review** (§10 Q3).
2. **Adjacent tagging** (`{"type":"…","content":…}`). Rejected — the plan explicitly eliminated `content` wrappers.
3. **Do nothing / unit-only-correct only.** Rejected — leaves the convention half-applied, which is what review flagged.

## 10. Open questions for review panel

- **Q1 (scope):** Is `ArrayItemType` (a document-schema type) safe to reshape, or does its wire form feed JSON-Schema validation / contract storage where the external shape is load-bearing?
- **Q2 (scope):** Should `ContestedIndexFieldMatch` and `TokenConfigurationChangeItem` be reopened given they were *just* deliberately refactored to external camelCase on this same branch?
- **Q3 (convention):** For single-payload enums, is uniform internal `{"type":"x","value":N}` actually better than external `{"x":N}` for JS DX — or does the user's "convenient JSON" goal argue the opposite for these specifically?
- **Q4 (feasibility):** Does `DistributionFunction::Stepwise(BTreeMap)` survive auto-internal-tagging, or must it drop to a custom impl?
- **Q5 (correctness):** Best pattern to preserve `json_safe_u64`/`u128` inside hand-written `Serialize` without duplicating the helper logic 6×?
- **Q6 (blast radius):** Which of the wasm-dpp2 references (DistributionFunction 16, RewardDistributionType 15, TokenConfigurationChangeItem 31) actually depend on the serde wire shape vs. just the Rust type?
- **Q7 (sequencing):** Land Tier A (2 enums, low-risk) first as a standalone commit, then Tier B (6 enums) after the consumer audit — or all at once?
