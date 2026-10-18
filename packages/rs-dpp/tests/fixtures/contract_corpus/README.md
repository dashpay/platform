# Data contract corpus

The data contracts registered on mainnet and testnet, as Drive stores them, taken on
18 October 2026. `tests/contract_corpus.rs` loads every one, and later changes to how
contracts are parsed, validated, serialized or stored run over the same set.

Each line of `mainnet.jsonl` and `testnet.jsonl` is one contract:

```json
{"id": "<base58 contract id>", "version": 1, "data": "<base64 of the stored contract bytes>"}
```

`data` is the contract element's bytes, exactly what `getDataContract` returns and what
Drive deserializes with `DataContract::versioned_deserialize_trusted`.

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
stores them.
