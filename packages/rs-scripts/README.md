# rs-scripts

Utility scripts for debugging and inspecting Dash Platform data.

## decode-document

Decodes a hex or base64-encoded platform document into human-readable output. Uses the actual platform deserialization code, so it handles all document format versions correctly.

### Usage

```bash
cargo run -p rs-scripts --bin decode-document -- <DOC_BYTES> [OPTIONS]
```

### Options

| Option | Required | Description |
|--------|----------|-------------|
| `-c, --contract` | yes | System data contract name or ID (base58/base64/hex) |
| `-d, --doc-type` | yes | Document type name within the contract |
| `-f, --format` | no | Input encoding: `base64`, `hex`, or `auto` (default: `auto`) |

### Supported contracts

`withdrawals`, `dpns`, `dashpay`, `masternode-reward-shares`, `feature-flags`, `wallet-utils`, `token-history`, `keyword-search`

You can also pass the contract ID directly instead of a name (you'll need `-d` to specify the document type):
```bash
# base58
cargo run -p rs-scripts --bin decode-document -- -c 4fJLR2GYTPFdomuTVvNy3VRrvWgvkKPzqehEBpNf2nk6 -d withdrawal "base64data..."
# base64
cargo run -p rs-scripts --bin decode-document -- -c "NmK7YeF/rj6ilM9gMZf7CqttURgL2LYQTElEpi/i2X8=" -d withdrawal "base64data..."
# hex
cargo run -p rs-scripts --bin decode-document -- -c 3662bb61e17fae3ea294cf603197fb0aab6d51180bd8b6104c4944a62fe2d97f -d withdrawal "base64data..."
```

### Examples

Decode a withdrawal document:
```bash
cargo run -p rs-scripts --bin decode-document -- -c withdrawals -d withdrawal "AgIintqUs1vl..."
```

Decode a DPNS domain document:
```bash
cargo run -p rs-scripts --bin decode-document -- -c dpns -d domain "base64data..."
```

Pipe from a gRPC query (decode each document from the response):
```bash
echo '{"v0":{"prove":false,"data_contract_id":"NmK7YeF/rj6ilM9gMZf7CqttURgL2LYQTElEpi/i2X8=","document_type":"withdrawal","where":"gYNmc3RhdHVzYT0C","limit":10}}' \
  | grpcurl -insecure -import-path packages/dapi-grpc/protos -d @ \
    -proto platform/v0/platform.proto \
    <node-ip>:443 org.dash.platform.dapi.v0.Platform/getDocuments \
  | jq -r '.v0.documents.documents[]' \
  | while read doc; do
      cargo run -p rs-scripts --bin decode-document -- -c withdrawals -d withdrawal "$doc"
      echo "---"
    done
```

## register-contract

Registers a data contract on Dash Platform from a JSON file.

The script fetches the supplied identity, finds which of its public keys
corresponds to the supplied private key, and broadcasts a
`DataContractCreate` state transition. The `id` and `ownerId` fields in
the JSON file are overridden: the on-chain contract id is regenerated
deterministically from `(identity_id, identity_nonce)` and the owner is
set to `--identity`, so fixture contracts under
`packages/rs-drive/tests/supporting_files/contract/` work as-is.

### Usage

```bash
cargo run -p rs-scripts --bin register-contract -- \
  -c <CONTRACT_FILE> \
  -i <IDENTITY_ID> \
  -k <PRIVATE_KEY> \
  -a <DAPI_ADDRESS> \
  [-n testnet|mainnet|devnet|regtest] \
  [--devnet <DEVNET_NAME>]
```

| Option | Required | Description |
|--------|----------|-------------|
| `-c, --contract` | yes | Path to the contract JSON file |
| `-i, --identity` | yes | Identity id (base58) that will own the new contract |
| `-k, --private-key` | yes | Private key for that identity — WIF or 64-char hex |
| `-a, --address` | yes | DAPI address, e.g. `https://52.12.176.90:1443` |
| `-n, --network` | no | `mainnet` \| `testnet` \| `devnet` \| `regtest` (default: `testnet`) |
| `--devnet` | no | Devnet name (only with `--network devnet`) |

The private key must correspond to an `AUTHENTICATION` + `CRITICAL` +
`ECDSA_SECP256K1` key on the identity — that's the only key shape DPP
accepts on a contract-create signature.

### Example

Register the `family` fixture contract under a testnet identity:

```bash
cargo run -p rs-scripts --bin register-contract -- \
  -c packages/rs-drive/tests/supporting_files/contract/family/family-contract.json \
  -i HccabTZZpMEDAqU4oQFk3PE47kS6jDDmCjoxR88gFttA \
  -k cTPVy... \
  -a https://52.12.176.90:1443
```
