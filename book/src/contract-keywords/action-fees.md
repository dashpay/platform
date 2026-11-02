# Action Fees (actionFees)

`actionFees` charges a fixed fee in credits, on top of the gas, for an action on a document of the type. Each fee has two parts: one for the contract owner and one for the contract's moderators, each collected in a pot that its recipients claim. Reach for it when an app wants to earn from the documents written under it, or to pay the people who moderate it. The transition that pays must state the fee it agrees to, so a fee can never surprise a signer.

| | |
|---|---|
| **Where** | Document type |
| **Value** | An object with an optional `pricing`, and one or more of the actions `create`, `replace`, `delete`, `transfer`, `update_price`, `purchase`, each an object with `owner` and/or `moderators` (below) |
| **Default** | Absent: no action charges a fee |
| **Since** | protocol version 14 |
| **On update** | Fixed (`DocumentTypeUpdateError`, 40212): an update may not add, change or remove the fees of an existing document type, nor switch their pricing. A document type the update adds may declare its own |
| **Errors** | On a document transition: `DocumentActionFeeAgreementNotSetError` (40132), `DocumentActionFeeAgreementMismatchError` (40133), `DocumentActionFeeMultiplierNotToleratedError` (40134), `DocumentActionFeeModeratorsShareMismatchError` (40139). At registration: `DocumentActionFeesWithoutModerationError` (10902), `JsonSchemaError` (10101), `InvalidContractStructure` (10231) |

The keys:

| Key | Value | Default | Meaning |
|---|---|---|---|
| `pricing` | `"feeMultiplier"` or `"fixed"` | `"feeMultiplier"` | Whether the amounts follow the network's fee multiplier or are charged as written |
| `<action>.owner` | credits, 0 to 9223372036854775807 | 0 | Added to the contract's owner pot, which the contract owner claims |
| `<action>.moderators` | credits, 0 to 9223372036854775807 | 0 | Added to the contract's moderators pot, which the moderation team shares. Needs `moderation` in the contract config |

Amounts are in credits: 1 Dash is 100,000,000,000 credits (1000 credits per duff).

## Example

```json
"post": {
  "type": "object",
  "actionFees": {
    "pricing": "feeMultiplier",
    "create": { "moderators": 100000000, "owner": 10000000 }
  },
  "properties": {
    "text": { "type": "string", "minLength": 1, "maxLength": 280, "maxBytes": 560, "position": 0 }
  },
  "required": ["text"],
  "additionalProperties": false
}
```

In a contract that declares moderation, creating a post costs an extra 0.001 Dash for the moderation team and 0.0001 Dash for the contract owner, at a fee multiplier of 1. Replacing, deleting and every other action cost only their gas.

## How it works

