# Return Proofs

After a state transition is confirmed in a block, clients can request a **return proof** —
a GroveDB Merkle proof demonstrating that the expected state changes were actually applied.
This lets light clients verify execution without trusting the node.

## How It Works

The flow is:

1. Client broadcasts a state transition via DAPI.
2. Client calls `waitForStateTransitionResult` with `prove: true`.
3. The node waits for the transition to be included in a block.
4. Drive deserializes the transition and calls `prove_state_transition()`.
5. This builds a `PathQuery` describing which GroveDB paths/keys the transition affected.
6. GroveDB generates a Merkle proof covering exactly those paths.
7. The proof is returned to the client in the response.

The client then calls `verify_state_transition_was_executed_with_proof()` to check the
proof against the known app hash (root hash). If verification succeeds, the client receives
a `StateTransitionProofResult` containing the verified data.

## Key Design Decisions

- **Minimal proofs.** Only the paths/keys affected by the transition are included, not the
  entire state tree. This keeps proofs small.
- **Type-specific.** Each transition type proves different data — an identity create proves
  the full identity, while a top-up only proves the new balance.
- **On-demand.** Proofs are generated after confirmation, not during validation.
- **Batch limitation.** Batch transitions (documents/tokens) currently support proofs only
  for single-transition batches.
- **Limits removed.** All `PathQuery` limits are set to `None` before proof generation to
  ensure the full result set is included.

## What Each Transition Proves

### Identity Transitions

| Transition | What's Proved | Verified Result |
|---|---|---|
| **IdentityCreate** | Full identity: data, balance, nonce, all public keys | `VerifiedIdentity(Identity)` |
| **IdentityTopUp** | Balance and revision only | `VerifiedPartialIdentity { balance, revision }` |
| **IdentityCreditWithdrawal** | Balance only | `VerifiedPartialIdentity { balance }` |
| **IdentityUpdate** | All public keys | `VerifiedPartialIdentity { loaded_public_keys }` |
| **IdentityCreditTransfer** | Sender balance + recipient balance | `VerifiedBalanceTransfer(sender, recipient)` |

The proof generation uses these Drive query helpers:

- `Drive::full_identity_query()` — identity tree + balance + nonce + all key subtree
- `Drive::revision_and_balance_path_query()` — just balance and revision elements
- `Drive::identity_balance_query()` — just the balance element
- `Drive::identity_all_keys_query()` — identity key subtree

For **IdentityCreditTransfer**, the sender and recipient balance queries are merged into
a single `PathQuery` via `PathQuery::merge()`.

### Identity + Address Transitions

| Transition | What's Proved | Verified Result |
|---|---|---|
| **IdentityCreditTransferToAddresses** | Identity balance/revision + recipient address balances | `VerifiedIdentityWithAddressInfos` |
| **IdentityCreateFromAddresses** | Full identity + all input/output address balances | `VerifiedIdentityFullWithAddressInfos` |
| **IdentityTopUpFromAddresses** | Identity balance/revision + input/output address balances | `VerifiedIdentityWithAddressInfos` |

These combine `Drive::revision_and_balance_path_query()` (or `full_identity_query()` for
create) with `Drive::balances_for_clear_addresses_query()`, merged into a single proof.

### Address Fund Transitions

| Transition | What's Proved | Verified Result |
|---|---|---|
| **AddressFundsTransfer** | All input + output address balances | `VerifiedAddressInfos` |
| **AddressFundingFromAssetLock** | All input + output address balances | `VerifiedAddressInfos` |
| **AddressCreditWithdrawal** | Input addresses + output address balance | `VerifiedAddressInfos` |

All use `Drive::balances_for_clear_addresses_query()`. Each address entry in the proof
contains its nonce and credit balance, allowing the client to verify post-transition
balances.

### Data Contract Transitions

| Transition | What's Proved | Verified Result |
|---|---|---|
| **DataContractCreate** | The contract itself | `VerifiedDataContract(DataContract)` |
| **DataContractUpdate** | The updated contract | `VerifiedDataContract(DataContract)` |

The query depends on whether the contract keeps history:
- Historical: `Drive::fetch_historical_contracts_query()`
- Non-historical: `Drive::fetch_non_historical_contracts_query()`

Verification reconstructs the contract from the proof and compares it field-by-field
against the state transition's contract data via `first_mismatch()`.

### Document Transitions (via Batch)

