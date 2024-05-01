# Dash Platform Protocol JS

[![NPM Version](https://img.shields.io/npm/v/@dashevo/dpp)](https://www.npmjs.com/package/@dashevo/dpp)
[![Build Status](https://github.com/dashevo/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashevo/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashevo/platform)](https://github.com/dashevo/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

The WASM JavaScript binding of the Rust implementation of the [Dash Platform Protocol](https://dashplatform.readme.io/docs/explanation-platform-protocol)

### THIS IS A DEV VERSION, NOT INTENDED FOR A PRODUCTION USAGE JUST YET

## Dev environment

In order for this binding to work, you have to have a rs-platform cloned
alongside platform repo, so you can have access to the rust dpp.

## IMPORTANT

### Build on a Mac

Built-in `llvm` on OSX does not work, it needs to be installed from brew:

- `brew install llvm`
- LLVM installed from brew is keg only, and path to it must be provided in the profile file, e.g.`echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc`

### Class names minification

Library consumers must ignore class names minification for `@dashevo/wasm-dpp` library in their bundlers.  

## Table of Contents

- [Prerequisites](#prerequisites)
- [Build](#build)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Prerequisites

- Install [Rust](https://www.rust-lang.org/tools/install) v1.73+
- Add wasm32 target: `$ rustup target add wasm32-unknown-unknown`
- Install wasm-bingen-cli: `cargo install wasm-bindgen-cli@0.2.85`
  - *double-check that wasm-bindgen-cli version above matches wasm-bindgen version in Cargo.lock file*
  - *Depending on system, additional packages may need to be installed as a prerequisite for wasm-bindgen-cli. If anything is missing, installation will error and prompt what packages are missing (i.e. clang, llvm, libssl-dev)*

## Build

`$ yarn build`

## TODO

## Usage

## TODO

## Maintainer

[@antouhou](https://github.com/antouhou)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
