# Immutable V019 kept records

`v019_kept_records.txt` contains `name txid outpoint record_blob`, with binary
columns in hex. Do not regenerate it when runtime serializers change.

These rows were encoded once with the V019 writer's unchanged dependencies:
key-wallet/rust-dashcore `aca020263990a86a15cc1d97ae2cbf965d488db5`,
grovedb-bincode `2.1.0`, bincode's standard little-endian variable-integer encoding.
The writer uses `blob::encode(TransactionRecord)`; V020 did not change that
writer or dependency revision. Both rows belong under SQL `height = NULL`.

- `mempool`: `TransactionContext::Mempool`, transaction lock time 17.
- `instant_send`: `TransactionContext::InstantSend`, lock time 18. Its lock
  names the transaction and the same input. The quorum signature is a zero
  test value; migration decoding does not verify quorum signatures.

Both are version-2 transactions spending output 7 of txid `[0x42; 32]`, with
scriptSig `51`, sequence `u32::MAX - 1`, and witness `[010203]`. The output
pays 999 duffs to testnet P2PKH hash `[0x11; 20]`. Each record uses standard
BIP44 account 0, standard transaction type, internal direction, one input
detail (1000 duffs), one change-output detail (999 duffs), net -1, fee 1,
and label `V019 <name> fixture`. The input and output details carry that
address, exercising the historical address codec as well as scripts/witnesses.
