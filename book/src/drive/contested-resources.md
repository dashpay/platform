# Contested Resources

A contested resource is a scarce value under a unique index that more than one identity may want, the canonical example being a short DPNS name. Instead of "first write wins", documents that claim the same value under a contested index enter a masternode vote, each contender's document is held until the poll ends, and a native block event awards the value to the winner. This chapter is about the contract side of that mechanism: what a contract may declare, which parameters the native rules honour, which rules govern which actions, and why nothing outside the native rules can influence the award.

The vote itself (the `MasternodeVote` transition, vote weights, abstain and lock choices, the vote state queries) is covered by the voting queries and the masternode vote transition; the storage layout of a contest (the contenders tree, the end-date queue, the stored info) is internal to Drive.

## The declaration

A contested index is a unique index with a `contested` object:

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
      { "field": "normalizedLabel", "regexPattern": "^[a-zA-Z01]{3,19}$" }
    ],
    "resolution": 0,
    "description": "Short names are contested; longer names register directly."
  }
}
```

The object has three members, and no others:

- `fieldMatches` (optional): a list of `{ field, regexPattern }`. A document is contested only when every match holds against the named property. Without the list every document under the index is contested.
- `resolution` (required): how a contest is resolved. `0`, a masternode vote, is the only resolution.
- `description` (optional): free text for readers of the contract. It is dropped at parse time and has no effect on the contest.

These three members are the whole surface through which a contract parameterizes a contest. There is no hook, callback, guard or predicate slot on the declaration, and a later chapter of the platform (native guards and Wasm predicates for ordinary document actions) adds none: a declaration that claims to govern an award is rejected at registration, exactly as an unknown key is.

The parser enforces the shape: a contested index must be `unique`, a document type may carry at most one contested index and no other unique index beside it, the type's documents must be immutable (`documentsMutable: false`), a `timeRange` index cannot be contested, an indexOnly type cannot carry one, `resolution` must be a known value and every `regexPattern` must compile.

## Supported parameters

From protocol version 17 the parameters themselves are validated at contract create and update, by `validate_contested_index_parameters`, a versioned class method called from the shared document-type parser under full validation (`packages/rs-dpp/src/data_contract/document_type/class_methods/validate_contested_index_parameters/`). Four rules, each closing a way for a declaration the parser accepts to break once a contest starts:

| Rule | Why the native machinery needs it |
|---|---|
| Every property of a contested index is a top-level user property (no dotted path, no `$` system property). | The vote poll key is built by a flat lookup of the document's properties, while the contested tree walker resolves the same names through the document type. A nested or system property gives the two different segments; today a nested property yields an empty poll key and every create on the contest fails inside the node. |
| Every property of a contested index is listed in `required`. | The contested tree cannot key a null. An absent property reaches the walker as an empty key. |
| Every `fieldMatches` entry names a property of the index. | A match on a property outside the index lets two documents with equal index values take different paths (one becomes a contender, the other an ordinary unique insert). When the poll ends, the award collides with the ordinary document in the unique index while the block executes. |
| Every matched property is a string. | A regex never matches a non-string, so the index silently degrades to a plain unique index and no contest can start. |

A rejected declaration fails the create or update with `ContestedIndexInvalidParametersError` (code 10277), naming the document type, the index and the parameter. The check is versioned on `dpp.validation.document_type.validate_contested_index_parameters`: `None` on every protocol version before 17, so contracts stored before the check existed are never re-judged (a stored contract is parsed without full validation and never runs it), and a contract accepted at protocol version 14 to 16 stays accepted on replay. DPNS and every contested fixture in the tree pass all four rules.

### Parameters are frozen on update

A contract update cannot re-parameterize a contest. The contested declaration is part of the parsed `Index` value, and `validate_update` (generation 1 from protocol version 14) compares every index by name and rejects any difference as `DataContractInvalidIndexDefinitionUpdateError` ("changed index"). A changed `regexPattern`, an added or removed field match, a change of the matched field or the removal of the declaration are all rejected. A description-only edit compares equal and is accepted, because the description never reaches the contest. This is what keeps a later update from rewriting how an existing poll resolves.

## Rule scopes

Two kinds of actions touch a contested document type, and they answer to different rules.

**Ordinary document actions** (create, replace, delete, transfer, purchase, price update) are governed by ordinary validation: the schema, the index rules, `documentsMutable` and `canBeDeleted`, `creationRestrictionMode`, the data triggers of the system contracts and, when they arrive, the native guards and Wasm predicates a contract declares for those actions. A contested create is an ordinary create with one more requirement: it must prefund the vote resolution and name the poll the document resolves to (`DocumentContestIndexMismatchError` if it names another). On DPNS, for instance, the create trigger demands a matching preorder and the reject triggers refuse every replace and delete of a domain.

**The award** is not a document action. It is a native block event, run by every node at the end of the block in which the poll's end date is reached, with no submitter, no fee and no state transition. It is governed only by the native contested rules: eligibility, the tallies, the lock and abstain choices, the tie-break and the insertion of the winning document with its index effects. No data trigger, creation restriction, guard or predicate runs on it, whether or not the contract declares one for the create action, because the award is outside every ordinary-action rule scope. A trap, a rejecting guard or a guard that never returns are all never invoked for an award. The event tests in `rs-drive-abci` pin this on DPNS itself: both contenders delete their preorder before the poll ends, so the create trigger would reject the winning document if it were re-run, and the award still succeeds while the winner's later replace and delete are still rejected.

Which rule applies is therefore a property of the action, never of the document type or the contract: a contract may carry rules on other actions of a contested type without those rules ever reaching the award.

## The native award

From protocol version 17 the award is one Drive operation, `Drive::award_contested_document_vote_poll` (`packages/rs-drive/src/drive/document/insert_contested/award_contested_document_vote_poll/`), called by generation 1 of `check_for_ended_vote_polls` for every poll the end-date sweep returns. Its authority comes from what it refuses to take as input: the operation receives the resolved poll and the end date the sweep found it under, and nothing else. Every fact of the award is read from state inside the operation:

1. **The poll is a started contest.** Its stored info exists and its status is `Started`. A poll already awarded or locked, a poll that never started and a poll nobody contested are all rejected.
2. **The poll has ended and is queued for finalization.** The block time has reached the end date, and the poll's unique id is present in the end-date queue under that exact end date. The queue entry is written once, when the contest starts, with the real end date, and removed by the cleanup that follows a legitimate award. A caller can therefore neither award early, nor name an arbitrary end date, nor award the same poll again after cleanup.
3. **The winner is the native selection.** Contenders are tallied from the vote state; the highest tally wins; a tie is broken by the greatest creation time, then creation block height, then creation core height, then document id (the rule the block executor always applied); a lock tally strictly above the top contender locks the poll instead; no contender means no winner.
4. **The awarded bytes are the stored contender document.** The winner's document, as it was serialized when the contest was joined, is inserted through the same generic document insert the block executor used before the operation existed, so a legitimate award writes the same bytes and the same index entries as before.

Anything else is `DriveError::ContestedAwardRejected` with nothing applied. Between the award and the record keeper's status flip inside the same transaction, a second call fails on the primary storage existence check of the insert and cannot produce a divergent award. The operation has no estimation variant, takes no contender, and is on no state transition, batched action or Drive batch operation; a test in its dispatcher enumerates every variant of those enums without a wildcard so a future variant that could carry an award has to revisit it. Its version slot, `drive.methods.document.insert_contested.award_contested_document_vote_poll`, is `None` on every table before protocol version 17, so a historical table cannot dispatch it at all.

Crate visibility cannot separate a future host adapter from the block executor, and the design does not rely on it: the checks above hold for any caller holding a `Drive`, and every outcome such a caller can obtain is the one the native rules produce for the current state.

## Finalization once, and atomically

The award, the finalization record and the cleanup are one block transaction in one event. The record keeper fetches the poll's stored info, appends the finalized event and flips the status (`Awarded`, `Locked`, or back to `NotStarted`), and refuses a poll that is not `Started`; the cleanup removes the contenders, their votes, the end-date entry and the prefunded balance. After the block, the winning document is present in primary storage and in every index of its type, the stored info carries exactly one finalized event for the poll, and a later sweep finds nothing to award.

## Lifecycle

Contract lifecycle events that a later protocol version adds around contracts with code (freezing a contract's actions, wiping its state) are native lifecycle rules with their own deterministic effect on a running poll. They are never a veto over a poll's outcome, and no contract can trigger them to redirect, delay or retry an award; the outcome of a contest a contract declares is identical to the outcome of the same contest on a contract without code.
