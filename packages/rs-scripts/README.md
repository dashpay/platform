# rs-scripts

Utility scripts for debugging and inspecting Dash Platform data.

## decode-document

Decodes a base64-encoded platform document into human-readable output. Uses the actual platform deserialization code, so it handles all document format versions correctly.

### Usage

```bash
cargo run -p rs-scripts --bin decode-document -- <BASE64_DOC> [OPTIONS]
```

### Options

| Option | Required | Description |
|--------|----------|-------------|
| `-c, --contract` | yes | System data contract name or ID (base58/base64/hex) |
| `-d, --doc-type` | yes | Document type name within the contract |

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
