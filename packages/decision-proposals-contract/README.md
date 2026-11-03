# Decision Proposals Contract

The decision proposals system contract holds yes/no questions that anyone puts
to the masternodes. It activates at protocol version 15, registered at genesis
by chains born at 15 and inserted by the upgrade to 15, and has the same ID on
every network: `3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF`.

Creating a proposal opens a masternode yes/no vote poll on it. The poll is
signaling only: nothing on Platform acts on its result, which stays readable
with `getYesNoVotePollState`.

## `proposal`

The only document type. Any identity may create one, and nobody may delete it.
Its rule (the threshold, the minimum and the end date) never changes, so
masternodes always vote by the rule the poll opened with. Its owner may edit the
`title`, `description` and `url` for a day after creating it, or for a fifth of
the time it runs if that is shorter; after that they freeze too. Every version
is kept (`documentsKeepHistory`), so a voter can see the text as it was when it
voted.

| Property | Type | Meaning |
| --- | --- | --- |
| `title` | string, 1 to 128 characters and at most 512 bytes, required | The question, in one line |
| `description` | string, 1 to 4096 characters and at most 4096 bytes, required | What a yes and a no mean |
| `url` | string, a URI of 1 to 512 characters and at most 512 bytes | Where the proposal is discussed |
| `passPercent` | integer 50 to 80, required | The proposal passes when yes is strictly more than this percent of yes plus no |
| `minimumTurnoutPercent` | integer 0 to 50, required | The least yes plus no voting power the proposal needs, as a percent of the masternode list's total voting power when the poll closes, rounded up (a masternode votes with 1, an evonode with 4) |
| `endDate` | integer, milliseconds since the Unix epoch, required | When voting closes |

Additional properties are rejected.

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

- `passPercent` is 50 to 80: at 50, yes must be strictly more than no;
- `minimumTurnoutPercent` is at most 50;
- the end date is one to four weeks after the block that opens the poll.

See [the protocol guide](../../docs/protocol/decision-proposals.md) for
details.

## Install

```sh
npm install @dashevo/decision-proposals-contract
```

## License

[MIT](LICENSE) © Dash Core Group, Inc.
