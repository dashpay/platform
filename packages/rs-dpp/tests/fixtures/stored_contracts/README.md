# Stored contract fixture

The bytes a node stores contracts in at protocol version 15: format version 2, each
document type compiled in stored generation 0. `tests/stored_contract_compatibility.rs`
stores these contracts at protocol version 15 and compares the result with `stored.jsonl`:

- every system contract protocol version 15 has;
- every contract of the corpus (`../contract_corpus`), loaded from its schemas as Drive
  loads a contract stored before protocol version 15;
- the contracts written for this fixture, for what neither of those declares:
  `settled_deletion_and_action_fees.json`, a contract moderated by an elected team, whose
  type says who approves a settled deletion (`deleteSettled`) and declares `actionFees`.

Each line of `stored.jsonl` is one contract:

```json
{"contract": "system DPNS", "sha256": "<hex>", "indexStructures": "<hex>", "bytes": "<base64>"}
```

- `contract`: `system <name>`, `mainnet <id>` or `testnet <id>` for a corpus contract, or
  `written <file>`.
- `sha256`: the SHA-256 of the stored bytes.
- `indexStructures`: the SHA-256 of the `Debug` rendering of each document type's index
  structure, which the stored bytes do not hold: reading them back rebuilds it from the
  stored indexes with the shared builder.
- `bytes`: the stored bytes, only for the contracts kept whole: every system contract,
  every mainnet contract and every written contract, and of testnet's, for each part of a
  contract the test lists (`Exercises`), the one stored in the fewest bytes that has it.

Keeping every contract whole would take about 4.5 MB (3.2 MB of stored bytes). The
hashes check what the code stores for every contract; the whole bytes also check that
bytes stored before a change still read back as the contract its schemas build.

## Regenerating

Only while protocol version 15 is unreleased: not in a stable release, and not activated
on testnet or mainnet. Until then, changing what protocol version 15 stores or reads back
is an edit of an unreleased format, and the fixture follows it:

```bash
UPDATE_STORED_CONTRACT_FIXTURE=1 cargo test -p dpp --test stored_contract_compatibility should_store_every
cargo test -p dpp --test stored_contract_compatibility
```

Check the diff: only the contracts the change was meant to affect may change.

Once protocol version 15 has shipped, nodes hold these bytes in state. A change that
fails the test is a new stored generation selected by a new protocol version, with
generation 0, the types it embeds and the index structure builder it calls left as they
are. Regenerating is then only for two cases:

- A corpus taken again: the diff may only touch the lines of contracts the new corpus
  added, dropped or holds at another version.
- A field of `IndexLevel`, or of a type a level holds, renamed: that changes the `Debug`
  rendering `indexStructures` hashes without changing any level. The diff may only touch
  `indexStructures`, and the pull request shows the renderings before and after differ
  only by the name.
