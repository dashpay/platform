# drive-abci

This crate contains the Drive ABCI application together with a couple of helpers.

## `prepare_proposal_apphash` example

The `prepare_proposal_apphash` example replays a `RequestPrepareProposal` against an existing
GroveDB database and prints the resulting app hash. It loads the same `.env` configuration format
as the full server, so you can point it at the same credential and RPC settings that were used
when the faulty block was produced.

```bash
cargo run -p drive-abci --example prepare_proposal_apphash -- \
  --db-path /path/to/grovedb \
  --request /tmp/request.ron \
  --config /path/to/.env \
  --request-format ron
```

Notes:

- `--request` supports both JSON and RON (default) files. When using RON you can either paste the
  full `Request` dump from logging (`Request { value: Some(PrepareProposal(...)) }`) or only the
  `RequestPrepareProposal { ... }` portion.
- JSON payloads should be plain `RequestPrepareProposal` objects that follow the proto field names
  (e.g., `max_tx_bytes`, `proposer_pro_tx_hash`, …).
- The `.env` file is optional; the loader walks up the directory tree just like the production
  binary if `--config` is omitted.
- The program prints the computed app hash in hex (`app_hash: 0x...`), making it easy to compare
  two runs.
