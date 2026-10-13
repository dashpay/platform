# Contested Indexes

A unique index normally works first come, first served: the first document to take a value keeps it, and every later one is refused as a duplicate. `contested` changes that for the values a contract considers valuable. A document whose values fall in the contested range opens a **contest**, or joins one already open for the same values, and masternodes and evonodes vote on which identity gets them. DPNS uses it so that a short name such as `alice.dash` cannot simply be taken by whoever submits first. A contract author reaches for it when a unique value is scarce and should be awarded rather than raced for.

| | |
|---|---|
| **Where** | a unique index |
| **Value** | object: `resolution` (required), `fieldMatches`, `description` |
| **Since** | protocol version 1; `resolution: 1` from protocol version 14 |
| **On update** | Fixed, like every index (`DataContractInvalidIndexDefinitionUpdateError`, 10217) |
| **Errors** | `DocumentContestNotPaidForError` (40114), `DocumentContestCurrentlyLockedError` (40110), `DocumentContestNotJoinableError` (40111), `DocumentContestIdentityAlreadyContestantError` (40112), `DocumentContestIndexMismatchError` (40118), `DocumentContestNotRequiredError` (40119), `DocumentContestMaximumContendersReachedError` (40141); `DuplicateUniqueIndexError` (40105) for values outside the contested range |

## Example

The `domain` document type of the DPNS contract:

```json
{
  "name": "parentNameAndLabel",
  "properties": [
    { "normalizedParentDomainName": "asc" },
    { "normalizedLabel": "asc" }
  ],
  "unique": true,
  "contested": {
    "fieldMatches": [
      { "field": "normalizedLabel", "regexPattern": "^[a-zA-Z01-]{3,19}$" }
    ],
    "resolution": 0,
    "description": "If the normalized label part of this index is less than 20 characters (all alphabet a-z, A-Z, 0, 1, and -) then a masternode vote contest takes place to give out the name"
  }
}
```

A name is unique under its parent domain. A label of 3 to 19 characters made of letters, `0`, `1` and `-` is contested: registering it opens a masternode vote, which may give it to a contender or lock it. Any other label, a longer one or one with other digits, is registered first come, first served, and a second registration of it is a duplicate.

## The keys

| Key | Value | What it does |
|---|---|---|
| `resolution` | `0` or `1`, required | How the contest is decided. `0`: masternodes vote for a contender, abstain, or **lock** the value so nobody gets it. `1` (from protocol version 14): masternodes vote for a contender or abstain; there is no lock, so the contest always ends with a winner. |
| `fieldMatches` | array of at least one `{ "field", "regexPattern" }` | Which values are contested. `field` is a property path of the document; `regexPattern` a regular expression its value must match. Each is 1 to 256 characters. |
| `description` | string, 1 to 256 characters | A note for readers. Consensus does not read it. |

## How it works

**Which documents are contested.** A document is contested when, for every `fieldMatches` entry, the document holds a string at `field` and `regexPattern` matches it. A missing value, a value that is not a string, or one entry that does not match makes the document an ordinary unique-index document. Without `fieldMatches`, every document the index covers is contested. The pattern uses the syntax of Rust's `regex` crate and matches anywhere in the value, so write `^` and `$` to match the whole of it, as DPNS does.

**Values outside the contested range** behave exactly like any unique index: the first document takes the value, and a later create with the same values is refused with `DuplicateUniqueIndexError` (40105).

**Opening or joining a contest.** A create whose values are contested must carry `$prefundedVotingBalance`, a pair `[indexName, amount]`: the contested index's name and the most credits the contender will pay to fund the vote. The document is not stored under the index yet; it is held as a contender until the contest ends. A create is refused:

- with `DocumentContestNotPaidForError` (40114) when it carries no prefunded balance, or less than the contest's fund. The fund is the contested document fund, 0.1 Dash. From protocol version 14 it doubles once the contest holds 250 contenders and again for every 50 more, and a contender is charged the fund and keeps what it stated beyond it; before 14 every contender stated and paid exactly the fund.
- with `DocumentContestIndexMismatchError` (40118, from protocol version 14) when the pair names another index than the contested one its values match.
- with `DocumentContestNotRequiredError` (40119, from protocol version 14) when the document is not contested but carries a prefunded balance.
- with `DocumentContestNotJoinableError` (40111) when the contest's join window (one week on mainnet) has passed.
- with `DocumentContestIdentityAlreadyContestantError` (40112) when its owner is already a contender.
- with `DocumentContestMaximumContendersReachedError` (40141, from protocol version 14) when the contest already holds 1,000 contenders.
- with `DocumentContestCurrentlyLockedError` (40110) when an earlier contest for these values ended locked.

**The vote.** Masternodes and evonodes vote with `MasternodeVote` transitions for the length of the poll (two weeks on mainnet). Under `resolution: 0` the contender with the most votes wins unless the lock tally beats it, and the contest always runs its full length, even with one contender. Under `resolution: 1` a contender always wins, and a contest that still has a single contender when its join window closes is awarded to it at once. From protocol version 14 a tie goes to the earliest contender.

**After the contest.** The winning document is stored and held by the index like any unique-index document; the other contenders' documents are removed. The contest's result stays readable.

The fund, the windows, the tallies and the special case of moderation elections are described in [Contested Documents](../data-model/contested-documents.md).

## Rules at registration

- The index is `unique: true`. A contested index that is not unique is refused (`InvalidContractStructure`, 10231).
- The document type's documents cannot be replaced: `documentsMutable: false` (`ContestedUniqueIndexOnMutableDocumentTypeError`, 10248). They may still be transferred and sold, as DPNS domains are.
- A document type has at most one contested index, and no other unique index beside it (`ContestedUniqueIndexWithUniqueIndexError`, 10249).
- `resolution` is present and is `0` or `1`; `1` is refused before protocol version 14.
- `fieldMatches`, when present, holds at least one entry, and each `regexPattern` is a valid regular expression (`RegexError`, 10247).
- A contested index cannot carry a [`timeRange`](time-range.md), an [`integerRange`](integer-range.md) or a ranking (a ranking needs a non-unique index), and an [index-only type](index-only.md) cannot have one.
- A document type with a contested index cannot set [`ttl`](ttl.md), nor `moderatorAbilities.delete` (see [Deletion](deletion.md)): a moderator's restore puts a document back by an ordinary insert, and a contested value is only awarded through a vote.
- From protocol version 14, a document type with a contested index that sums a property ([`summable`, `averageable`, `documentsSummable` or `documentsAverageable`](aggregates.md)) declares that property with a `minimum` of at least -134217728 and a `maximum` of at most 134217728 (±2^27, `max_contested_summed_value_magnitude`). The end of a contest adds the winner's value to the type's sums, after any number of other documents were written, and a sum that left the signed 64-bit range there could not be stored. Values this small keep the sums in range short of 2^36 documents. Checked when a contract is registered or updated, so contracts registered earlier keep their bounds.

## See also

- [Contested Documents](../data-model/contested-documents.md) for the contest's lifecycle, fund, resolutions, ties and storage.
- [Indexes](indexes.md) for `unique` and the other index keywords.
- [Mutability](mutability.md) for `documentsMutable`, which a contested type sets to `false`.
