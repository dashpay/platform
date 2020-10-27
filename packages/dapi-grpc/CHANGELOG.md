# [0.16.0](https://github.com/dashevo/dapi-grpc/compare/v0.15.0...v0.16.0) (2020-10-26)


### Bug Fixes

* protobuf converts empty Buffer to undefined ([#94](https://github.com/dashevo/dapi-grpc/issues/94)) ([cbeacd8](https://github.com/dashevo/dapi-grpc/commit/cbeacd832f843fc955254c1cddfd3454969ee209))


### Features

* add `getIdentitiesByPublicKeyHashes` and `getIdentityIdsByPublicKeyHashes` to platform service ([#89](https://github.com/dashevo/dapi-grpc/issues/89), [#92](https://github.com/dashevo/dapi-grpc/issues/92))
* use bytes for identifiers ([#91](https://github.com/dashevo/dapi-grpc/issues/91))


### BREAKING CHANGES

* `getIdentityIdByFirstPublicKey` and `getIdentityByFirstPublicKey` removed
* `GetDataContractRequest`, `GetDocumentsRequest`, `GetIdentityRequest` now accepts bytes



# [0.15.0](https://github.com/dashevo/dapi-grpc/compare/v0.14.0...v0.15.0) (2020-09-04)


### Features

* build version specific clients ([#85](https://github.com/dashevo/dapi-grpc/issues/86), [#86](https://github.com/dashevo/dapi-grpc/issues/86))
* combine `Core` and `TxFilterStream` services ([#84]((https://github.com/dashevo/dapi-grpc/issues/84)))
* update gRPC-Web to 1.2.0 version ([#83](https://github.com/dashevo/dapi-grpc/issues/83))


### BREAKING CHANGES

* paths to generated clients are changed
* `TxFilterStream` is removed. `subscribeToTransactionsWithProofs` included in Core service.



# [0.14.0](https://github.com/dashevo/dapi-grpc/compare/v0.13.0...v0.14.0) (2020-07-22)


### Features

* allow passing of options to calls in NodeJS clients ([#74](https://github.com/dashevo/dapi-grpc/issues/74))
* strip URL passed on to client and leave only ip/host:port pair ([#75](https://github.com/dashevo/dapi-grpc/issues/75))


### Refactoring

* refactor: remove java artifacts ([#78](https://github.com/dashevo/dapi-grpc/issues/78))


### Tests

* update Mocha config ([#77](https://github.com/dashevo/dapi-grpc/issues/77))



# [0.13.0](https://github.com/dashevo/dapi-grpc/compare/v0.12.1...v0.13.0) (2020-06-08)


### Features

* get identity by public key endpoints ([#71](https://github.com/dashevo/dapi-grpc/issues/71))
* add python to the list of clients generated ([#60](https://github.com/dashevo/dapi-grpc/issues/60))
* use protocol version interceptor in JS clients ([#63](https://github.com/dashevo/dapi-grpc/issues/63), [#68](https://github.com/dashevo/dapi-grpc/issues/68))



## [0.12.1](https://github.com/dashevo/dapi-grpc/compare/v0.12.0...v0.12.1) (2020-02-13)


### Bug Fixes

* namespacing of the `platform` service in the build ([#57](https://github.com/dashevo/dapi-grpc/issues/57)) ([2b22219](https://github.com/dashevo/dapi-grpc/commit/2b22219d319588413058f11e800a9603c0ee7a0c))



# [0.12.0](https://github.com/dashevo/dapi-grpc/compare/v0.11.0...v0.12.0) (2020-01-27)


### Bug Fixes

* core services ([1fde938](https://github.com/dashevo/dapi-grpc/commit/1fde938b2c48c9f79555203af1c615ff82b83ac5))
* platform bugs ([210cdd7](https://github.com/dashevo/dapi-grpc/commit/210cdd7709c009c0303d50c98089f22f8b96ebd8))


### Features

* add more methods to Core service ([41f3ad0](https://github.com/dashevo/dapi-grpc/commit/41f3ad0ad6aee3acf4b1760949cde36d8df7d6f2))
* fetchIdentity endpoint ([75d32d8](https://github.com/dashevo/dapi-grpc/commit/75d32d883be4d7a113fe34f1d008e1d9bcc3c7e1))
* introduce Platform service ([c88b891](https://github.com/dashevo/dapi-grpc/commit/c88b891ecfac8987cd76c773b2f783ad7a155540))