- **What is charged.** With `fixed` pricing, the declared amounts. With `feeMultiplier`, the declared amounts scaled by the fee multiplier of the epoch the action executes in (`declared * multiplier_permille / 1000`, rounded down), so a fee follows the network's fees. A scaled amount is held at the maximum number of credits instead of overflowing; such a fee refuses the action for an insufficient balance.
- **Who pays.** Whoever pays the gas pays the fee: the signer, or the contract owner when they pay the gas of a token-paid action (see [Token Costs](token-cost.md#who-pays-the-gas-gasfeespaidby)). The contract owner never pays the `owner` part, which would only travel through their pot back to them: a contract owner who pays, as the signer or as the gas sponsor, pays the `moderators` part only. A contract that sponsors gas should price that in: a sponsor's balance must cover the gas and those fees, or the transition falls back to the signer or is refused, as the token cost's rules say.
- **Only executed actions pay.** A transition that fails, at any stage, owes no fee.
- **Where it goes.** One removal from the payer's balance, and one addition to each pot the fee has a part for. The fee is not part of the gas: the fee pools and block proposers get none of it.

## The agreement: `$actionFeeAgreement`

The contract is read when the transition executes, not when it was signed. So every transition on an action that charges a fee carries an action fee agreement in its base, naming the fee its signer saw (version 2 of the document base transition, the default from protocol version 14):

```json
"$actionFeeAgreement": {
  "$formatVersion": "0",
  "owner": 10000000,
  "moderators": 100000000,
  "feeMultiplier": { "knownPermille": 1000, "increaseTolerancePercent": 20 }
}
```

| Field | Meaning |
|---|---|
| `owner`, `moderators` | The amounts the document type declares for the action, before any multiplier. They must match exactly, each part on its own |
| `feeMultiplier` | Present for a `feeMultiplier` fee, left out for a `fixed` one. `knownPermille` is the fee multiplier the signer priced the fee with, in thousandths (1000 is 1x). `increaseTolerancePercent` is how far above it the executing epoch's multiplier may be, in percent of the known one: 20 accepts up to 1.2 times |

A transition on an action that charges a fee is refused:

- without an agreement (`DocumentActionFeeAgreementNotSetError`, 40132), whoever pays, a sponsored transition included;
- with other amounts, parts moved between the pots, or the other pricing (`DocumentActionFeeAgreementMismatchError`, 40133). The signer reads the contract again;
- when the executing epoch's multiplier is above what the agreement tolerates (`DocumentActionFeeMultiplierNotToleratedError`, 40134). A multiplier that fell is always accepted, and what is charged follows the epoch's multiplier, never the known one.

Each refusal bumps the signer's nonce, and no action fee is charged. The mempool applies the same checks on arrival and on every recheck, so a transition whose agreement no longer holds leaves the mempool with the same error. An agreement on an action that charges nothing is ignored.

A client should build the agreement from the contract it showed its user, never from a contract fetched behind their back at signing time. In Rust, `DocumentActionFeeAgreement::for_document_type_action` builds it from a document type, and the SDK's document transition builders take it with `with_action_fee_agreement`.

**A seated team's discount.** On a document type that an elected contract moderates, the `moderators` part of an agreement may name less than the declared amount: exactly the share the contract's seated moderation charter takes (its `moderatorsShare`, in percent, rounded down to the credit). Everything else must still match. The action is then charged the agreed amount. Any other amount below the declared one, including a discount on a contract with no seated charter yet, is refused (`DocumentActionFeeModeratorsShareMismatchError`, 40139). A lower amount anywhere else, on a type the contract does not moderate or a contract that is not elected, is the plain mismatch (40133). See [Elected Moderation](../data-model/contract-moderation.md#elected-moderation).

## The pots and the claim

The `owner` parts collect in the contract's **owner pot** and the `moderators` parts in its **moderators pot**. A `ContractFeeClaim` state transition pays a pot out:

- the owner pot goes whole to the contract owner, the only identity that may claim it;
- the moderators pot is split equally between the moderation team (the identities the contract appoints, or the owner alone when it appoints none), and any member of the team may claim it for all of them. A seated elected team splits it by its charter's reward split instead. What a split leaves over, less than a credit per member, stays in the pot;
- each pot is paid out at most once per epoch, and the two are independent.

A claim is refused when the signer is not a recipient of the pot (`ContractFeeClaimNotAllowedError`, 41113), when the pot was already paid out this epoch (`ContractFeesAlreadyClaimedThisEpochError`, 41111), or when a recipient would get less than a credit (`ContractFeesNothingToClaimError`, 41112). The JavaScript SDK reads the pots with `contracts.feePots` and claims with `contracts.claimFees`.

## Rules at registration

- The meta-schema checks the shape (`JsonSchemaError`, 10101): only `pricing` and the six action keys; `pricing` one of the two values; each action an object with `owner`, `moderators` or both, each an integer from 0 to 9223372036854775807.
- At least one action is priced, and a priced action charges something: an action whose parts are all 0 is refused, leave it out instead. The two parts of an action may not add up to more than 9223372036854775807 credits (`InvalidContractStructure`, 10231).
- A nonzero `moderators` part needs a contract whose config declares `moderation` (`DocumentActionFeesWithoutModerationError`, 10902): the moderation team is who that pot is for. This is checked when the contract is created and when it is updated.

## See also

- [Document action fees](../fees/overview.md#document-action-fees), the deep dive
- [Fee Pots and the Claim](../data-model/contract-moderation.md#fee-pots-and-the-claim), for the pots, the claim and its proofs
- [Token Costs (tokenCost)](token-cost.md), which prices the same six actions in tokens
- [Contract-Level Keys and config](contract-config.md), for `moderation`
- [Contract Keywords overview](../contract-keywords.md), for the conventions of these tables
