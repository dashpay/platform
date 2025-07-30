# Dash Platform WASM JS SDK

This package provides WebAssembly bindings for the Dash Platform SDK, allowing JavaScript and TypeScript applications in browsers to interact with Dash Platform.

## Overview

The WASM JS SDK provides:
- **Queries**: Read-only operations to fetch data from Dash Platform
- **State Transitions**: Write operations to modify state on Dash Platform

## Usage

### Quick Start

1. Build the WASM module:
   ```bash
   ./build.sh
   ```

2. Serve the demo application:
   ```bash
   python3 -m http.server 8888
   ```

3. Open http://localhost:8888 in your browser

### Integration

```javascript
import init, { WasmSdk } from './pkg/wasm_sdk.js';

// Initialize WASM module
await init();

// Create SDK instance
const transport = { 
    url: "https://52.12.176.90:1443/", // testnet
    network: "testnet"
};
const sdk = await WasmSdk.new(transport, true); // true = enable proofs

// Example query
const identity = await sdk.get_identity("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
```

## Documentation

- **[User Documentation](docs.html)**: Comprehensive guide for all queries and state transitions
- **[AI Reference](AI_REFERENCE.md)**: Quick reference optimized for AI assistants and developers
- **[Live Demo](index.html)**: Interactive interface to test all SDK functionality

## Development

### Building

The SDK requires Rust and wasm-pack:

```bash
# Install wasm-pack if not already installed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the WASM module
./build.sh
```

### Documentation

**IMPORTANT**: Documentation must be kept in sync with `index.html`. When adding or modifying queries/state transitions in `index.html`, you MUST update the documentation:

```bash
# Regenerate documentation after changes to index.html
python3 generate_docs.py

# Check if documentation is up to date
python3 check_documentation.py
```

The CI will fail if documentation is out of sync with the code.

### Adding New Features

1. Add the query/transition definition to `index.html`
2. Implement the corresponding method in the Rust code
3. Regenerate documentation: `python3 generate_docs.py`
4. Test your changes using the web interface

### CI/CD

This package has automated checks for:
- Documentation completeness (all queries/transitions must be documented)
- Documentation freshness (docs must be regenerated when index.html changes)

The checks run on:
- Pull requests that modify relevant files
- Pushes to master and release branches

## Architecture

### File Structure

- `src/`: Rust source code
  - `queries/`: Query implementations
  - `state_transitions/`: State transition implementations
  - `sdk.rs`: Main SDK interface
- `index.html`: Interactive demo and test interface
- `docs.html`: User-friendly documentation
- `AI_REFERENCE.md`: Developer/AI reference documentation
- `generate_docs.py`: Documentation generator
- `check_documentation.py`: Documentation validation

### Key Concepts

1. **Queries**: Read operations that don't modify state
   - Identity queries (balance, keys, nonce)
   - Document queries (with where/orderBy support)
   - Data contract queries
   - Token queries
   - System queries

2. **State Transitions**: Operations that modify platform state
   - Identity operations (create, update, transfer)
   - Document operations (create, update, delete)
   - Token operations (mint, burn, transfer)
   - Voting operations

3. **Proofs**: Cryptographic proofs can be requested for most queries to verify data authenticity

## Testing

The web interface (`index.html`) provides comprehensive testing capabilities:
- Network selection (mainnet/testnet)
- Query execution with parameter validation
- State transition testing with authentication
- Proof verification toggle

## Contributing

1. Make your changes
2. Update documentation if needed: `python3 generate_docs.py`
3. Run tests
4. Submit a pull request

## License

See the main platform repository for license information.