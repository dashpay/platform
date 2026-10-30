# Decision Proposals Contract

The decision proposals system contract holds yes/no questions that anyone puts
to the masternodes. It activates at protocol version 15, registered at genesis
by chains born at 15 and inserted by the upgrade to 15, and has the same ID on
every network: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`.

Creating a proposal opens a masternode yes/no vote poll on it. The poll is
signaling only: nothing on Platform acts on its result, which stays readable
with `getYesNoVotePollState`.

## `proposal`

The only document type. Any identity may create one. It is immutable and
undeletable, so masternodes vote on what the proposal said when it opened.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The question, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What a yes and a no mean |
| `url` | string, a URI of 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `supermajorityNumerator` | integer 1 to 255, required | The proposal passes when yes is more than `supermajorityNumerator / supermajorityDenominator` of yes plus no |
| `supermajorityDenominator` | integer 1 to 255, required | |
| `minimumVotingPower` | integer 0 to 4294967295 | The least yes plus no voting power the proposal needs (a masternode votes with 1, an evonode with 4) |
| `minimumShare` | object | The least yes plus no voting power the proposal needs, as a share of the masternode list's total when the poll closes: `numerator` (0 to 255), `denominator` (1 to 255) and `rounding`, one of `down`, `up`, `downPlusOne` and `upMinusOne`, all three required |
| `endDate` | integer, milliseconds since the Unix epoch, required | When voting closes |

A proposal sets `minimumVotingPower` or `minimumShare`. Additional properties
are rejected, in the proposal and in `minimumShare`.

Indexes: `byEndDate` (`endDate`, `$createdAt`) lists the proposals closing
soonest first; `byOwner` (`$ownerId`, `$createdAt`) lists an identity's
proposals; `byCreatedAt` (`$createdAt`) lists the newest.

## The poll

At protocol version 15, a `proposal` create opens a yes/no vote poll for the
proposal and prefunds the masternode votes on it with 0.5 Dash from the
owner's balance (`decision_proposal_fund_required_amount`). What the votes
leave is released as processing fees when the poll ends.

The poll's rule is read from the proposal and must be within the protocol's
limits, or the create is refused:

- the threshold is at least a simple majority: `supermajorityNumerator /
  supermajorityDenominator` is at least one half and below one;
- the minimum is at most 2,000 voting power, or at most 50% of the network's
  total voting power;
- the end date is one to four weeks after the block that opens the poll.

See [the protocol guide](../../docs/protocol/decision-proposals.md) for
details.

## Install

```sh
npm install @dashevo/decision-proposals-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
