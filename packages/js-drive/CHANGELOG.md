# [0.12.0](https://github.com/dashevo/drive/compare/v0.11.1...v0.12.0) (2020-04-18)

### Features

* publish docker image with tag for every Semver segment ([#332](https://github.com/dashevo/drive/issues/332))
* introduce ABCI and Machine logic, remove API and upgrade to DPP 0.12 ([#328](https://github.com/dashevo/drive/issues/328))
* validate fee, reduce balance and move fees to distribution pool ([#329](https://github.com/dashevo/drive/issues/329))

### BREAKING CHANGES

* JSON RPC and gRPC endpoints are removed. Use Tendermint ABCI query endpoint in order to fetch data
* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.12.0)


## [0.11.1](https://github.com/dashevo/drive/compare/v0.11.0...v0.11.1) (2020-03-17)

### Bug Fixes

* do not validate ST second time in `applyStateTransition` ([d296608](https://github.com/dashevo/drive/commit/d29660886deb7e5556c5346da54506aebc005bfa))
* check for MongoDb replica set on start ([286074f](https://github.com/dashevo/drive/commit/286074fe297bb693ffe7492523e560aeb2512330))

# [0.11.0](https://github.com/dashevo/drive/compare/v0.7.0...v0.11.0) (2020-03-09)

### Bug Fixes

* prevent to update dependencies with major version `0` to minor versions ([9f1dd95](https://github.com/dashevo/drive/commit/9f1dd95fe2294de2d0a3157807eec9598d0f0db7))

### Features

* upgrade DPP to v0.11 ([9797e51](https://github.com/dashevo/drive/commit/9797e51bee6899c07aabcf733fa54650037c42cd))

### Chore

* update gRPC errors ([1d31326](https://github.com/dashevo/drive/commit/1d31326977b2b5f1537426d9d31d89f459aaace6))

### BREAKING CHANGES

* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.11.0)
