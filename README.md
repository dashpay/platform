<p align="center">
  <a href="https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/">
    <img alt="babel" src="https://media.dash.org/wp-content/uploads/dash_digital-cash_logo_2018_rgb_for_screens.png" width="546">
  </a>
</p>

<p align="center">
  Fastest decentralized applications for the Dash network.
</p>

<p align="center">
  <a href="https://github.com/dashevo/platform/actions/workflows/all-packages.yml"><img alt="GitHub CI Status" src="https://github.com/dashevo/platform/actions/workflows/all-packages.yml/badge.svg"></a>
  <a href="https://chat.dashdevs.org/"><img alt="Devs Chat" src="https://img.shields.io/badge/discord-Dev_chat-738adb"></a>
  <a href="https://discordapp.com/invite/PXbUxJB"><img alt="General Chat" src="https://img.shields.io/badge/discord-General_chat-738adb"></a>
  <a href="https://twitter.com/intent/follow?screen_name=Dashpay"><img alt="Follow on Twitter" src="https://img.shields.io/twitter/follow/Dashpay.svg?style=social&label=Follow"></a>
</p>

Dash Platform is a technology stack for building decentralized applications on the Dash network. 
The two main architectural components, Drive and DAPI, turn the Dash P2P network into a cloud that 
developers can integrate with their applications.

If you are looking for how to contribute to the project or need any help with building an app on 
the Dash Platform - message us in [Devs Discord](https://chat.dashdevs.org/)!

## Intro

This is a multi-package repository - sometimes also known as monorepository - that contains
all packages that comprise the Dash platform - for example, Drive, which is the 
storage component of Dash Platform, the JavaScript SDK, wallet-lib, DAPI, and others. 
Every individual package contains its own readme. Packages are located under the
[packages](./packages) directory.

## FAQ

### How to build and set up a node from the code in this repo?

- Clone the repo
- Install prerequisites - nodejs and docker
- Run `npm run setup` - it will install dependencies and configure and build all packages
- Run `npm run start` to start the local dev environment built from the sources
- Run `npm test` to run whole test suite (note that running tests requires a node running 
 so be sure to call `npm run start` first). You also can run tests for a specific package
 by running `npm test -w <package_name>`, for example running 
 `npm test -w @dashevo/dapi-client` will run tests for the JS DAPI client.
- `npm run stop` will stop the local dev env. Running a dev env requires some resources,
 so you should stop the local node when you not need it anymore.
- Run `npm run build` to rebuild project after changes. If you have a local node
 running, you may need to restart by running `npm run stop && npm run start`
- To completely reset all local data and builds, run `npm run reset`

### Looking for support?

For questions and support please join our [Devs Discord](https://chat.dashdevs.org/)

### Where are the docs?

Our docs are hosted on the [readme.io](https://dashplatform.readme.io/docs/introduction-what-is-dash-platform), 
and report issues/features at this repository [issues](https://github.com/dashevo/platform/issues).

### Want to report a bug or request a feature?

Please read through our [CONTRIBUTING.md](CONTRIBUTING.md) and fill 
out the issue template at [platform/issues](https://github.com/dashevo/platform/issues)!

### Want to contribute to Dash Platform?

Check out:

- Our [Developers Discord](https://chat.dashdevs.org/)

Some resources:

- Our [CONTRIBUTING.md](CONTRIBUTING.md) to get started with setting up the repo.
- Our blog which contains release posts and explanations: [/blog](https://www.dash.org/blog/)

### How is the repo structured?

The Dash Platform repo is managed as a that is composed of many [packages](packages/README.md).

## License

[MIT](LICENSE.md)
