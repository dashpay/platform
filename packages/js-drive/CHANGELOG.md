## [0.21.1](https://github.com/dashevo/js-drive/compare/v0.21.0...v0.21.1) (2021-10-28)


### Bug Fixes

* getFeatureFlagForHeight must not try to fetch feature flags before they were created ([#575](https://github.com/dashevo/js-drive/issues/575))



# [0.21.0](https://github.com/dashevo/js-drive/compare/v0.20.0...v0.21.0) (2021-10-14)


### Features

* support higher protocol version ([#571](https://github.com/dashevo/js-drive/issues/571))
* set protocol version on `begin block` ([#558](https://github.com/dashevo/js-drive/issues/558))
* comprehensive error codes ([#564](https://github.com/dashevo/js-drive/issues/564), [#572](https://github.com/dashevo/js-drive/issues/572))
* multiproof for the identity non inclusion proof root tree ([#560](https://github.com/dashevo/js-drive/issues/560))


### Bug Fixes

* consensus logger wasn't set on error ([#567](https://github.com/dashevo/js-drive/issues/567))
* previousRootTree not rebuilt on commit, resulting in a wrong proof ([#563](https://github.com/dashevo/js-drive/issues/563))



# [0.20.0](https://github.com/dashevo/js-drive/compare/v0.19.3...v0.20.0) (2021-07-22)


### Features

* use latest version of Merk storage ([#546](https://github.com/dashevo/js-drive/issues/546))
* remove chainlock SML verification in favor of more robust core verification ([#536](https://github.com/dashevo/js-drive/issues/536))
* remove SML instant lock verification in favor of more robust core verification [#533](https://github.com/dashevo/js-drive/issues/533))
* make compatible with Tenderdash v0.5 ([#527](https://github.com/dashevo/js-drive/issues/527))
* add additional info to proofs ([#518](https://github.com/dashevo/js-drive/issues/518), [#523](https://github.com/dashevo/js-drive/issues/523), [#525](https://github.com/dashevo/js-drive/issues/525), [#540](https://github.com/dashevo/js-drive/issues/540), [#542](https://github.com/dashevo/js-drive/issues/542))
* validator set rotation ([#446](https://github.com/dashevo/js-drive/issues/446), [#515](https://github.com/dashevo/js-drive/issues/515), [#517](https://github.com/dashevo/js-drive/issues/517), [#530](https://github.com/dashevo/js-drive/issues/530), [#531](https://github.com/dashevo/js-drive/issues/531), )


### Bug Fixes

* invalid instant lock if no blocks are produced ([#513](https://github.com/dashevo/js-drive/issues/513))
* `getProofs` method was missing from `PublicKeyToIdentityIdStoreRootTreeLeaf` class ([#544](https://github.com/dashevo/js-drive/issues/544))
* typo in trace output ([#516](https://github.com/dashevo/js-drive/issues/516))


### BREAKING CHANGES

* `document`, `dataContract`, `identity`, `identityIdsByPublicKeyHashes` and `identitiesByPublicKeyHashes` query handlers now returns Protobuf messages instead of cbor'ed data. data is not sent if a proof is requested
* `VALIDATOR_SET_LLMQ_TYPE` env is required
* due to changes in hashing algorithm `appHash` is no longer same and not reproducible for old blocks hence new nodes would not be able to sync
* removing SML IS lock verification make some previously invalid transactions valid
* not compatible with Tenderdash v0.4
* new ABCI messages and types not compatible with previous ones



## [0.19.3](https://github.com/dashevo/js-drive/compare/v0.19.2...v0.19.3) (2021-06-04)


### Bug Fixes

* documents were deleted using wrong id ([#514](https://github.com/dashevo/js-drive/issues/514))



## [0.19.2](https://github.com/dashevo/js-drive/compare/v0.19.1...v0.19.2) (2021-05-25)


### Bug Fixes

* InvalidQuery error due to feature flags ([#512](https://github.com/dashevo/js-drive/issues/512))



## [0.19.1](https://github.com/dashevo/js-drive/compare/v0.19.0...v0.19.1) (2021-05-13)


### Bug Fixes

* feature flags contract height variable has had an invalid name ([#509](https://github.com/dashevo/js-drive/issues/509))



# [0.19.0](https://github.com/dashevo/js-drive/compare/v0.18.1...v0.19.0) (2021-05-05)


### Features

* use Dash Core to verify chain locks ([#503](https://github.com/dashevo/js-drive/issues/503), [#505](https://github.com/dashevo/js-drive/issues/505), [#506](https://github.com/dashevo/js-drive/issues/506))
* verify instant locks using Dash Core ([#499](https://github.com/dashevo/js-drive/issues/499), [#501](https://github.com/dashevo/js-drive/issues/501), [#492](https://github.com/dashevo/js-drive/issues/492), [#498](https://github.com/dashevo/js-drive/issues/498))
* feature flags ([#491](https://github.com/dashevo/js-drive/issues/491), [#504](https://github.com/dashevo/js-drive/issues/504), [#485](https://github.com/dashevo/js-drive/issues/485))
* output Core network on start ([#490](https://github.com/dashevo/js-drive/issues/490))
* update js-dp-services-ctl to 0.19-dev ([#486](https://github.com/dashevo/js-drive/issues/486))
* enable docker build npm cache ([#478](https://github.com/dashevo/js-drive/issues/478))
* do not setup node if SKIP_TEST_SUITE option is set ([#480](https://github.com/dashevo/js-drive/issues/480))
* remove regtest fallbacks ([#477](https://github.com/dashevo/js-drive/issues/477))
* add `verifyInstantLock` in favor of `getSMLStore` method ([#474](https://github.com/dashevo/js-drive/issues/474))


### Bug Fixes

* error loading shared library libzmq.so.5 ([#483](https://github.com/dashevo/js-drive/issues/483))
* blockExecutionContext header might be null ([#481](https://github.com/dashevo/js-drive/issues/481))


### BREAKING CHANGES

* running in standalone regtest mode is not supported anymore
* `fetchSMLStore` method has been removed
* See [DPP v0.19 breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.19.0)


# [0.18.1](https://github.com/dashevo/js-drive/compare/v0.18.0...v0.18.1) (2021-03-08)


### Documentation

* polish changelog ([ac434e](https://github.com/dashevo/dapi/commit/ac434eea9e1588077445ac13a7f4c066a710a3ec))



# [0.18.0](https://github.com/dashevo/js-drive/compare/v0.17.14...v0.18.0) (2021-03-03)


### Bug Fixes

* ABCI request length error still not parsing properly ([#476](https://github.com/dashevo/js-drive/issues/476))


### Features

* output ABCI connection error message ([ff3660a](https://github.com/dashevo/js-drive/commit/ff3660a638f0f4170ff3f7242f84c256e75fa4c6))
* getProofs ABCI query endpoint ([#451](https://github.com/dashevo/js-drive/issues/451), [#462](https://github.com/dashevo/js-drive/issues/462))



## [0.17.14](https://github.com/dashevo/js-drive/compare/v0.18.0-dev.5...v0.17.14) (2021-02-20)


### Bug Fixes

* can't parse ABCI request length error ([f55fac7](https://github.com/dashevo/js-drive/commit/f55fac7aced9334cf26e930cfaf23258cec66a9d))



## [0.17.13](https://github.com/dashevo/js-drive/compare/v0.17.12...v0.17.13) (2021-02-19)


### Features

* reimplemented ABCI server for better reliability ([#475](https://github.com/dashevo/js-drive/issues/475))



## [0.17.12](https://github.com/dashevo/js-drive/compare/v0.17.11...v0.17.12) (2021-02-16)


### Features

* better handle abci connection errors ([f4348e9](https://github.com/dashevo/js-drive/commit/f4348e944825dc9b554eec8dcf7752e972081b2a))



## [0.17.11](https://github.com/dashevo/js-drive/compare/v0.17.8...v0.17.11) (2021-02-16)


### Bug Fixes

* stack overflow due to write on write error ([cb3e0ac](https://github.com/dashevo/js-drive/commit/cb3e0ac4212d95372c2b402496125afdf5e69cea))



## [0.17.10](https://github.com/dashevo/js-drive/compare/v0.17.8...v0.17.10) (2021-02-16)


### Bug Fixes

* abci connection error writes to closed stream ([41a891a](https://github.com/dashevo/js-drive/commit/41a891a922bf2f924c543410dd6d19b3a3ba03d0))



## [0.17.9](https://github.com/dashevo/js-drive/compare/v0.17.8...v0.17.9) (2021-02-15)


### Features

* robust error handling ([#473](https://github.com/dashevo/js-drive/issues/473))
* use a different handler for ABCI connection error ([#465](https://github.com/dashevo/js-drive/issues/465), [b9d452a](https://github.com/dashevo/js-drive/commit/b9d452a20bdf75699fa532eb69af7500fc985045))



## [0.17.8](https://github.com/dashevo/js-drive/compare/v0.17.7...v0.17.8) (2021-02-11)


### Bug Fixes

* could not resolve `previousBlockExecutionStoreTransactions` on query ([#470](https://github.com/dashevo/js-drive/issues/470))


### Features

* add `driveVersion` to every log output ([#469](https://github.com/dashevo/js-drive/issues/469))
* await Node logger stream to be ended ([#471](https://github.com/dashevo/js-drive/issues/471))
* distinguishing log data ([#472](https://github.com/dashevo/js-drive/issues/472))



## [0.17.7](https://github.com/dashevo/js-drive/compare/v0.17.6...v0.17.7) (2021-02-04)


### Features

* disable state repository and merk logging by default ([#467](https://github.com/dashevo/js-drive/issues/467))



## [0.17.6](https://github.com/dashevo/js-drive/compare/v0.17.5...v0.17.6) (2021-01-26)


### Bug Fixes

* only info log level is present in log streams ([#463](https://github.com/dashevo/js-drive/issues/463))



## [0.17.5](https://github.com/dashevo/js-drive/compare/v0.17.4...v0.17.5) (2021-01-21)


### Features

* different logging levels ([#461](https://github.com/dashevo/js-drive/issues/461))


### BREAKING CHANGES

* `LOGGING_LEVEL` is ignored. Use `LOG_STDOUT_LEVEL`.



## [0.17.4](https://github.com/dashevo/js-drive/compare/v0.17.3...v0.17.4) (2021-01-20)


### Bug Fixes

* logger with context is not used in some cases ([#458](https://github.com/dashevo/js-drive/issues/458))
* tx counters and logger were not reset ([#460](https://github.com/dashevo/js-drive/issues/460))


### Features

* log to human-readable and json files ([#459](https://github.com/dashevo/js-drive/issues/459))



## [0.17.3](https://github.com/dashevo/js-drive/compare/v0.17.2...v0.17.3) (2021-01-20)


### Features

* better logging ([#456](https://github.com/dashevo/js-drive/issues/456))



## [0.17.2](https://github.com/dashevo/js-drive/compare/v0.17.1...v0.17.2) (2021-01-19)


### Bug Fixes

* could not resolve 'previousBlockExecutionStoreTransactions' ([5a9dbff](https://github.com/dashevo/js-drive/commit/5a9dbffb05cfb85e6e394ed79538d979eb4a73a7))
* ST isolation leads to non-deterministic results ([#455](https://github.com/dashevo/js-drive/issues/455))
* handle rawChainLockMessage parsing errors ([#454](https://github.com/dashevo/js-drive/issues/454))



## [0.17.1](https://github.com/dashevo/js-drive/compare/v0.17.0...v0.17.1) (2021-01-12)


### Bug Fixes

* duplicate MongoDB index name ([#453](https://github.com/dashevo/js-drive/issues/453))



# [0.17.0](https://github.com/dashevo/js-drive/compare/v0.16.1...v0.17.0) (2020-12-30)


### Features

* introduce `DriveStateRepository#fetchSMLStore` ([#444](https://github.com/dashevo/js-drive/issues/444), [#445](https://github.com/dashevo/js-drive/issues/445))
* update `dashcore-lib` ([#411](https://github.com/dashevo/js-drive/issues/411), [#442](https://github.com/dashevo/js-drive/issues/442), [#443](https://github.com/dashevo/js-drive/issues/443))
* add old zmq client from DAPI ([#439](https://github.com/dashevo/js-drive/issues/439))
* dashpay contract support ([#441](https://github.com/dashevo/js-drive/issues/441))
* change merk to @dashevo/merk
* gracefull shutdown on SIGINT, SIGTERM, SIGQUIT and unhandled errors ([#427](https://github.com/dashevo/js-drive/issues/427))
* handle core chain locked height ([#428](https://github.com/dashevo/js-drive/issues/428))
* implement verify chainlock query handler ([#402](https://github.com/dashevo/js-drive/issues/402))
* intermediate merk tree for the current block ([#429](https://github.com/dashevo/js-drive/issues/429))
* pass latestCoreChainLock on block end ([#434](https://github.com/dashevo/js-drive/issues/434))
* provide proofs for getIdentitiesByPublicKeyHashes endpoint ([#422](https://github.com/dashevo/js-drive/issues/422))
* provide proofs for getIdentitiyIdsByPublicKeyHashes endpoint ([#419](https://github.com/dashevo/js-drive/issues/419))
* provide proofs in ABCI query and DAPI getIdentity ([#415](https://github.com/dashevo/js-drive/issues/415))
* set IDENTITY_SKIP_ASSET_LOCK_CONFIRMATION_VALIDATION to false ([#437](https://github.com/dashevo/js-drive/issues/437))
* sort keys for MerkDB ([#413](https://github.com/dashevo/js-drive/issues/413))
* store ChainInfo in MerkDb ([#404](https://github.com/dashevo/js-drive/issues/404))
* store Data Contracts in merk tree ([#405](https://github.com/dashevo/js-drive/issues/405))
* store documents in MerkDb ([#410](https://github.com/dashevo/js-drive/issues/410))
* store height in externalStorage instead of merkDB ([#433](https://github.com/dashevo/js-drive/issues/433))
* store identities in merk tree ([#400](https://github.com/dashevo/js-drive/issues/400))
* store Public Key to Identity ID in MerkDb ([#409](https://github.com/dashevo/js-drive/issues/409))
* update `dpp` to include asset lock verification logic ([#432](https://github.com/dashevo/js-drive/issues/432))
* introduce merkle forest ([#401](https://github.com/dashevo/js-drive/issues/401))
* move block execution context out of blockchain state ([#403](https://github.com/dashevo/js-drive/issues/403))
* add abstraction for MerkDb ([#407](https://github.com/dashevo/js-drive/issues/407))


### Bug Fixes

* hash was used as a Buffer where it should be hex string ([#440](https://github.com/dashevo/js-drive/issues/440))
* documents DB transaction is already started error ([#417](https://github.com/dashevo/js-drive/issues/417))
* e.getErrors is not a function error ([#418](https://github.com/dashevo/js-drive/issues/418))
* missing nested indexed fields and transaction ([#426](https://github.com/dashevo/js-drive/issues/426))


### BREAKING CHANGES

* AppHash is not equal to nils anymore.
* data created with 0.16 and lower versions of Drive is not compatible anymore
* ABCI query responses are changed



## [0.16.1](https://github.com/dashevo/js-drive/compare/v0.16.0...v0.16.1) (2020-10-29)


### Bug Fixes

* `header` is not present in `RequestEndBlock` ([#399](https://github.com/dashevo/js-drive/issues/399))



# [0.16.0](https://github.com/dashevo/js-drive/compare/v0.15.0...v0.16.0) (2020-10-28)


### Bug Fixes

* incorrect deliver state transition hash logging ([#396](https://github.com/dashevo/js-drive/issues/396))


### Features

* verify DPNS contract existence ([#397](https://github.com/dashevo/js-drive/issues/397))
* add `LoggedStateRepositoryDecorator` ([#393](https://github.com/dashevo/js-drive/issues/393))
* debug mode to respond internal error with message and stack ([#383](https://github.com/dashevo/js-drive/issues/383))
* implement `fetchIdentityIdsByPublicKeys` method ([#385](https://github.com/dashevo/js-drive/issues/385))
* implement `storeIdentityPublicKeyHashes` method ([#387](https://github.com/dashevo/js-drive/issues/387))
* implement getting identities by multiple public keys hashes ([#388](https://github.com/dashevo/js-drive/issues/388), [#395](https://github.com/dashevo/js-drive/issues/395), [#386](https://github.com/dashevo/js-drive/issues/386))
* update DPP to 0.16.0 ([#392](https://github.com/dashevo/js-drive/issues/392))


### Refactoring

* remove unnecessary InvalidDocumentTypeError handling ([#384](https://github.com/dashevo/js-drive/issues/384))


### BREAKING CHANGES

* If `DPNS_CONTRACT_ID` is set it requires `DPNS_CONTRACT_BLOCK_HEIGHT` to be set too.
* See [DPP v0.16 breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.16.0)



# [0.15.0](https://github.com/dashevo/js-drive/compare/v0.14.0...v0.15.0) (2020-09-04)


### Bug Fixes

* internal errors are not logged ([#380](https://github.com/dashevo/js-drive/issues/380))
* unique index throws duplicate key error (#378)


### Features

* handle protocol and software versions ([#377](https://github.com/dashevo/js-drive/issues/377))
* handle user-defined binary fields ([#373](https://github.com/dashevo/js-drive/issues/373), [#381](https://github.com/dashevo/js-drive/issues/381))


### BREAKING CHANGES

* protocol version (`AppVersion`) is required in a Tendermint block header
* the previous state is not compatible due to new DPP serialization format
* See [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.15.0)



# [0.14.0](https://github.com/dashevo/drive/compare/v0.13.2...v0.14.0) (2020-07-23)


### Features

* increase MongoDB query allowed field length ([#366](https://github.com/dashevo/drive/issues/366))
* logging of block execution process ([#365](https://github.com/dashevo/drive/issues/365))
* use test suite to run functional and e2e tests ([#362](https://github.com/dashevo/drive/issues/362))
* update to DPP v0.14 with timestamps ([#363](https://github.com/dashevo/drive/issues/363))


### BREAKING CHANGES

* See [DPP v0.14 breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.14.0)



## [0.13.2](https://github.com/dashevo/drive/compare/v0.13.0-dev.2...v0.13.2) (2020-06-12)


### Bug Fixes

* internal errors lead to inability to fix bugs as it leads to a state inconsistency ([#360](https://github.com/dashevo/drive/issues/360))



## [0.13.1](https://github.com/dashevo/drive/compare/v0.13.0...v0.13.1) (2020-06-12)


### Bug Fixes

* document repository not created properly due to missing `await` ([#358](https://github.com/dashevo/drive/issues/358))



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
