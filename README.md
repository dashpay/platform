: <!-- markdownlint-disable MD033 MD041 -->
<p align="center">
  <a href="https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/">
    <img alt="babel" src="https://media.dash.org/wp-content/uploads/dash_digital-cash_logo_2018_rgb_for_screens.png" width="546">
  </a>
</p>

<p align="center">
  Seriously fast decentralized applications for the Dash network
</p>

<p align="center">
  <a href="https://github.com/dashpay/platform/actions/workflows/all-packages.yml"><img alt="GitHub CI Status" src="https://github.com/dashpay/platform/actions/workflows/all-packages.yml/badge.svg"></a>
  <a href="https://chat.dashdevs.org/"><img alt="Devs Chat" src="https://img.shields.io/badge/discord-Dev_chat-738adb"></a>
  <a href="https://discordapp.com/invite/PXbUxJB"><img alt="General Chat" src="https://img.shields.io/badge/discord-General_chat-738adb"></a>
  <a href="https://twitter.com/intent/follow?screen_name=Dashpay"><img alt="Follow on Twitter" src="https://img.shields.io/twitter/follow/Dashpay.svg?style=social&label=Follow"></a>
</p>

## Table of Contents

- Philosophy
- Install (TODO)
- Usage (Testnet Only)
- Packages Overview
- Build
  - Repo
  - Pre-Requisites
    - Linux
    - Mac

## Philosophy

Dash Platform is a technology stack for building decentralized applications on
the Dash network. The two main architectural components, Drive and DAPI, turn
the Dash P2P network into a cloud that developers can integrate with their
applications.

