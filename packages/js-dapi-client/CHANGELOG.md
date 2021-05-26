## [0.19.2](https://github.com/dashevo/dapi-client/compare/v0.19.1...v0.19.2) (2021-05-20)


### Chores

* Update DPP to a 0.19.2 ([#237](https://github.com/dashevo/dapi-client/issues/237))



## [0.19.1](https://github.com/dashevo/dapi-client/compare/v0.19.0...v0.19.1) (2021-05-04)


### Bug Fixes

* `getStatus` reponse was returning buffers as Base64 encoded strings ([#235](https://github.com/dashevo/dapi-client/issues/235))



# [0.19.0](https://github.com/dashevo/dapi-client/compare/v0.18.0...v0.19.0) (2021-05-03)


### Features

* `getStatus` method handler response updated ([#229](https://github.com/dashevo/dapi-client/issues/229), [#230](https://github.com/dashevo/dapi-client/issues/230))


### Bug Fixes

* critical security vulnerability in axios@0.19.2 ([#233](https://github.com/dashevo/dapi-client/issues/233))


### BREAKING CHANGES

* `getStatus` method handler response have changed



# [0.18.0](https://github.com/dashevo/dapi-client/compare/v0.17.2...v0.18.0) (2021-03-03)


### Bug Fixes

* BLS was throwing an error inside `uncaughtException` handler ([#226](https://github.com/dashevo/dapi-client/issues/226))


### Features


* `waitForStateTransitionResult` method ([#221](https://github.com/dashevo/dapi-client/issues/221), ([#223](https://github.com/dashevo/dapi-client/issues/223))


### Chores

* remove temporary `timeout` option from `broadcastStateTransition` ([2abdf5](https://github.com/dashevo/js-dapi-client/commit/2abdf5dc45859bc142dd0f293d9f071190f2a59d))



## [0.17.2](https://github.com/dashevo/dapi-client/compare/v0.17.1...v0.17.2) (2020-12-30)


### Bug Fixes

* broadcastStateTransitions is timing out on testnet ([#219](https://github.com/dashevo/dapi-client/issues/219))



## [0.17.1](https://github.com/dashevo/dapi-client/compare/v0.17.0...v0.17.1) (2020-12-30)


### Bug Fixes

* merkleRootQuorums from the diff doesn’t match calculated quorum root after diff is applied ([#217](https://github.com/dashevo/dapi-client/issues/217))



# [0.17.0](https://github.com/dashevo/dapi-client/compare/v0.16.0...v0.17.0) (2020-12-29)


### Features

* introduce testnet network ([#214](https://github.com/dashevo/dapi-client/issues/214))
* update `dpp` and `dashcore-lib` ([#207](https://github.com/dashevo/dapi-client/issues/207), [#210](https://github.com/dashevo/dapi-client/issues/210), [#211](https://github.com/dashevo/dapi-client/issues/211), [#212](https://github.com/dashevo/dapi-client/issues/212))


### Bug Fixes

* SML unhandled error on blockchain reorg ([#215](https://github.com/dashevo/dapi-client/issues/215))


### BREAKING CHANGES

* DAPI client is now connecting to a testnet by default



# [0.16.0](https://github.com/dashevo/dapi-client/compare/v0.15.0...v0.16.0) (2020-10-27)


### Features

* `getIdentitiesByPublicKeyHashes` and `getIdentityIdsByPublicKeyHashes` methods ([#191](https://github.com/dashevo/dapi-client/issues/191), [#196](https://github.com/dashevo/dapi-client/issues/196), [#205](https://github.com/dashevo/dapi-client/issues/205))
* `getDataContract`, `getDocuments`, `getIdentity` accept `Buffer` ([#201](https://github.com/dashevo/dapi-client/issues/201))


### Documentation

* fix URLs in README ([#193](https://github.com/dashevo/dapi-client/issues/193))


### BREAKING CHANGES

* `getIdentityByFirstPublicKey` and `getIdentityIdByFirstPublicKey` removed
* `getDataContract`, `getDocuments`, `getIdentity` accept `Buffer` or `TypedArray`



# [0.15.0](https://github.com/dashevo/dapi-client/compare/v0.14.0...v0.15.0) (2020-09-04)


### Bug Fixes

* internal error when submitting `fromBlockHeight` as `0` to `subscribeToTransactionsWithProofs` ([#174](https://github.com/dashevo/js-dapi-client/pull/174))


### Features

* retry request on `UNIMPLEMENTED` error ([#185](https://github.com/dashevo/dapi-client/issues/185))
* update DAPI gRPC to 0.15 ([#179](https://github.com/dashevo/dapi-client/issues/179), [#186](https://github.com/dashevo/dapi-client/issues/186))
* remove `getUTXO` and `getAddressSummary` core methods ([#178](https://github.com/dashevo/js-dapi-client/pull/178))
* rename `sendTransaction` and `applyStateTransition` ([#175](https://github.com/dashevo/js-dapi-client/pull/175))


### BREAKING CHANGES

* `broadcastTransaction` and `broadcastStatTransition` gRPC method names are using instead of `sendTransaction` and `applyStateTransition`
* `getUTXO` and `getAddressSummary` core methods are removed
* see [DAPI gRPC breaking changes](https://github.com/dashevo/dapi-grpc/releases/tag/v0.15.0)



# [0.14.0](https://github.com/dashevo/dapi-client/compare/v0.13.6...v0.14.0) (2020-07-23)

We completely rewrote DAPI Client to improve code quality, usability, and testability.

In the new version, you can specify not just seeds to connect but also specific DAPI addresses
and even inject own logic to obtain/select nodes. API methods accept the same options
as the `DAPIClient` constructor so you can specify different behavior for each API call.

Previously, faulty nodes were excluded for a specific API call. Now they are banning
for a period of time, and this time increments exponentially in the event of repeated faults.


### Bug Fixes

* cannot read property 'getHttpPort' of undefined ([#173](https://github.com/dashevo/dapi-client/issues/173))
* internal error when submitting `fromBlockHeight` as 0 to `subscribeToTransactionsWithProofs` ([#174](https://github.com/dashevo/dapi-client/issues/174))
* ambiguity in `addresses` option ([#170](https://github.com/dashevo/dapi-client/issues/170))
* JSON RPC does not retry on `ETIMEDOUT` ([#156](https://github.com/dashevo/dapi-client/issues/156))
* 2 seconds timeout not enough for some requests ([#151](https://github.com/dashevo/dapi-client/issues/151))
* construct DAPIClient with network option didn't work properly ([#150](https://github.com/dashevo/dapi-client/issues/150))
* global default timeout applies for streams ([#152](https://github.com/dashevo/dapi-client/issues/152))


### Features

* add ports to string representation of DAPIAddress ([#171](https://github.com/dashevo/dapi-client/issues/171)) ([1f4ffb7](https://github.com/dashevo/dapi-client/commit/1f4ffb7ed2cd8079eccf938ede2d43f37a5f80d3))
* allow to specify network with other connection options ([#160](https://github.com/dashevo/dapi-client/issues/160)) ([cfbc5cd](https://github.com/dashevo/dapi-client/commit/cfbc5cd649358420df99f76f1ca84b8c7ae826a4))
* update DAPI gRPC to 0.14.0-dev.1 ([#149](https://github.com/dashevo/dapi-client/issues/149)) ([4598def](https://github.com/dashevo/dapi-client/commit/4598def13dbdba9c9c1392c65e2c97ceb322c34c))
* timeout options for gRPC requests and simplified URL for gRPC client ([#146](https://github.com/dashevo/dapi-client/issues/146)) ([35685b9](https://github.com/dashevo/dapi-client/commit/35685b98fa05fc4436630f165113419b3f48833f))


### Documentation

* readme standard updates ([#167](https://github.com/dashevo/dapi-client/issues/147))


### Code Refactoring

* rewrite DAPI Client from scratch ([#140](https://github.com/dashevo/dapi-client/issues/140))


### BREAKING CHANGES

* DAPI Client options [are changed](https://github.com/dashevo/dapi-client/blob/1ec21652f1615ba95ea537c38632692f81deefa3/lib/DAPIClient.js#L42-L51)
* Core and Platform methods moved to specific namespaces (ie. `client.platform.getIdentity()`, `client.core.getStatus()`)



## [0.13.6](https://github.com/dashevo/dapi-client/compare/v0.13.5...v0.13.6) (2020-06-30)


### Features

* update dapi-client to `0.18.11` ([#163](https://github.com/dashevo/dapi-client/issues/163))



## [0.13.5](https://github.com/dashevo/dapi-client/compare/v0.13.4...v0.13.5) (2020-06-30)


### Features

* update `dashcore-lib` to `0.18.10` ([#162](https://github.com/dashevo/dapi-client/issues/162))



## [0.13.4](https://github.com/dashevo/dapi-client/compare/v0.13.3...v0.13.4) (2020-06-30)


### Bug Fixes

* network is not set to `SimplifiedMNListDiff` ([#161](https://github.com/dashevo/dapi-client/issues/161))



## [0.13.3](https://github.com/dashevo/dapi-client/compare/v0.13.2...v0.13.3) (2020-06-18)


### Bug Fixes

* calling method `getIp` of `undefined` ([#159](https://github.com/dashevo/dapi-client/issues/159))



## [0.13.2](https://github.com/dashevo/dapi-client/compare/v0.13.1...v0.13.2) (2020-06-11)


### Bug Fixes

* retries don't work for MN discovery ([#157](https://github.com/dashevo/dapi-client/issues/157))



## [0.13.1](https://github.com/dashevo/dapi-client/compare/v0.13.0...v0.13.1) (2020-06-11)


### Bug Fixes

* JSON RPC doesn't retry on `ETIMEDOUT ([#155](https://github.com/dashevo/dapi-client/issues/155))



# [0.13.0](https://github.com/dashevo/dapi-client/compare/v0.12.0...v0.13.0) (2020-06-08)


### Bug Fixes

* missed grpc-common peer dependency caused error ([#135](https://github.com/dashevo/dapi-client/pull/135))


## Features

* implement transports with retries ([#130](https://github.com/dashevo/dapi-client/pull/130), [#141](https://github.com/dashevo/dapi-client/pull/141))
* get identity by public key endpoints ([#133](https://github.com/dashevo/dapi-client/pull/133))


### Documentation

* add typing ([#143](https://github.com/dashevo/dapi-client/pull/143))
* JSDoc formatting ([#132](https://github.com/dashevo/dapi-client/pull/132))
* `subscribeToTransactionsWithProofs` ([#137](https://github.com/dashevo/dapi-client/pull/137))



# [0.12.0](https://github.com/dashevo/dapi-client/compare/v0.11.0...v0.12.0) (2020-04-20)


### Code Refactoring

* remove `forceJsonRpc` option ([#126](https://github.com/dashevo/dapi-client/issues/126))


### BREAKING CHANGES

* platform methods no longer available through JSON RPC


# [0.11.0](https://github.com/dashevo/dapi-client/compare/v0.8.0...v0.11.0) (2020-03-01)

### Bug Fixes

* return null if get "Not Found" gRPC error ([86af3f7](https://github.com/dashevo/dapi-client/commit/86af3f78d26e45dbe9ae1d49b6c215f5af9d0cba))
* gRPC web connection url should contain protocol ([0c7ad1f](https://github.com/dashevo/dapi-client/commit/0c7ad1f13ac1ec75a319c97514f19671f48c2b66))


### Features

* introduce `generateToAddress` endpoint ([f8b446b](https://github.com/dashevo/dapi-client/commit/f8b446ba41b0794b2d2007b0ad79e29f4a561b8e))
* bring back `getAddressSummary` endpoint ([d6de22c](https://github.com/dashevo/dapi-client/commit/d6de22cf8cbeb0ac7bb55ec5ae9e09f9900e3028))
* implement basic Core gRPC endpoints ([6fe4d4a](https://github.com/dashevo/dapi-client/commit/6fe4d4a79bce750210672ee7f2df9cc14d4437fd))
* remove obsolete API endpoints and code ([982a514](https://github.com/dashevo/dapi-client/commit/982a51437b94b3cb6ae0ba1b9031daef0a468940))


### BREAKING CHANGES

* Removed unsupported `generate` endpoint
* Removed insecure endpoints
