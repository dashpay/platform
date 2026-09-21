# Vote Polls

Masternodes and evonodes decide some things on Platform by voting: a masternode's vote counts
once, an evonode's four times. Every vote is a `MasternodeVote` state transition carrying a
`VotePoll`, the thing being decided, and a `ResourceVoteChoice`. There are two kinds of poll.

## Contested document resource polls

A `ContestedDocumentResourceVotePoll` decides which of several identities gets a contested
unique index value, such as a premium DPNS name. It names the contract, the document type, the
index and the index values. Contenders are the documents competing for the value; the choices
are a contender, abstain, or lock, which gives the value to nobody. The poll ends by plurality
and a tie goes to the **latest** contender. The poll is funded by the contenders' prefunded
voting balances and each vote costs a fixed amount from that balance.

Its state lives under `votes / contested_resource / active_polls`, laid out like the contested
index it decides, and the masternodes' vote references under
`votes / contested_resource / identity_votes`.

## Identity contender polls

An `IdentityContenderVotePoll` (protocol version 14) elects one identity among contenders over
a resource path the opener chooses: a list of byte strings, such as a contract id followed by
what the election is for when a moderation team is elected. Byte strings rather than typed
values, so a poll rebuilt from JSON hashes to the same id as the one in state. It has no lock
choice: a vote goes to a contender or abstains.

The poll runs in two phases with explicit end times:

1. **Join phase.** Contenders join. Nobody votes. When the join end time arrives, a poll with a
   single contender resolves at once with that contender as the winner, a poll with no
   contender resolves with no winner, and a poll with several contenders moves to the vote
   phase.
2. **Vote phase.** Masternodes vote until the vote end time. The contender with the highest
   tally wins by plurality, with no minimum. A tie goes to the **earliest** contender: the
   lowest block time, then the lowest block height, then the lowest reference id, the id of
   what made the identity a contender. With no votes at all every contender ties, so the first
   contender wins.

Votes on an identity contender poll cost the same as votes on a contested document resource
poll, paid from the poll's prefunded specialized balance, whose id is the poll's unique id.
When the poll resolves the balance's remainder goes to the processing pool.

A poll is keyed in state by its unique id, the double sha256 of the serialized poll. Its state
lives under `votes / identity_contender_polls / active_polls / <poll id>`: a stored info item
with the phase, the two end times and, once resolved, the result; an abstain tree; and one tree
per contender holding the contender's record and the votes towards it. The masternodes' vote
references live under `votes / identity_contender_polls / identity_votes`. The branch is
created by the first poll rather than at genesis. When a poll resolves its contenders, votes
and references are removed and the stored info stays with the result, which
`getIdentityContenderVotePollState` serves with a proof.

Opening a poll and adding contenders are Drive operations (`open_identity_contender_vote_poll`
and `add_identity_contender`); the state transitions that use them are the moderation team
election and its challenges.

### Consensus errors

| Code | Error | When |
|------|-------|------|
| 40301 | `VotePollNotFoundError` | The poll never opened. |
| 40307 | `VoteChoiceNotAllowedForVotePollError` | A lock vote, or a vote towards an identity that is not a contender. |
| 40308 | `IdentityContenderVotePollNotAvailableForVotingError` | A vote during the join phase or after the poll resolved. |
