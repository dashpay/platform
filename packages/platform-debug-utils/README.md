# platform-debug-utils

Small utilities that make inspecting Dash Platform state easier. Run any binary with `--help` to
see detailed usage and examples.

- `rocksdb-dump`: export RocksDB or GroveDB contents into a diff-friendly text file grouped by
  column family.
- `replay_abci_requests`: replay serialized RequestPrepareProposal / RequestProcessProposal payloads
  against a GroveDB snapshot to inspect resulting app hashes and outcomes; see vectors/ for sample payloads.