| Operation | What's Proved | Verified Result |
|---|---|---|
| **Create** | The created document | `VerifiedDocuments({ id: Some(doc) })` |
| **Replace** | The replaced document | `VerifiedDocuments({ id: Some(doc) })` |
| **Delete** | Absence of the document | `VerifiedDocuments({ id: None })` |
| **Transfer** | The document (with new owner) | `VerifiedDocuments({ id: Some(doc) })` |
| **UpdatePrice** | The document (with new price) | `VerifiedDocuments({ id: Some(doc) })` |
| **Purchase** | The document (with new owner) | `VerifiedDocuments({ id: Some(doc) })` |

All document operations use `SingleDocumentDriveQuery` to construct the path query.
For creates with prefunded voting balances, the query uses `Contested` status to look
up the document in the contested index tree instead of the regular document tree.

Verification checks:
- **Create/Replace:** Reconstructs the expected document from the transition and compares
  fields (ignoring time-based fields and transient fields).
- **Delete:** Asserts the document is absent from the proof.
- **Transfer/Purchase:** Verifies the document's `owner_id` matches the expected recipient.
- **UpdatePrice:** Verifies the document's `price` field matches the transition's price.

### Token Transitions (via Batch)

Token proof behavior depends on whether the token keeps historical documents for that
operation type. When history is enabled, the proof contains a historical document in the
token history contract. When disabled, the proof contains the raw state (balance, info, etc.).

| Operation | History Off | History On |
|---|---|---|
| **Mint** | Recipient token balance | Historical mint document |
| **Burn** | Owner token balance | Historical burn document |
| **Transfer** | Sender + recipient balances | Historical transfer document |
| **Freeze** | Frozen identity's token info | Historical freeze document |
| **Unfreeze** | Unfrozen identity's token info | Historical unfreeze document |
| **DirectPurchase** | Purchaser token balance | Historical purchase document |
| **SetPriceForDirectPurchase** | Token pricing schedule | Historical pricing document |
| **DestroyFrozenFunds** | Always historical document | — |
| **EmergencyAction** | Always historical document | — |
| **ConfigUpdate** | Always historical document | — |
| **Claim** | Always historical document | — |

**Group actions** add an extra layer: when a token transition uses group consensus
(multi-sig), the proof also includes the group action's signer and total power, plus the
action status (active vs closed). The verified result becomes one of the
`VerifiedTokenGroupAction*` variants.

### Masternode Vote

| Transition | What's Proved | Verified Result |
|---|---|---|
| **MasternodeVote** | The vote poll state for the specific vote | `VerifiedMasternodeVote(Vote)` |

Uses `IdentityBasedVoteDriveQuery` to construct the path query from the voter's ProTxHash
and the resource vote poll. Verification checks the vote exists and matches expectations.

### Shielded Transitions

| Transition | Proof Generation | Proof Verification |
|---|---|---|
| **Shield** | Not yet supported | Verifies input address balances (`VerifiedAddressInfos`) |
| **Unshield** | Not yet supported | Verifies output address balance (`VerifiedAddressInfos`) |
| **ShieldedTransfer** | Not yet supported | Verifies shielded pool total balance (`VerifiedShieldedPoolState`) |
| **ShieldFromAssetLock** | Not yet supported | Verifies shielded pool total balance (`VerifiedShieldedPoolState`) |
| **ShieldedWithdrawal** | Not yet supported | Verifies shielded pool total balance (`VerifiedShieldedPoolState`) |

Proof generation currently returns an error for all shielded transitions. The verification
side has been implemented in anticipation:

- **Shield** verifies the input platform address balances were debited.
- **Unshield** verifies the output platform address balance was credited.
- **ShieldedTransfer, ShieldFromAssetLock, ShieldedWithdrawal** verify the shielded credit
  pool's `total_balance` SumItem, confirming the pool balance changed as expected.

Note that shielded proofs intentionally do **not** reveal which notes were created or spent
(that would break privacy). Only aggregate pool state or transparent address balances are
provable.

## Code Locations

| Component | Path |
|---|---|
| Proof generation | `rs-drive/src/prove/prove_state_transition/v0/mod.rs` |
| Proof verification | `rs-drive/src/verify/state_transition/verify_state_transition_was_executed_with_proof/v0/mod.rs` |
| Proof result enum | `rs-dpp/src/state_transition/proof_result.rs` |
| DAPI wait service | `rs-dapi/src/services/platform_service/wait_for_state_transition_result.rs` |
| ABCI query handler | `rs-drive-abci/src/query/proofs/v0/mod.rs` |
| Shielded pool verify | `rs-drive/src/verify/shielded/verify_shielded_pool_state/v0/mod.rs` |
| Address balance verify | `rs-drive/src/verify/address_funds/verify_addresses_infos/v0/mod.rs` |
