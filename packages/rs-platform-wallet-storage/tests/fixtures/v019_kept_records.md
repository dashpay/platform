# Immutable V019 kept records

`v019_kept_records.txt` contains `name txid outpoint record_blob`, with binary
columns in hex. Do not regenerate it when runtime serializers change.

Shipped 5.0 stores (schema V018) were written with rust-dashcore/key-wallet
revision `d631aaca`. These fixture bytes were encoded once at
`aca020263990a86a15cc1d97ae2cbf965d488db5` with grovedb-bincode `2.1.0`, using bincode's standard little-endian variable-integer
encoding. The serde types used by these rows are compatible with `d631aaca`:
TransactionRecord gained methods without changing its fields; TransactionContext,
AccountType and InstantLock stayed the same; TransactionPayload appended variants;
Address serde stayed unchanged. These are generated compatibility fixtures for
the pending record format that reaches V020.

The writer uses `blob::encode(TransactionRecord)`. Both rows belong under SQL
`height = NULL`. Live-codec tests decode these exact bytes and assert their
values and unchanged re-encoding; the V020 test asserts literal indexed keys.
An incompatible dependency update must supply a historical decoder at that point.

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
