# Getting Started

This guide covers prerequisites and local development setup for the Dash
Platform monorepo.

## Prerequisites

- [Node.js](https://nodejs.org/) v20+
- [Docker](https://docs.docker.com/get-docker/) v20.10+
- [Rust](https://www.rust-lang.org/tools/install) v1.92+, with the wasm32
  target:

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- [protoc (Protocol Buffers compiler)](https://github.com/protocolbuffers/protobuf/releases)
  v32.0+. If `protoc` is not on your `PATH`, set the `PROTOC` environment
  variable to the binary location.
- [wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/):

  ```bash
  cargo install wasm-bindgen-cli@0.2.103
  ```

  > **Important:** the `wasm-bindgen-cli` version **must** match the
  > `wasm-bindgen` version in `Cargo.lock`. Check with
  > `grep 'name = "wasm-bindgen"' Cargo.lock`.

  Depending on your system, you may need additional packages before
  `wasm-bindgen-cli` will compile (e.g. `clang`, `llvm`, `libssl-dev`).

- [wasm-pack](https://rustwasm.github.io/wasm-pack/):

  ```bash
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
  ```

- Build essentials (Debian / Ubuntu):

  ```bash
  apt install -y build-essential libssl-dev pkg-config clang cmake llvm
  ```

### macOS-specific notes

The built-in Apple `llvm` toolchain does **not** work for WASM compilation.
Install LLVM from Homebrew and put it on your `PATH`:

```bash
brew install llvm
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

If you use Bash, replace `~/.zshrc` with `~/.bash_profile`. You can check your
default shell with `echo $SHELL`.

## Setup

```bash
# Enable corepack (ships with Node.js) to get the correct yarn version
corepack enable

# Install dependencies, configure, and build all packages
yarn setup

# Start the local development environment (runs a local Dash network in Docker)
yarn start

# Run the full test suite (requires a running local node)
yarn test

# Stop the local environment (frees system resources)
yarn stop

# Rebuild after making changes
yarn build

# If you need to restart services after a rebuild
yarn restart

# Complete reset of all local data and builds
yarn reset
```

## Running package-level tests

You can run tests for a single package instead of the entire suite:

```bash
yarn workspace <package_name> test
```

For example:

```bash
yarn workspace @dashevo/dapi-client test
```

See the [packages directory](https://github.com/dashpay/platform/tree/master/packages)
for the full list of available packages.

## Rust development

```bash
# Run tests for a specific Rust crate
cargo test -p <crate_name>

# Run all Rust workspace tests
cargo test --workspace

# Check compilation without building
cargo check --workspace

# Run the clippy linter
cargo clippy --workspace

# Format all Rust code
cargo fmt --all
```
