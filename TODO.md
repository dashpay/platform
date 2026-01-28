# TODO: Add creationRestrictionMode=3 (Any group member) with group index

## Schema + config updates
- [x] Update `packages/rs-dpp/schema/meta_schemas/document/v0/document-meta.json`:
  - [x] Extend `creationRestrictionMode` enum to include `3` and update description.
  - [x] Add a new optional integer field `creationRestrictionGroup` for the group index with bounds matching `GroupContractPosition` (u16).
  - [x] Add conditional schema so mode `3` requires the group index.
- [x] Update any schema consumers / validators that hardcode allowed values:
  - [x] `packages/rs-json-schema-compatibility-validator/src/rules/rule_set.rs` to recognize the new field and allow the new enum value in examples.

## DPP: document type model + parsing
- [x] Extend `CreationRestrictionMode` in `packages/rs-dpp/src/data_contract/document_type/restricted_creation/mod.rs`:
  - [x] Add a `AnyGroupMember` variant with Display text.
  - [x] Update `TryFrom<u8>` to accept `3` and update `UnknownDocumentCreationRestrictionModeError` allowed values.
- [ ] Add a `creation_restriction_group` (likely `Option<GroupContractPosition>`) to `DocumentTypeV0` and `DocumentTypeV1` structs:
  - [ ] `packages/rs-dpp/src/data_contract/document_type/v0/mod.rs`
  - [ ] `packages/rs-dpp/src/data_contract/document_type/v1/mod.rs`
- [ ] Add accessors (getters/setters if needed) for the group index:
  - [ ] `packages/rs-dpp/src/data_contract/document_type/accessors/v0/mod.rs`
  - [ ] `packages/rs-dpp/src/data_contract/document_type/accessors/mod.rs`
  - [ ] `packages/rs-dpp/src/data_contract/document_type/v0/accessors.rs`
  - [ ] `packages/rs-dpp/src/data_contract/document_type/v1/accessors.rs`
- [ ] Parse and validate the group index in document type schema parsing:
  - [ ] `packages/rs-dpp/src/data_contract/document_type/class_methods/try_from_schema/v0/mod.rs`
  - [ ] `packages/rs-dpp/src/data_contract/document_type/class_methods/try_from_schema/v1/mod.rs`
  - [ ] Enforce: mode `3` requires group index; other modes should forbid/ignore it.
- [ ] Validate that referenced group positions exist on the data contract:
  - [ ] Add a DPP validation step (similar to token config group checks) that checks all document types with mode `3` reference a valid group position.
  - [ ] Decide on the consensus error (`GroupPositionDoesNotExistError` vs new error).
- [ ] Update random document type generators to include the new field with sane defaults:
  - [ ] `packages/rs-dpp/src/data_contract/document_type/v0/random_document_type.rs`
  - [ ] (If v1 has random generators) update accordingly.

## Drive/ABCI: creation validation
- [ ] Extend creation restriction validation to support mode `3`:
  - [ ] `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/batch/action_validation/document/document_create_transition_action/advanced_structure_v0/mod.rs`
  - [ ] Logic: fetch the configured group from the data contract, verify `owner_id` is a group member (use `GroupV0Getters::member_power` or equivalent).
  - [ ] Decide error behavior for non-members (likely `IdentityNotMemberOfGroupError`) and for missing/invalid group index (likely `InvalidGroupPositionError` or new error).
- [ ] Ensure any other validation paths that depend on `CreationRestrictionMode` handle the new variant.

## Tests + fixtures
- [ ] Add DPP unit tests:
  - [ ] New enum value parses correctly; unknown mode errors include `3` in allowed list.
  - [ ] Schema parsing requires group index for mode `3` and rejects missing/invalid group index.
  - [ ] Data contract validation rejects references to non-existent group positions.
- [ ] Add Drive ABCI tests for document creation:
  - [ ] Success when creator is a member of the configured group.
  - [ ] Failure with correct consensus error when creator is not in the group.
  - [ ] Failure if document type references a missing group position.
  - [ ] Update/extend existing creation restriction tests in `packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/batch/tests/document/creation.rs`.
- [ ] Add/extend fixtures:
  - [ ] New contract JSON with groups + document type using `creationRestrictionMode: 3` and the group index.
  - [ ] Place in `packages/rs-drive-abci/tests/supporting_files/contract/` (or reuse `packages/wasm-sdk/tests/unit/fixtures/data-contract-v1-with-docs-tokens-groups.mjs` if appropriate).

## Optional cross-SDK alignment (if needed)
- [ ] Update any SDK/UI code that assumes only 0/1/2 values (Swift example app, wasm fixtures) to recognize mode `3` and display/handle it correctly.
