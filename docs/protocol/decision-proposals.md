# Decision proposals

A decision proposal is a yes/no question that any identity puts to the
masternodes. Creating one opens a masternode yes/no vote poll on it, which the
proposal's owner funds. The result is signaling only: nothing on Platform acts
on it.

The proposals live in the decision proposals system contract. It activates at
protocol version 15: a chain born at 15 registers it at genesis
(`create_genesis_state` v1, behind a `>= 15` version branch), an older chain
inserts it on the upgrade to 15 (`transition_to_version_15`), and the Drive
system contract cache and the trusted context provider serve it from 15 on.

- Contract ID: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`
- Owner: the all-zero system identity
- Registry entry: `SystemDataContract::DecisionProposals = 11`
- Schema version: 1
- Document types: `proposal`

## `proposal`

The type is immutable (`documentsMutable: false`) and undeletable
(`canBeDeleted: false`), so masternodes vote on what the proposal said when it
opened, and the proposal stays for as long as its result is read. Additional
properties are rejected.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes (`maxBytes`), required | The question, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What a yes and a no mean |
| `url` | string, `format: uri`, 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `supermajorityNumerator` | integer 1 to 255, required | With the denominator, the share of yes plus no that yes must exceed |
| `supermajorityDenominator` | integer 1 to 255, required | |
| `minimumVotingPower` | integer 0 to 4294967295 | A fixed minimum: the least yes plus no voting power the proposal needs |
| `minimumShare` | object of `numerator` (0 to 255), `denominator` (1 to 255) and `rounding`, all required | A minimum as a share of the masternode list's total voting power in the block that closes the poll, PoSe-banned masternodes left out |
| `endDate` | integer, milliseconds since the Unix epoch, required | When voting closes |

A proposal sets `minimumVotingPower` or `minimumShare`. `rounding` says how
the share rounds to a whole voting power: `down` (1/3 of 10 is 3), `up` (1/3 of
10 is 4), `downPlusOne` (1/3 of 9 is 4; half the total this way is a strict
majority of it) or `upMinusOne` (1/3 of 9 is 2, never below zero).

Indexes:

| Index | Properties | Lists |
| --- | --- | --- |
| `byEndDate` | `endDate`, `$createdAt` | The proposals closing soonest first |
| `byOwner` | `$ownerId`, `$createdAt` | An identity's proposals, in filing order |
| `byCreatedAt` | `$createdAt` | The newest proposals |

None is unique: anyone may ask the same question again.

## Opening the poll

A `proposal` create opens a yes/no vote poll in the same batch. The poll's
purpose is `YesNoVotePollPurpose::DecisionProposal { document_id }`, the
proposal's document id, and its id is derived from that purpose alone, so a
proposal has exactly one poll. Its rule is read from the proposal:

- it passes when the yes voting power is strictly more than
  `supermajorityNumerator / supermajorityDenominator` of the yes plus no voting
  power, abstentions left out of both sides,
- and the yes plus no voting power reaches the minimum.

A masternode votes with 1 and an evonode with 4. The rule is written to the
poll's stored info when it opens and cannot change after that; a vote names the
poll by id alone.

Opening the poll prefunds the masternode votes on it: 0.5 Dash
(`decision_proposal_fund_required_amount` in `VoteResolutionFundFees`, set in
`FEE_VERSION4`, protocol version 15) leaves the owner's balance for the poll's
prefunded balance. Each vote takes a fixed cost from it (0.00002 Dash), so the
fund pays for 25,000 votes, more than every masternode casting all of its votes
on it. What the votes leave is released as processing fees when the poll ends.

## Limits

The create is refused unless the rule is within these limits, which
`SystemLimits` carries:

| Limit | Value | Field |
| --- | --- | --- |
| Threshold | at least a simple majority: the fraction is at least 1/2, and below 1 | `min_yes_no_vote_poll_supermajority_percent` (50) |
| Fixed minimum | at most 2,000 voting power | `max_yes_no_vote_poll_minimum_voting_power` |
| Share minimum | at most 50% of the network's total voting power | `max_yes_no_vote_poll_minimum_voting_power_percent_of_total` |
| End date | one to four weeks after the block that opens the poll | `min_yes_no_vote_poll_duration_ms`, `max_yes_no_vote_poll_duration_ms` |

A threshold of at least a simple majority means a proposal cannot pass with
fewer yes than no. The one-week floor gives masternode operators time to see a
proposal before it closes.

## Reading the result

The poll closes in a block whose time has reached its end date. Its result,
whether it passed with the yes, no and abstain voting power cast and the
voting power it needed, stays in the poll's stored info. Read it with
`getYesNoVotePollState`, naming the poll by its id (`YesNoVotePoll::unique_id`
of the `DecisionProposal` purpose): the response carries the poll as it was
opened, its rule included, and once it has finished, the result and the blocks
it opened and closed in.

The result is signaling only. Nothing on Platform acts on a passed or failed
proposal (`on_yes_no_vote_poll_finished` does nothing for this purpose); what
it means is up to the people who read it.
