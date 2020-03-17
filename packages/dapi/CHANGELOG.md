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