If you are looking for how to contribute to the project or need any help with
building an app on the Dash Platform - message us on the [Devs
Discord](https://chat.dashdevs.org/)!

## Install

Coming soon...

## Usage (Testnet Only)

Dash Platform is currently available on the Dash Testnet only.

| Command           | Description                                                                  |
| ----------------- | ---------------------------------------------------------------------------- |
| `corepack enable` | Enables `yarn`. See <https://nodejs.org/dist/latest/docs/api/corepack.html>. |
| `yarn setup`      | Install dependencies and configure and build all packages.                   |
| `yarn build`      | Rebuild the project after changes (then run `yarn restart` to apply them).   |
| `yarn restart`    | `yarn stop` and `yarn start`                                                 |
| `yarn start`      | Start the local dev environment built from the sources.                      |
| `yarn stop`       | Stop the local dev environment.                                              |
| `yarn test`       | Run the whole test suite (run `yarn start` first to start a node to test).   |
|                   | Run tests for a specific package with `yarn workspace <package_name> test`.  |
|                   | See the [./packages/README.md](./packages/README.md) for available packages  |
|                   | Ex: `yarn workspace @dashevo/dapi-client test` tests the JS DAPI client      |
| `yarn reset`      | Completely reset all local builds and data                                   |

A note on **System Resources**: Running a dev environment requires a non-trivial amount of system resources,
 so it is best to stop the local node when not in use.

Todo.

## Packages Overview

🤯 See the README in [./packages/](./packages/) for a **structured listing**.

### TL;DR

This monorepo contains all of the Dash platform packages - JavaScript, Rust, and otherwise.

For example:

- Drive (storage layer, in Rust)
- JavaScript SDK
- wallet-lib
- DAPI

Some packages have a `README.md`.

The ones that don't, should. #up-for-grabs

## Build

**DO THIS FIRST**

0. Clone and enter `dashpay/platform`
   ```sh
   # a shallow clone is 115.66 MiB
   git clone --depth=1 git@github.com:dashpay/platform.git ./dashpay-platform/
   pushd ./dashpay-platform/
   ```

### Pre-Requisites

- [node.js](https://nodejs.org/) v18
- [docker](https://docs.docker.com/get-docker/) v20.10+
- [rust](https://www.rust-lang.org/tools/install) v1.67+ \
  - `wasm32-unknown-unknown` target
- [wasm-bingen toolchain](https://rustwasm.github.io/wasm-bindgen/)
- [`protoc`](https://github.com/protocolbuffers/protobuf/releases) 22.4

On Mac and Linux:

```sh
# Installs rust from rustup.sh
curl https://webi.sh/rust | sh

# Installs node from nodejs.org
curl https://webi.sh/node@v18 | sh

# Immediately update PATH without opening a new Terminal
source ~/.config/envman/PATH.env
```

##### Linux

Install `llvm` (clang) and other build tools:

```sh
# System Tools
sudo apt install -y \
    curl \
    gnupg2

# Build Tooling
sudo apt install -y \
    build-essential \
    clang \
    cmake \
    g++ \
    gcc \
    libgmp-dev \
    libpython3.10-dev \
    libssl-dev \
    libzmq3-dev \
    pkg-config

# usually not needed, but just in case
# (lsb includes things "minimal" Linux installs may lack)
sudo apt install -y \
    lsb-release
```

```sh
apt install -y protobuf-compiler
```

##### macOS

⚠️ You'll need a **conflict-free brew** intstall. \
⚠️ You'll need a **custom install of llvm**. \
❌ The **built-in llvm** will not work. \
❌ A **legacy brew** install may cause conflicts. \
✅ The install method below is **conflict-free** (it won't bork your install of macOS). \
⏰ This takes 20 minutes on an Apple M2 Pro Max. \
   It may take an hour or two or your machine. \
   (due to the conflict-free path, more dependencies will be freshly compiled)

**Why the conflict-free method?**

> "macOS already provides this software and installing another version in
> parallel can cause all kinds of trouble." - `brew`

```sh
# 0. Backup `brew` if it's installed in potentially conflicting system location
brew bundle dump --file ./Brewfile-"$(date '+%F')"
# (and check the backup)
cat ./Brewfile-*

# 1. Install `brew` in conflict-free fashion & restore your builds.
#    Webi does this in accordance with brew's conflict-free / tar anywhere method:
#    https://docs.brew.sh/Installation#alternative-installs
curl https://webi.sh/brew | sh
brew bundle --file ./Brewfile-*

# 2. Install `llvm` in conflict-free fashion.
#    This is important because we don't want to cause issues with macOS' built-in 'llvm'
brew install llvm
# install from ./dashpay-platform/Brewfile
brew bundle --file ./Brewfile
   
# 3. Install `pathman` to more edit PATHs
curl https://webi.sh/pathman | sh
source ~/.config/envman/PATH.env

# 4. Update your PATH to include our special llvm.
#    This works for bash, zsh, and fish
pathman add ~/.local/opt/brew/opt/llvm/bin
```

```sh
export LDFLAGS="-L$HOME/.local/opt/brew/opt/llvm/lib/c++ -Wl,-rpath,${HOME}/.local/opt/brew/opt/llvm/lib/c++"
set -gx LDFLAGS "-L$HOME/.local/opt/brew/opt/llvm/lib"
set -gx CPPFLAGS "-I$HOME/.local/opt/brew/opt/llvm/include"
```

```sh
brew install protobuf
```

### Others

- `protoc`: See [Protocol Buffers releases page]()

#### Build Evo

0. You may need to set the `PROTOC` ENV to the install path of `protoc`:
   ```sh
   command -v protoc
   # ex: ~/.local/opt/brew/bin/protoc
   
   export PROTOC="$HOME/.local/opt/brew/bin/protoc"
   ```
2. Build `wasm-bindgen` (TODO this needs to go into a script)
   ```sh
   # Check the VERSION (ex: 0.2.85)
   grep -B 1 -A 8 'name = "wasm-bindgen"' Cargo.lock

   # Install *that* version of the CLI
   my_wbg_cli_ver="$(
       grep -B 1 -A 8 'name = "wasm-bindgen"' Cargo.lock |
           grep 'version' |
           cut -d'"' -f2
   )"
   echo "$my_wbg_cli_ver"
   # ex: 0.2.86
   cargo install wasm-bindgen-cli@"$my_wbg_cli_ver"
   wasm-bindgen -V
   ```
3. Install wasm32 rust target
   ```sh
   rustup target add wasm32-unknown-unknown
   ```
3. Install platform packages
   ```sh
   corepack enable
   yarn install
   ```

### Supported networks

Dash Platform is currently undergoing testing and final development necessary to
support its release on the Dash production network (mainnet). The packages in
this repository may be used on the following networks:

- [x] **Development networks** ([**devnets**](https://dashplatform.readme.io/docs/reference-glossary#devnet))
- [x] [**Testnet**](https://dashplatform.readme.io/docs/reference-glossary#testnet)
- [ ] [Mainnet](https://dashplatform.readme.io/docs/reference-glossary#mainnet)

## FAQ

### Looking for support?

For questions and support, please join our [Devs
Discord](https://chat.dashdevs.org/)

### Where are the docs?

Our docs are hosted on
[readme.io](https://dashplatform.readme.io/docs/introduction-what-is-dash-platform).
You can create issues and feature requests in the
[issues](https://github.com/dashpay/platform/issues) for this repository.

### Want to report a bug or request a feature?

Please read through our [CONTRIBUTING.md](CONTRIBUTING.md) and fill out the
issue template at [platform/issues](https://github.com/dashpay/platform/issues)!

### Want to contribute to Dash Platform?

Check out:

- Our [Developers Discord](https://chat.dashdevs.org/)
- Our [CONTRIBUTING.md](CONTRIBUTING.md) to get started with setting up the
  repo.
- Our [news](https://www.dash.org/news/) and [blog](https://www.dash.org/blog/) which contains release posts and
  explanations.

## License

[MIT](LICENSE.md)
