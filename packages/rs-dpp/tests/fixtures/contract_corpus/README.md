# Data contract corpus

The data contracts registered on mainnet and testnet, as a node serves them, taken on
18 October 2026. `tests/contract_corpus.rs` loads every one, and later changes to how
contracts are parsed, validated, serialized or stored run over the same set.

Each line of `mainnet.jsonl` and `testnet.jsonl` is one contract:

```json
{"id": "<base58 contract id>", "version": 1, "data": "<base64 of the served contract bytes>"}
```

`data` is the contract as `getDataContracts` and `getDataContractsByRange` return it. The
serving node deserializes the stored contract and serializes it again at its own protocol
version, so every line is in the V1 serialization format, while Drive still stores a
contract written before protocol version 9 in the V0 format. Drive deserializes these
bytes with `DataContract::versioned_deserialize_trusted` as it does stored ones, but the
corpus does not exercise decoding the V0 format: regenerating it from proofs, whose
elements hold the stored bytes, would.

## Where they come from

- **Mainnet:** all 55 contracts, 106 document types. No mainnet node served
  `getDataContractsByRange` when the corpus was taken, so the ids come from the 50
  successful data contract create transitions a block explorer lists, plus the 5 system
  contracts registered at genesis; each contract was then fetched by id with
  `getDataContracts`.
- **Testnet:** 694 contracts, 1,397 document types, enumerated with
  `getDataContractsByRange`. Testnet held 3,982 contracts, most of them repeats of a few
  templates, so only one contract per distinct set of document schemas, `$defs`, tokens
  and groups is kept: the one with the smallest id.

Each contract is at the version its network held on the day. Earlier versions of updated
contracts are not included.

## Updating

Regenerate both files the same way and update the counts `tests/contract_corpus.rs`
asserts. Never edit a line by hand: the bytes are only worth testing because a network
serves them.
