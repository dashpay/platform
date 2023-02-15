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

## IMPORTANT! 
### Build on a Mac

To build on a mac, you need to perform two steps. First, install `clang`
from the homebrew. XCode's `clang` doesn't ship with the WASM support. Second,
just adding llvm to the `.zshrc` doesn't seem to work - run 
`AR=/usr/local/opt/llvm/bin/llvm-ar CC=/usr/local/opt/llvm/bin/clang yarn workspace @dashevo/wasm-dpp build:node`
instead.

Alternatively, you can add the following to the `yarn workspace @dashevo/wasm-dpp build:node:mac` instead.

### Class names minification
Library consumers must ignore class names minification for `@dashevo/wasm-dpp` library in their bundlers.  

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Install

## TODO

## Usage

## TODO

## Maintainer

[@antouhou](https://github.com/antouhou)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
