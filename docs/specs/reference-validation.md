# Reference Validation (`refersTo`)

## Summary
Introduce an optional `refersTo` keyword on document properties so contracts can request existence checks for referenced identities. When present, state validation rejects documents whose referenced identity does not exist. Contracts and documents without `refersTo` are unchanged.

## Schema Changes
- Extend document meta-schema (`packages/rs-dpp/schema/meta_schemas/document/v0/document-meta.json`) to allow a `refersTo` object alongside existing keywords.
- `refersTo` structure:
  - `type`: `"identity"` (current target; keep the mechanism extensible for other reference targets such as documents or contracts).
  - `mustExist`: `boolean` (optional, defaults to `true`; `false` means no reference validation).
- Validation rules during schema parsing:
  - Only allowed on identifier-typed properties (array byteArray=true, minItems=32, maxItems=32, `contentMediaType` identifier).
  - Not allowed on non-identifier properties (reject contract).
- Mutability: `refersTo` may only be set at contract creation; contract updates cannot add or change it.
- JSON Schema compatibility rules must allow the keyword but should reject updates that attempt to add/modify it post-creation.

## Data Model / Parsing
- During `DocumentType::try_from_schema`, detect `refersTo` and store per-property reference metadata (path → `{ targetType: Identity, mustExist: bool }`).
- Expose reference metadata through DocumentType accessors and WASM/JS bindings so clients can introspect.
- Keep existing `identifier_paths`/`binary_paths` behavior (the sets of property paths already tracked for identifier and binary fields); `refersTo` is additive on top.

## Runtime Validation
- Enforce during Drive document state validation (create/replace state validators) for document create and replace transitions:
  - For each property with `refersTo.mustExist == true`, fetch the referenced identity ID and fail with a consensus state error if missing.
  - Support nested properties (use flattened property paths).
  - Count identity fetches in execution context fee accounting.
- Implement via versioned document state validators (new v2 modules) while keeping v0/v1 behavior unchanged.
- Applied in ABCI paths: CheckTx, PrepareProposal, and ProcessProposal.
- Basic validation (DPP) only checks keyword shape/placement; no state access.

## Errors
- Add a dedicated consensus state error, e.g., `ReferencedIdentityNotFoundError { path, identityId }`.
- Avoid overloading signature errors; ensure deterministic mapping to codes.

## Backward Compatibility
- Gated by platform/protocol version (and/or data contract system version). Legacy nodes reject contracts containing `refersTo`; such contracts are accepted only after activation. Post-activation, newer nodes enforce `mustExist:true` semantics.
- Existing pre-activation contracts and documents remain valid; documents are rejected only when the contract opts in with `mustExist:true` and the network is past activation.

## Implementation Notes
- Reference existence checks use the identity revision lookup (`fetch_identity_revision`) as the minimal-cost existence check.
- Reference validation dispatches through a versioned `DocumentReferenceValidation` trait; v0 rules are implemented for identity references.

## Acceptance Criteria
- Contracts containing `refersTo` validate against updated meta-schema and pass compatibility checks when added to existing identifier fields.
- Documents with `refersTo.type=identity` + `mustExist:true` are accepted only if the referenced identities exist; missing ones return the new consensus error.
- Documents without `refersTo`, or with `mustExist:false`, behave exactly as today.
- Enforcement applies to create and replace transitions (including nested fields) with proper fee accounting for identity lookups.
- WASM/JS bindings serialize/deserialize `refersTo` metadata; tests cover parsing and state validation failures.
