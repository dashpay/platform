<p align="center">
  <a href="https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/">
    <img alt="babel" src="https://media.dash.org/wp-content/uploads/dash_digital-cash_logo_2018_rgb_for_screens.png" width="546">
  </a>
</p>

<p align="center">
  Seriously fast decentralized applications for the Dash network
</p>

<p align="center">
  <a href="https://github.com/dashevo/platform/actions/workflows/all-packages.yml"><img alt="GitHub CI Status" src="https://github.com/dashevo/platform/actions/workflows/all-packages.yml/badge.svg"></a>
  <a href="https://chat.dashdevs.org/"><img alt="Devs Chat" src="https://img.shields.io/badge/discord-Dev_chat-738adb"></a>
  <a href="https://discordapp.com/invite/PXbUxJB"><img alt="General Chat" src="https://img.shields.io/badge/discord-General_chat-738adb"></a>
  <a href="https://twitter.com/intent/follow?screen_name=Dashpay"><img alt="Follow on Twitter" src="https://img.shields.io/twitter/follow/Dashpay.svg?style=social&label=Follow"></a>
</p>

Dash Platform is a technology stack for building decentralized applications on
the Dash network. The two main architectural components, Drive and DAPI, turn
the Dash P2P network into a cloud that developers can integrate with their
applications.

If you are looking for how to contribute to the project or need any help with
building an app on the Dash Platform - message us on the [Devs
Discord](https://chat.dashdevs.org/)!

## Note: Dash Platform is currently available on the Dash Testnet only

## Intro

This is a multi-package repository - sometimes also known as monorepository -
that contains all packages that comprise the Dash platform - for example, Drive,
which is the storage component of Dash Platform, the JavaScript SDK, wallet-lib,
DAPI, and others. Every individual package contains its own readme. Packages are
located in the [packages](./packages) directory.

### Supported networks

Dash Platform is currently undergoing testing and final development necessary to
support its release on the Dash production network (mainnet). The packages in
this repository may be used on the following networks:

- [x] **Development networks** ([**devnets**](https://dashplatform.readme.io/docs/reference-glossary#devnet))
- [x] [**Testnet**](https://dashplatform.readme.io/docs/reference-glossary#testnet)
- [ ] [Mainnet](https://dashplatform.readme.io/docs/reference-glossary#mainnet)

## Install & Build

**Important**: Building the dev environment requires 2GB+ RAM - whatever the OS needs, plus 1.5GB for itself.

1. Clone and enter the repo
   ```bash
   git clone https://github.com/dashevo/platform ./platform/
   pushd ./platform/
   ```
2. Install prerequisites:
  - gcc toolchain
    ```bash
    sudo apt install -y build-essential
    ```
  - [node.js](https://nodejs.org/) v16.10.0+
    ```bash
    curl https://webinstall.dev/node@16 | bash
    ```
  - [docker](https://docs.docker.com/get-docker/) v20.10+
    ```bash
    sudo apt update
    
    sudo sh -eux <<EOF
    apt-get install -y uidmap
    EOF
    
    curl -fsSL https://get.docker.com -o get-docker.sh
    bash get-docker.sh
    
    dockerd-rootless-setuptool.sh install
    ```
    **Important**: follow the on-screen instructions about `DOCKER_HOST`!
3. Enable [corepack](https://nodejs.org/dist/latest/docs/api/corepack.html) to install yarn
   ```bash
   # installs yarn, bundled with node v16+
   corepack enable
   ```
4. Install dependencies and configure and build all packages
   ```bash
   yarn setup
   ```
   
## Run & Test

**Important**: Running a dev environment requires a **non-trivial amount of system resources**. \
(you may wish to stop the local node when not in use)

1. Start the local dev environment built from the sources
   ```bash
   yarn start
   ```
2. Rebuild and restart after changes
   ```bash
   yarn build
   yarn restart
   ```
3. Stop the local node
   ```bash
   yarn stop
   ```

Notes:

- To run the whole test suite):
  ```bash
  # running tests requires a running node 
  #yarn start
  
  # run all tests
  yarn test
  ```
- To run tests for a specific package:
  ```bash
  # yarn workspace <package_name> test
  # Example: run tests for the JS DAPI client
  yarn workspace @dashevo/dapi-client test
  ```
  See [./packages/README.md](./packages/README.md) for the list of available packages.
- To completely reset all local data and builds:
  ```bash
  yarn reset
  ```

## FAQ

### Where can I find support?

For questions and support, please join our [Devs
Discord](https://chat.dashdevs.org/)

### Where are the docs?

Our docs are hosted on
[readme.io](https://dashplatform.readme.io/docs/introduction-what-is-dash-platform).
You can create issues and feature requests in the
[issues](https://github.com/dashevo/platform/issues) for this repository.

### Want to report a bug or request a feature?

Please read through our [CONTRIBUTING.md](CONTRIBUTING.md) and fill out the
issue template at [platform/issues](https://github.com/dashevo/platform/issues)!

### Want to contribute to Dash Platform?

Check out:

- Our [Developers Discord](https://chat.dashdevs.org/)
- Our [CONTRIBUTING.md](CONTRIBUTING.md) to get started with setting up the
  repo.
- Our [news](https://www.dash.org/news/) and [blog](https://www.dash.org/blog/) which contains release posts and
  explanations.

## License

[MIT](LICENSE.md)
