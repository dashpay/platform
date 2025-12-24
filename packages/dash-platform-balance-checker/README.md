# Dash Platform Balance Checker

A collection of Rust command-line tools for checking Dash Platform balances using various SDK configurations and providers.

## Overview

This package contains multiple binary targets that demonstrate different approaches to fetching Dash Platform balances:

- **dash-platform-balance-checker**: Main balance checker with full SDK integration (supports both mainnet and testnet)
- **dash-platform-balance-simple**: Direct DAPI client approach without full SDK overhead  
- **dash-platform-balance-trusted**: Uses trusted HTTP context provider instead of Core RPC

## Building

```bash
cargo build --release
```

## Running

Each binary can be run independently:

```bash
# Main balance checker (requires identity ID)
cargo run --bin dash-platform-balance-checker <identity-id> [--testnet]

# Direct DAPI implementation (hardcoded testnet example)
cargo run --bin dash-platform-balance-simple

# With trusted provider (hardcoded testnet example)
cargo run --bin dash-platform-balance-trusted
```

## Dependencies

- `dash-sdk`: Dash Platform SDK
- `rs-dapi-client`: DAPI client implementation
- `dapi-grpc`: gRPC definitions for DAPI
- `dpp`: Dash Platform Protocol
- `drive-proof-verifier`: Proof verification
- `rs-sdk-trusted-context-provider`: Trusted HTTP context provider
- `tokio`: Async runtime
- `anyhow`: Error handling

## Usage

### Identity ID Format

The tools accept identity IDs in Base58 format. Example:
```
5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk
```

### Examples

#### Using the main balance checker
```bash
# Check balance on mainnet with local Core (default)
cargo run --bin dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk

# Check balance on testnet
cargo run --bin dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk --testnet

# Check balance with custom Core connection
cargo run --bin dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk \
  --core-host 192.168.1.100 --core-port 9998 \
  --core-user myuser --core-password mypass

# Check balance without Core connection
cargo run --bin dash-platform-balance-checker 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk --no-core

# Show help
cargo run --bin dash-platform-balance-checker --help
```

#### Command Line Options
- `--testnet` - Use testnet instead of mainnet (default: mainnet)
- `--core-host <host>` - Core RPC host (default: localhost)
- `--core-port <port>` - Core RPC port (default: 9998 for mainnet, 19998 for testnet)
- `--core-user <username>` - Core RPC username (default: dashrpc)
- `--core-password <password>` - Core RPC password (default: password)
- `--no-core` - Skip Core connection (may limit some functionality)

#### Expected Output
```
Dash Platform Balance Checker - Mainnet
=====================================

Identity ID: 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk
Parsed Identity ID: 5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk

Core connection: localhost:9998
Core user: dashrpc

Connecting to mainnet...
Fetching identity...

✓ Identity found!
  Balance: 1000000000 credits
  Balance in DASH: 0.01 DASH
  Revision: 1
  Public keys: 2
```

## Network Configuration

The main balance checker supports both networks:

### Mainnet (default)
- https://dapi.dash.org:443
- https://dapi-1.dash.org:443
- https://dapi-2.dash.org:443

### Testnet (with --testnet flag)
- https://52.13.132.146:1443
- https://52.89.154.48:1443
- https://44.227.137.77:1443
- (and others)

## Prerequisites

- Rust toolchain (1.70+)
- Internet connection to reach network nodes
- Dash Core node (optional but recommended):
  - Default ports: 9998 (mainnet) or 19998 (testnet)
  - Default credentials: dashrpc/password
  - Can be skipped with `--no-core` flag

## Documentation

For JavaScript/Web implementations, see the `dash-platform-balance-checker-web` package.

## License

MIT