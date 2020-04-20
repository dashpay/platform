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
