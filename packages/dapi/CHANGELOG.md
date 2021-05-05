# [0.19.0](https://github.com/dashevo/dapi/compare/v0.18.1...v0.19.0) (2021-05-05)


### Features

* enable Docker build npm cache ([#348](https://github.com/dashevo/dapi/issues/348))
* remove insight API ([#351](https://github.com/dashevo/dapi/issues/351), [#344](https://github.com/dashevo/dapi/issues/344), [#345](https://github.com/dashevo/dapi/issues/345), [#346](https://github.com/dashevo/dapi/issues/346), [#345](https://github.com/dashevo/dapi/issues/345), [#347](https://github.com/dashevo/dapi/issues/347), [#362](https://github.com/dashevo/dapi/issues/362))


### Bug Fixes

* error loading shared library libzmq.so.5 ([51e66f7](https://github.com/dashevo/dapi/commit/51e66f76c9fbdecef000fc6acd5d2ab5dd20f01d))


### BREAKING CHANGES

* `getStatus` response format is changed and is not compatible with older version



# [0.18.1](https://github.com/dashevo/dapi/compare/v0.18.0...v0.18.1) (2021-03-08)


### Chores

* update dependencies to stable versions ([cb9070](https://github.com/dashevo/dapi/commit/cb9070a2e58c66eb24a16d01e5d9c28bb9eef95d))



# [0.18.0](https://github.com/dashevo/dapi/compare/v0.17.1...v0.18.0) (2021-03-03)


### Features

* handle Unavailable ABCI error ([#337](https://github.com/dashevo/dapi/issues/337))
* `waitForStateTransitionResult` endpoint ([#331](https://github.com/dashevo/dapi/issues/331), [#338](https://github.com/dashevo/dapi/issues/338), [#340](https://github.com/dashevo/dapi/issues/340), [#341](https://github.com/dashevo/dapi/issues/341))
* replace `broadcast_tx_commit` with `broadcast_tx_sync` ([#330](https://github.com/dashevo/dapi/issues/330))


### BREAKING CHANGES

* `broadcastStateTransition` doesn't wait for state transition commit. Use `waitForStateTransitionResult` to get ST acknowledgment.



## [0.17.1](https://github.com/dashevo/dapi/compare/v0.17.0...v0.17.1) (2021-01-19)


### Bug Fixes

* **core:** timeOffset from Insight expected to be uint32 ([#332](https://github.com/dashevo/dapi/issues/332))



# [0.17.0](https://github.com/dashevo/dapi/compare/v0.16.2...v0.17.0) (2020-12-30)


### Bug Fixes

* internal error if state transaction was broadcasted twice ([#328](https://github.com/dashevo/dapi/issues/328))


### Features

* provide state tree proofs ([#323](https://github.com/dashevo/dapi/issues/323))
* add instant send locks to the transaction stream ([#318](https://github.com/dashevo/dapi/issues/318), [#327](https://github.com/dashevo/dapi/issues/327))
* use new drive response format ([#316](https://github.com/dashevo/dapi/issues/316))
* update dashcore-lib to 0.19.5 ([#312](https://github.com/dashevo/dapi/issues/312))



## [0.16.2](https://github.com/dashevo/dapi/compare/v0.16.1...v0.16.2) (2020-12-21)


### Bug Fixes

* crash in dapi-tx-filter-stream "can't read property trim() of undefined"



## [0.16.1](https://github.com/dashevo/dapi/compare/v0.16.0...v0.16.1) (2020-11-16)


### Bug Fixes

* `count too big` being thrown in `subscribeToTransactionsWithProofsHandler` ([#315](https://github.com/dashevo/dapi/issues/315))



# [0.16.0](https://github.com/dashevo/dapi/compare/v0.15.0...v0.16.0) (2020-10-27)


### Features

* `getIdentitiesByPublicKeyHashes` and `getIdentityIdsByPublicKeyHashes` endpoints ([#304](https://github.com/dashevo/dapi/issues/304), [#307](https://github.com/dashevo/dapi/issues/307))
* debug mode to respond internal error with message and stack ([#302](https://github.com/dashevo/dapi/issues/302))
* use Drive 0.16 endpoints ([#308](https://github.com/dashevo/dapi/issues/308), [#309](https://github.com/dashevo/dapi/issues/309))

### BREAKING CHANGES

* `getIdentityByFirstPublicKey` and `getIdentityIdByFirstPublicKey` removed



# [0.15.0](https://github.com/dashevo/dapi/compare/v0.14.0...v0.15.0) (2020-09-04)


### Features

* update to DAPI gRPC 0.15 ([#298](https://github.com/dashevo/dapi/issues/298))
* remove getUTXO & getAddressSummary rpc methods ([#292](https://github.com/dashevo/dapi/issues/292), [#293](https://github.com/dashevo/dapi/issues/293))
* rename sendTransaction and applyStateTransition to broadcast ([#287](https://github.com/dashevo/dapi/pull/287))


### BREAKING CHANGES

* `broadcastTransaction` and `broadcastStatTransition` gRPC method names are using instead of `sendTransaction` and `applyStateTransition`
* TxFilterStream `subscribeToTransactionsWithProofs` endpoint uses `Core` gRPC service
* see [DAPI gRPC breaking changes](https://github.com/dashevo/dapi-grpc/releases/tag/v0.15.0)



# [0.14.0](https://github.com/dashevo/dapi/compare/v0.13.0...v0.14.0) (2020-07-23)

### Bug Fixes

* internal error when `fromBlockHeight` submitted as 0  to `subscribeToTransactionsWithProofs` ([#285](https://github.com/dashevo/dapi/issues/285))


### Features

* update dependencies (dpp to 0.14.0, dashcore-lib to 0.18.11) ([#283](https://github.com/dashevo/dapi/issues/283))
* reduce artifical slowdown of the transaction stream ([#275](https://github.com/dashevo/dapi/issues/275))
* use test-suite to run functional tests ([#276](https://github.com/dashevo/dapi/issues/276), [#280](https://github.com/dashevo/dapi/issues/280))



# [0.13.0](https://github.com/dashevo/dapi/compare/v0.12.0...v0.13.0) (2020-06-08)


### Bug Fixes

* invalid JSON RPC internal error code ([#271](https://github.com/dashevo/dapi/pull/271))
* incorrect behaviour on undefined data in `handleAbciResponseError` ([#265](https://github.com/dashevo/dapi/pull/265))


### Features

* get identity by public key endpoints ([#263](https://github.com/dashevo/dapi/pull/263), [#266](https://github.com/dashevo/dapi/pull/266))


### Tests

* identity topup functional test ([#268](https://github.com/dashevo/dapi/pull/268))
* functional for validating public key uniqueness ([#269](https://github.com/dashevo/dapi/pull/269))


### Code Refactoring

* actualize drive env variables ([#270](https://github.com/dashevo/dapi/pull/270))


### BREAKING CHANGES

* previously internal errors were respond with wrong error code `-32602` (invalid argument). The error code is changed
 to `-32603` (internal error).
* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.13.0)


# [0.12.0](https://github.com/dashevo/dapi/compare/v0.11.1...v0.12.0) (2020-04-18)

### Bug Fixes

* in case of `Timed out waiting for tx to be included in a block` DAPI responds with Internal error ([#258](https://github.com/dashevo/dapi/issues/258))

### Code Refactoring

* remove Platform JSON RPC endpoints ([#256](https://github.com/dashevo/dapi/issues/256))
* rename `TENDERMINT_CORE_...` envs to `TENDERMINT_RPC_...` ([98c6ad0](https://github.com/dashevo/dapi/commit/98c6ad02c1f8cf2ad76f30bec052f9a1f6eac34f))
* remove rate limiter errors ([#254]((https://github.com/dashevo/dapi/issues/254)))

### Features

* handle insufficient funds ABCI error ([#257](https://github.com/dashevo/dapi/issues/257))
* update deploy script to tag image for every Semver segment ([#260](https://github.com/dashevo/dapi/issues/260))
* update according to merge of Drive and Machine ([#255](https://github.com/dashevo/dapi/issues/255), [#259](https://github.com/dashevo/dapi/issues/259))

### BREAKING CHANGES

* `fetchDocuments`, `fetchDataContract`, `fetchIdentity`, `applyStateTransition` JSON RPC endpoints are removed. Use gRPC analogues.
* rename `TENDERMINT_CORE_...` envs to `TENDERMINT_RPC_...`
* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.12.0)


## [0.11.1](https://github.com/dashevo/dapi/compare/v0.11.0...v0.11.1) (2020-03-17)

### Bug Fixes

* throw correct JSON RPC error on invalid Insight params (#252, [52b1276](https://github.com/dashevo/dapi/commit/52b12765b2a369099d7700bdb077a9d6454d99b5))


# [0.11.0](https://github.com/dashevo/dapi/compare/v0.9.0...v0.11.0) (2020-03-09)

### Bug Fixes

* Core gRPC service is not initialized ([86dff35](https://github.com/dashevo/dapi/commit/86dff354415669e206e543b3b83704eaf62ceb32))
* load .env at correct time for tx-filter-stream ([7b091e0](https://github.com/dashevo/dapi/commit/7b091e0cefcd7d6c63829bd6229a0c3e8d4b692f))
* prevent to update dependencies with major version `0` to minor versions ([ea7de93](https://github.com/dashevo/js-dpp/commit/ea7de9379a38b856f4a7b779786986afacd75b0d))
* handle errors in `getTransaction` endpoints ([e0d36ae](https://github.com/dashevo/dapi/commit/e0d36aebc717f67e90fc44a2256007031ab2f9ba))
* handle errors in `sendTransaction` endpoint ([cd2e6c8](https://github.com/dashevo/dapi/commit/cd2e6c821b7e6822c4b582c758eeeae26627b173))
* handle errors in `getBlock` endpoint ([6d474b4](https://github.com/dashevo/dapi/commit/6d474b46edf5b98f2424b6e20836a6296b5a413e))
* handle rate, time and resource limit ABCI errors ([4c979a3](https://github.com/dashevo/dapi/commit/4c979a3044bc025352962b35292fceedd2d3e7c9))
* handle Tendermint errors in applyStateTransition ([f8764e9](https://github.com/dashevo/dapi/commit/f8764e901c09445e66319fc5d2ff7cf8bc0dd7da))
* "not found" instead of "invalid argument" in gRPC endpoints ([126c929](https://github.com/dashevo/dapi/commit/126c92905d63e2b63f9949d3c58d3a469e680201))


### Features

* remove insecure API endpoints and code ([11b3df3](https://github.com/dashevo/dapi/commit/11b3df3c3dd0fef9d892320f35745b1b68b5b66c))
* introduce `generateToAddress` endpoint ([3a2f497](https://github.com/dashevo/dapi/commit/3a2f49737f5cc75c02a3abffb64b2060b14beb39))
* upgrade DPP to 0.11 ([3b36078](https://github.com/dashevo/dapi/commit/3b360787697d9cfb7f5088058cf11ea12a516c50))


### Tests

* functional test for `getStatus` endpoint ([3f3ec06](https://github.com/dashevo/dapi/commit/3f3ec0606c3a2b6875fa40c17943ac080bc945eb))
* forced json rpc client tests ([5259535](https://github.com/dashevo/dapi/commit/52595357bef4ee0c0ed9d704a2232cfa59b9a11c))


### BREAKING CHANGES

* A ton of insecure endpoints were removed so it's easier to list what left.
    * JSON RPC (deprecated)
        * `generateToAddress`
        * `getAddressSummary`
        * `getBestBlockHash`
        * `getBlockHash`
        * `getMnListDiff`
        * `getUTXO`
    * Core gRPC
        * `subscribeToTransactionsWithProofs`
        * `getBlock`
        * `getStatus`
        * `getTransaction`
        * `sendTransaction`
    * Platform gRPC
        * `applyStateTransition`
        * `getDataContract`
        * `getDocuments`
        * `getIdentity`
* see [DPP breaking changes](https://github.com/dashevo/js-dpp/releases/tag/v0.11.0)
