# [0.13.0](https://github.com/dashevo/drive/compare/v0.12.1...v0.13.0) (2020-06-08)


### Features

* update to DPP 0.13 ([#336](https://github.com/dashevo/drive/issues/336), [#338](https://github.com/dashevo/drive/issues/338), [#340](https://github.com/dashevo/drive/issues/340), [#344](https://github.com/dashevo/drive/issues/344), [#346](https://github.com/dashevo/drive/issues/346), [#348](https://github.com/dashevo/drive/issues/348), [#354](https://github.com/dashevo/drive/issues/354), [#357](https://github.com/dashevo/drive/issues/357))
* wait mongoDB replica set initialization ([#349](https://github.com/dashevo/drive/issues/349))
* wait for Core to be synced before starting ([#345](https://github.com/dashevo/drive/issues/345), [#353](https://github.com/dashevo/drive/issues/353), [#356](https://github.com/dashevo/drive/issues/356))
* get identity by public key endpoints ([#341](https://github.com/dashevo/drive/issues/341))
* store identity id with identity's public key as a DB key ([#337](https://github.com/dashevo/drive/issues/337), [#339](https://github.com/dashevo/drive/issues/339))


### Code Refactoring

* use async function with cache to connect and get `MongoClient` ([#350](https://github.com/dashevo/drive/issues/350))


### BREAKING CHANGES

* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.13.0)



## [0.12.2](https://github.com/dashevo/drive/compare/v0.12.1...v0.12.2) (2020-05-21)


### Bug Fixes

* validateFee error handling expects only BalanceIsNotEnoughError ([#343](https://github.com/dashevo/drive/issues/343))



## [0.12.1](https://github.com/dashevo/drive/compare/v0.12.0...v0.12.1) (2020-04-22)


### Features

* update `dpp` version to `0.12.1` ([#335](https://github.com/dashevo/drive/issues/335))


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
