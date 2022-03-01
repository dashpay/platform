## [0.22.0-dev.8](https://github.com/dashevo/platform/compare/v0.21.8...v0.22.0-dev.8) (2022-03-01)


### ⚠ BREAKING CHANGES

* **dpp:** do not allow to index array properties (#225)

### Features

* **dapi-client:** get and verify block headers with dash-spv ([#211](https://github.com/dashevo/platform/issues/211))
* **dapi-client:** handle asynchronous errors ([#233](https://github.com/dashevo/platform/issues/233))
* **dashmate:** add an ability to configure node subnet mask ([#237](https://github.com/dashevo/platform/issues/237))
* **dpp:** allow using BLS key to sign state transitions ([#268](https://github.com/dashevo/platform/issues/268))
* **dpp:** do not allow to index array properties ([#225](https://github.com/dashevo/platform/issues/225))
* **drive:** create/update identities based on SML changes ([#170](https://github.com/dashevo/platform/issues/170))
* integrate RS Drive and GroveDB ([#177](https://github.com/dashevo/platform/issues/177))


### Bug Fixes

* `group:status` command was missing a `format` flag ([#262](https://github.com/dashevo/platform/issues/262))
* `startAt` and `startAfter` invalid decoding ([#255](https://github.com/dashevo/platform/issues/255))
* **build:** `zeromq` build is not working on linux ([#236](https://github.com/dashevo/platform/issues/236))
* cannot install `protobufjs` in some cases ([#266](https://github.com/dashevo/platform/issues/266))
* **dashmate:** `rimraf` module could not remove config directory ([#248](https://github.com/dashevo/platform/issues/248))
* **dashmate:** logs were incorrectly mounted ([#261](https://github.com/dashevo/platform/issues/261))
* **dpp:** Identity public key `readOnly` flag was read as `undefined` instead of `false` ([#239](https://github.com/dashevo/platform/issues/239))
* **drive:** unable to reconstruct SML ([#257](https://github.com/dashevo/platform/issues/257))
* invalid query errors are fatal ([#259](https://github.com/dashevo/platform/issues/259))
* **sdk:** can't update cached data contract ([#223](https://github.com/dashevo/platform/issues/223))


### Documentation

* ignore folder with empty docs during build ([#212](https://github.com/dashevo/platform/issues/212))


### Build System

* `protobufjs` isn't installing from git sometimes ([#267](https://github.com/dashevo/platform/issues/267))


### Miscellaneous Chores

* **dashmate:** update Core to 0.18.0.0-beta4 ([#269](https://github.com/dashevo/platform/issues/269))
* **release:** revert version back
* update tenderdash and core images ([#252](https://github.com/dashevo/platform/issues/252))

## [0.22.0-dev.7](https://github.com/dashevo/platform/compare/v0.21.7...v0.22.0-dev.7) (2022-01-21)


### Features

* added WalletStore ([#197](https://github.com/dashevo/platform/issues/197))
* **drive:** allow using `in` and `startsWith` only in last `where` condition ([#201](https://github.com/dashevo/platform/issues/201))
* **drive:** allow using `orderBy` for fields having `in` and `startsWith` in last `where` clause ([#199](https://github.com/dashevo/platform/issues/199))
* register system contracts on `initChain` ([#182](https://github.com/dashevo/platform/issues/182))
* **wallet-lib:** ChainStore ([#196](https://github.com/dashevo/platform/issues/196))


### Bug Fixes

* **sdk:** system contract ids were hardcoded in SDKs Client module ([#192](https://github.com/dashevo/platform/issues/192))


### Build System

* fix configure test suite script for grep 2.5.1 ([#187](https://github.com/dashevo/platform/issues/187))


### Miscellaneous Chores

* **dashmate:** update tenderdash to 0.7.0-dev ([#188](https://github.com/dashevo/platform/issues/188))
* **release:** update changelog and bump version to 0.22.0-dev.7 ([#209](https://github.com/dashevo/platform/issues/209))
* remove `fixCumulativeFeesBug` feature flag ([#191](https://github.com/dashevo/platform/issues/191))

## [0.22.0-dev.6](https://github.com/dashevo/platform/compare/v0.22.0-dev.5...v0.22.0-dev.6) (2022-01-11)


### ⚠ BREAKING CHANGES

* **drive:** temporary restrictions for a document query (#77)

### Features

* **dapi:** `subscribeToBlockHeadersWithChainLocks` endpoint ([#153](https://github.com/dashevo/platform/issues/153))


### Bug Fixes

* **drive:** missed `nodeAddress` field on `EndBlock` ([#184](https://github.com/dashevo/platform/issues/184))


### Build System

* **test-suite:** docker image build doesn't work ([#172](https://github.com/dashevo/platform/issues/172))


### Code Refactoring

* **dapi:** rename tx-filter-stream.js to core-streams.js ([#169](https://github.com/dashevo/platform/issues/169))


### Documentation

* add readme to docs folder ([#175](https://github.com/dashevo/platform/issues/175))
* indicate which network(s) this repo supports ([#174](https://github.com/dashevo/platform/issues/174))


### Miscellaneous Chores

* **drive:** temporary restrictions for a document query ([#77](https://github.com/dashevo/platform/issues/77))
* **release:** update changelog and version to 0.22.0-dev.6 ([#185](https://github.com/dashevo/platform/issues/185))

## [0.22.0-dev.5](https://github.com/dashevo/platform/compare/v0.22.0-dev.4...v0.22.0-dev.5) (2022-01-07)


### ⚠ BREAKING CHANGES

* **dpp:** `$id` can't be used in secondary indices
* **dpp:** Indexed properties now require size constraints
* allow using non-unique Identity public keys (#168)

### Features

* allow adding non-unique indices for newly defined properties ([#83](https://github.com/dashevo/platform/issues/83))
* allow using non-unique Identity public keys ([#168](https://github.com/dashevo/platform/issues/168))
* **dpp:** `$id` can't be used in secondary indices ([#178](https://github.com/dashevo/platform/issues/178))
* **dpp:** size constraints for indexed properties ([#179](https://github.com/dashevo/platform/issues/179))
* masternode reward shares contract ([#160](https://github.com/dashevo/platform/issues/160))


### Bug Fixes

* downgrade dash-core image to v0.17 ([#171](https://github.com/dashevo/platform/issues/171))


### Documentation

* minor Readme fixes ([#163](https://github.com/dashevo/platform/issues/163))


### Miscellaneous Chores

* **drive:** send initial core chain locked height on init chain ([#180](https://github.com/dashevo/platform/issues/180))
* **release:** update changelog and version to 0.22.0-dev.5 ([#181](https://github.com/dashevo/platform/issues/181))
* update to use current @oclif/core ([#154](https://github.com/dashevo/platform/issues/154))

## [0.22.0-dev.4](https://github.com/dashevo/platform/compare/v0.22.0-dev.3...v0.22.0-dev.4) (2021-12-24)


### Bug Fixes

* **drive:** `ValidatorSetUpdate` doesn't contain `nodeAddress` ([#155](https://github.com/dashevo/platform/issues/155))


### Continuous Integration

* pass NPM token to `npm publish` command ([#151](https://github.com/dashevo/platform/issues/151))
* set NPM token to `setup-node` action ([#150](https://github.com/dashevo/platform/issues/150))


### Miscellaneous Chores

* configure NPM registry ([#149](https://github.com/dashevo/platform/issues/149))
* **release:** update changelog and bump version to 0.22.0-dev.4 ([#157](https://github.com/dashevo/platform/issues/157))
* yarn abci cache ([#156](https://github.com/dashevo/platform/issues/156))

## [0.22.0-dev.3](https://github.com/dashevo/platform/compare/v0.21.6...v0.22.0-dev.3) (2021-12-22)


### ⚠ BREAKING CHANGES

* add required `name` property to index definition (#74)
* add an ability to update data contract (#52)
* Identity public key now has two more fields, purpose and securityLevel, and keys without those fields won't be valid anymore

### Features

* add an ability to update data contract ([#52](https://github.com/dashevo/platform/issues/52))
* add required `name` property to index definition ([#74](https://github.com/dashevo/platform/issues/74))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31))
* **dpp:** add `readOnly` flag to `IdentityPublicKey` ([#142](https://github.com/dashevo/platform/issues/142))
* **dpp:** implement hashed ECDSA key type ([#141](https://github.com/dashevo/platform/issues/141))
* **drive:** network address in `ValidatorUpdate` ABCI ([#140](https://github.com/dashevo/platform/issues/140))
* enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* identity public key purpose and security levels ([#46](https://github.com/dashevo/platform/issues/46))
* **wallet-lib:** do not sync transactions if mnemonic is absent
* **wallet-lib:** dump wallet storage ([#8](https://github.com/dashevo/platform/issues/8))


### Bug Fixes

* **dashmate:** `cannot read properties of undefined (reading 'dpns')` on reset ([#47](https://github.com/dashevo/platform/issues/47))


### Documentation

* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))
* provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))


### Tests

* **wallet-lib:** enable skipped test after the fix for grpc-js lib ([#71](https://github.com/dashevo/platform/issues/71))


### Miscellaneous Chores

* **release:** update changelog and bump version to 0.22.0-dev.3 ([#147](https://github.com/dashevo/platform/issues/147))
* **release:** update changelog and version to 0.22.0-dev.1 ([#146](https://github.com/dashevo/platform/issues/146))

### [0.21.8](https://github.com/dashevo/platform/compare/v0.22.0-dev.7...v0.21.8) (2022-02-15)


### Bug Fixes

* sorting unconfirmed tx as oldest ([#206](https://github.com/dashevo/platform/issues/206))
* **wallet-lib:** get transaction history missing txs ([#246](https://github.com/dashevo/platform/issues/246))


### Tests

* **platform-suite:** add -b flag to abort after first error ([#222](https://github.com/dashevo/platform/issues/222))


### Miscellaneous Chores

* **release:** update changelog and version to 0.21.8 ([#247](https://github.com/dashevo/platform/issues/247))
* updates @dashevo/dashcore-lib to v0.19.30 ([#238](https://github.com/dashevo/platform/issues/238))

## [0.21.8](https://github.com/dashevo/platform/compare/v0.21.7...v0.21.8) (2022-02-15)


### Bug Fixes

* sorting unconfirmed tx as oldest ([#206](https://github.com/dashevo/platform/issues/206))
* **wallet-lib:** get transaction history missing txs ([#246](https://github.com/dashevo/platform/issues/246))


### Tests

* **platform-suite:** add -b flag to abort after first error ([#222](https://github.com/dashevo/platform/issues/222))


### Miscellaneous Chores

* updates @dashevo/dashcore-lib to v0.19.30 ([#238](https://github.com/dashevo/platform/issues/238))


## [0.22.0-dev.7](https://github.com/dashevo/platform/compare/v0.21.7...v0.22.0-dev.7) (2022-01-19)


### Features

* added WalletStore ([#197](https://github.com/dashevo/platform/issues/197))
* **drive:** allow using `in` and `startsWith` only in last `where` condition ([#201](https://github.com/dashevo/platform/issues/201))
* **drive:** allow using `orderBy` for fields having `in` and `startsWith` in last `where` clause ([#199](https://github.com/dashevo/platform/issues/199))
* register system contracts on `initChain` ([#182](https://github.com/dashevo/platform/issues/182))
* **wallet-lib:** ChainStore ([#196](https://github.com/dashevo/platform/issues/196))


### Bug Fixes

* **sdk:** system contract ids were hardcoded in SDKs Client module ([#192](https://github.com/dashevo/platform/issues/192))


### Build System

* fix configure test suite script for grep 2.5.1 ([#187](https://github.com/dashevo/platform/issues/187))


### Miscellaneous Chores

* **dashmate:** update tenderdash to 0.7.0-dev ([#188](https://github.com/dashevo/platform/issues/188))
* remove `fixCumulativeFeesBug` feature flag ([#191](https://github.com/dashevo/platform/issues/191))



## [0.21.7](https://github.com/dashevo/platform/compare/v0.21.6...v0.21.7) (2022-01-17)


### ⚠ BREAKING CHANGES

* **dashmate:** `platform.drive.abci.docker.build.path' and 'platform.dapi.api.docker.build.path' are removed in favor of `platform.sourcePath'

### Features

* **dashmate:** build DAPI and Drive from monorepo path ([#145](https://github.com/dashevo/platform/issues/145))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* support Apple Silicone ([#143](https://github.com/dashevo/platform/issues/143))


### Bug Fixes

* instantlock waiting period for transaction <hash> timed out


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))


### Documentation

* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))


### Tests

* **wallet-lib:** fix hanging functional test ([#186](https://github.com/dashevo/platform/issues/186))

## [0.22.0-dev.6](https://github.com/dashevo/platform/compare/v0.22.0-dev.5...v0.22.0-dev.6) (2022-01-11)


### ⚠ BREAKING CHANGES

* **drive:** temporary restrictions for a document query (#77)

### Features

* **dapi:** `subscribeToBlockHeadersWithChainLocks` endpoint ([#153](https://github.com/dashevo/platform/issues/153))


### Bug Fixes

* **drive:** missed `nodeAddress` field on `EndBlock` ([#184](https://github.com/dashevo/platform/issues/184))


### Miscellaneous Chores

* **drive:** temporary restrictions for a document query ([#77](https://github.com/dashevo/platform/issues/77))


### Build System

* **test-suite:** docker image build doesn't work ([#172](https://github.com/dashevo/platform/issues/172))


### Code Refactoring

* **dapi:** rename tx-filter-stream.js to core-streams.js ([#169](https://github.com/dashevo/platform/issues/169))


### Documentation

* add readme to docs folder ([#175](https://github.com/dashevo/platform/issues/175))
* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))
* indicate which network(s) this repo supports ([#174](https://github.com/dashevo/platform/issues/174))

## [0.22.0-dev.5](https://github.com/dashevo/platform/compare/v0.22.0-dev.4...v0.22.0-dev.5) (2022-01-07)


### ⚠ BREAKING CHANGES

* **dpp:** `$id` can't be used in secondary indices
* **dpp:** Indexed properties now require size constraints
* allow using non-unique Identity public keys (#168)
* **dashmate:** `platform.drive.abci.docker.build.path' and 'platform.dapi.api.docker.build.path' are removed in favor of `platform.sourcePath'

### Features

* allow adding non-unique indices for newly defined properties ([#83](https://github.com/dashevo/platform/issues/83))
* allow using non-unique Identity public keys ([#168](https://github.com/dashevo/platform/issues/168))
* **dashmate:** build DAPI and Drive from monorepo path ([#145](https://github.com/dashevo/platform/issues/145))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* **dpp:** `$id` can't be used in secondary indices ([#178](https://github.com/dashevo/platform/issues/178))
* **dpp:** size constraints for indexed properties ([#179](https://github.com/dashevo/platform/issues/179))
* masternode reward shares contract ([#160](https://github.com/dashevo/platform/issues/160))


### Bug Fixes

* downgrade dash-core image to v0.17 ([#171](https://github.com/dashevo/platform/issues/171))


### Documentation

* minor Readme fixes ([#163](https://github.com/dashevo/platform/issues/163))


### Miscellaneous Chores

* **drive:** send initial core chain locked height on init chain ([#180](https://github.com/dashevo/platform/issues/180))
* update to use current @oclif/core ([#154](https://github.com/dashevo/platform/issues/154))

## [0.22.0-dev.4](https://github.com/dashevo/platform/compare/v0.22.0-dev.3...v0.22.0-dev.4) (2021-12-24)


### Bug Fixes

* **drive:** `ValidatorSetUpdate` doesn't contain `nodeAddress` ([#155](https://github.com/dashevo/platform/issues/155))
* **drive:** missed JS ABCI yarn cache ([#156](https://github.com/dashevo/platform/issues/156))

## [0.22.0-dev.3](https://github.com/dashevo/platform/compare/v0.21.6...v0.22.0-dev.3) (2021-12-21)


### ⚠ BREAKING CHANGES

* add required `name` property to index definition (#74)
* add an ability to update data contract (#52)
* Identity public key now has two more fields, purpose and securityLevel, and keys without those fields won't be valid anymore

### Features

* add an ability to update data contract ([#52](https://github.com/dashevo/platform/issues/52))
* add required `name` property to index definition ([#74](https://github.com/dashevo/platform/issues/74))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31))
* **dpp:** add `readOnly` flag to `IdentityPublicKey` ([#142](https://github.com/dashevo/platform/issues/142))
* **drive:** network address in `ValidatorUpdate` ABCI ([#140](https://github.com/dashevo/platform/issues/140))
* enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* identity public key purpose and security levels ([#46](https://github.com/dashevo/platform/issues/46))
* support Apple Silicone ([#143](https://github.com/dashevo/platform/issues/143))
* **wallet-lib:** do not sync transactions if mnemonic is absent
* **wallet-lib:** dump wallet storage ([#8](https://github.com/dashevo/platform/issues/8))


### Bug Fixes

* **dashmate:** `cannot read properties of undefined (reading 'dpns')` on reset ([#47](https://github.com/dashevo/platform/issues/47))


### Documentation

* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))
* provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))


### Tests

* **wallet-lib:** enable skipped test after the fix for grpc-js lib ([#71](https://github.com/dashevo/platform/issues/71))


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))


## [0.21.6](https://github.com/dashevo/platform/compare/v0.21.5...v0.21.6) (2021-12-13)


### Bug Fixes

* **dashmate:** RPC error on stopping node ([#61](https://github.com/dashevo/platform/issues/61))
* **wallet-lib:** "Failure: Type not convertible to Uint8Array" ([#60](https://github.com/dashevo/platform/issues/60))
* **wallet-lib:** eventemitter memory leak ([#56](https://github.com/dashevo/platform/issues/56))
* **wallet-lib:** invalid deserialization of persistent storage ([#76](https://github.com/dashevo/platform/issues/76))


### Documentation

* publish consolidated docs using mkdocs ([#42](https://github.com/dashevo/platform/issues/42))


### Miscellaneous Chores

* changelogs generation script ([#62](https://github.com/dashevo/platform/issues/62))
* enable yarn PnP to achieve zero installs ([#63](https://github.com/dashevo/platform/issues/63))
* exit if some env variables are empty during setup ([#75](https://github.com/dashevo/platform/issues/75))
* fix `test:drive` script ([#78](https://github.com/dashevo/platform/issues/78))
* migrate from NPM to Yarn 3 ([#50](https://github.com/dashevo/platform/issues/50))
* remove temporary reset script ([#64](https://github.com/dashevo/platform/issues/64))
* update oclif and remove pnpify ([#73](https://github.com/dashevo/platform/issues/73))


### Build System

* fix bash syntax issue in release script ([#79](https://github.com/dashevo/platform/issues/79))
* release process automation ([#67](https://github.com/dashevo/platform/issues/67))

## [0.21.5](https://github.com/dashevo/platform/compare/v0.21.4...v0.21.5) (2021-11-25)


### Bug Fixes

* new instant lock is not compatible with DashCore 0.17 ([#57](https://github.com/dashevo/platform/issues/57))
* **wallet-lib:** tx chaining mempool conflict errors ([#57](https://github.com/dashevo/platform/issues/44))


### Continuous Integration
* use correct Dockerfile in test suite release ([#58](https://github.com/dashevo/platform/issues/58))
* set correct docker tag outputs in release workflow ([#55](https://github.com/dashevo/platform/issues/55))
* enable NPM login on for release workflow ([#54](https://github.com/dashevo/platform/issues/54))


## [0.21.4](https://github.com/dashevo/platform/compare/v0.21.0...v0.21.4) (2021-11-23)


### Bug Fixes

* **dapi-client:** expect 100 but got 122 in SML provider test ([#22](https://github.com/dashevo/platform/issues/22))
* **dapi-client:** retry doesn’t work with 502 errors ([#35](https://github.com/dashevo/platform/issues/35))
* **dapi:** Identifier expects Buffer ([#28](https://github.com/dashevo/platform/issues/28))
* **dashmate:** ajv schema errors ([#14](https://github.com/dashevo/platform/issues/14))
* **dashmate:** reset command doesn't work if setup failed ([#23](https://github.com/dashevo/platform/issues/23))
* **dashmate:** cannot read properties error on group:reset ([#47](https://github.com/dashevo/platform/issues/47))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31))
* **dashmate:** enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* **dpp:** rename generateEntropy to entropyGenerator ([#13](https://github.com/dashevo/platform/issues/13))
* **sdk:** dpp hash function import ([#15](https://github.com/dashevo/platform/issues/15))
* **sdk:** override ts-node target for unit tests ([#21](https://github.com/dashevo/platform/issues/21))
* **sdk:** this is undefined during unit tests ([#18](https://github.com/dashevo/platform/issues/18))


### Features

* **dashmate:** force option for `group:stop` command ([#36](https://github.com/dashevo/platform/issues/36))
* **dashmate:** provide docker build logs for verbose mode ([#19](https://github.com/dashevo/platform/issues/19))
* migrate to DashCore 0.18.0.0-beta1 ([#51](https://github.com/dashevo/platform/issues/51))
* **wallet-lib:** dump wallet storage ([#8](https://github.com/dashevo/platform/issues/8))
* **wallet-lib:** do not sync transactions if mnemonic is absent ([#7](https://github.com/dashevo/platform/issues/7))


### Performance Improvements

* **test-suite:** speedup test suite up to 6 times ([#30](https://github.com/dashevo/platform/issues/30))


### Build System
* build only necessary packages ([#27](https://github.com/dashevo/platform/issues/27))
* run npm scripts in parallel ([#33](https://github.com/dashevo/platform/issues/33))
* cache native npm modules during docker build ([#20](https://github.com/dashevo/platform/issues/20))
* setup semantic pull requests ([#11](https://github.com/dashevo/platform/issues/11))
* **sdk:** upgrade to webpack 5 ([#6](https://github.com/dashevo/platform/issues/6))


### Continuous Integration
* simplify release workflow ([#48](https://github.com/dashevo/platform/issues/48))
* show docker logs on failure ([#43](https://github.com/dashevo/platform/issues/43))
* check mismatch dependencies ([#26](https://github.com/dashevo/platform/issues/26))
* run package tests in parallel ([#25](https://github.com/dashevo/platform/issues/25))


### Tests
* adjust timeouts ([#45](https://github.com/dashevo/platform/issues/45))
* **test-suite:** skipSynchronizationBeforeHeight option with new wallet ([#34](https://github.com/dashevo/platform/issues/34))
* **dpp:** fix invalid network floating error ([#32](https://github.com/dashevo/platform/issues/32))
* **dpp:** grpc common bootstrap not working ([#16](https://github.com/dashevo/platform/issues/16))


### Documentation
* markdown link fixes ([#49](https://github.com/dashevo/platform/issues/49))
* add README.md for the whole platform as a project ([#38](https://github.com/dashevo/platform/issues/38))
* add contributing.md ([#37](https://github.com/dashevo/platform/issues/37))
* **sdk:** provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))
* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))


### Styles
* fix ES linter errors ([#24](https://github.com/dashevo/platform/issues/24))


### BREAKING CHANGES

* supports only new DashCore InstantLock format https://github.com/dashpay/dips/blob/master/dip-0022.md


# Previous versions

Before 0.21.x, packages were located in separate repositories and have own changelogs:

* [DAPI Client](https://github.com/dashevo/js-dapi-client/blob/master/CHANGELOG.md)
* [DAPI gRPC](https://github.com/dashevo/dapi-grpc/blob/master/CHANGELOG.md)
* [DAPI](https://github.com/dashevo/dapi/blob/master/CHANGELOG.md)
* [Dashmate](https://github.com/dashevo/dashmate/blob/master/CHANGELOG.md)
* [DashPay contract](https://github.com/dashevo/dashpay-contract/blob/master/CHANGELOG.md)
* [Feature Flags Contract](https://github.com/dashevo/feature-flags-contract/blob/master/CHANGELOG.md)
* [Dash SDK](https://github.com/dashevo/js-dash-sdk/blob/master/CHANGELOG.md)
* [Dash Platform Protocol JS](https://github.com/dashevo/js-dpp/blob/master/CHANGELOG.md)
* [Drive](https://github.com/dashevo/js-drive/blob/master/CHANGELOG.md)
* [Dash Platform Test Suite](https://github.com/dashevo/platform-test-suite/blob/master/CHANGELOG.md)
* [Wallet Library](https://github.com/dashevo/wallet-lib/blob/master/CHANGELOG.md)
