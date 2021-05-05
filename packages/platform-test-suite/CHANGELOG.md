# [0.19.0](https://github.com/dashevo/platform-test-suite/compare/v0.18.0...v0.19.0) (2021-05-05)


### Features

* integrate Chain Asset Lock Proofs ([#115](https://github.com/dashevo/platform-test-suite/issues/115), [#120](https://github.com/dashevo/platform-test-suite/issues/120), [#122](https://github.com/dashevo/platform-test-suite/issues/122))
* update to new `getStatus` endpoint ([#111](https://github.com/dashevo/platform-test-suite/issues/111), [#119](https://github.com/dashevo/platform-test-suite/issues/119), [#114](https://github.com/dashevo/platform-test-suite/issues/114))
* remove fallbacks from regtest mode ([#103](https://github.com/dashevo/platform-test-suite/issues/103))
* CI with Github Actions ([#108](https://github.com/dashevo/platform-test-suite/issues/108), [#117](https://github.com/dashevo/platform-test-suite/issues/117))


### Bug Fixes

* bash script could not run mocha in github actions ([#105](https://github.com/dashevo/platform-test-suite/issues/105))



# [0.18.0](https://github.com/dashevo/platform-test-suite/compare/v0.17.0...v0.18.0) (2021-03-03)


### Features

* use SDK with new ST acknowledgment ([#96](https://github.com/dashevo/platform-test-suite/pull/96))


### Bug Fixes

* identity was used in a wrong way ([f115468](https://github.com/dashevo/platform-test-suite/commit/f1154689e5a9c451a625a77c5b8c929e118a7fc6))
* removed unused identity variable ([81f4839](https://github.com/dashevo/platform-test-suite/commit/81f4839bc67a8fdcb0df6283dae3276a72c579d7))



# [0.17.0](https://github.com/dashevo/platform-test-suite/compare/v0.16.0...v0.17.0) (2020-12-30)


### Features

* make test works without fallback ([#91](https://github.com/dashevo/platform-test-suite/issues/91))
* update `dashcore-lib`, `dpp`, `wallet-lib`, `dashjs` ([#81](https://github.com/dashevo/platform-test-suite/issues/81), [#83](https://github.com/dashevo/platform-test-suite/issues/83), [#88](https://github.com/dashevo/platform-test-suite/issues/88))
* identity funding double-spend tests ([#86](https://github.com/dashevo/platform-test-suite/issues/86))


### Bug Fixes

* fake asset lock must be passed only for regtest ([#90](https://github.com/dashevo/platform-test-suite/issues/90))
* invalid assertions in `Identity` functional test ([#84](https://github.com/dashevo/platform-test-suite/issues/84))



# [0.16.0](https://github.com/dashevo/platform-test-suite/compare/v0.15.0...v0.16.0) (2020-10-28)


### Chore

* update to SDK 0.16 ([#77](https://github.com/dashevo/platform-test-suite/issues/77))


### BREAKING CHANGES

* Nodes with Dash Platform 0.15 are not supported



# [0.15.0](https://github.com/dashevo/platform-test-suite/compare/v0.14.0...v0.15.0) (2020-09-04)


### Bug Fixes

* faucet client singleton ([#70](https://github.com/dashevo/platform-test-suite/issues/70))
* core tests were using `serialize` instead of `toBuffer` ([#66](https://github.com/dashevo/platform-test-suite/issues/66))
* npm is not running `prepare` script as root ([#63](https://github.com/dashevo/platform-test-suite/issues/63))


### Features

* `wallet` e2e test ([#59](https://github.com/dashevo/platform-test-suite/issues/59))
* new test timeout option ([#71](https://github.com/dashevo/platform-test-suite/issues/71))
* remove pending `subscribeToTransactionsWithProofs` functional tests ([#72](https://github.com/dashevo/platform-test-suite/issues/72))
* remove getAddressSummary tests ([#67](https://github.com/dashevo/platform-test-suite/issues/67))
* update Wallet, DPP, DPNS and SDK deps ([#50](https://github.com/dashevo/platform-test-suite/issues/50), [#64](https://github.com/dashevo/platform-test-suite/issues/64), [#68](https://github.com/dashevo/platform-test-suite/issues/68), [#60](https://github.com/dashevo/platform-test-suite/issues/60))
* support for installation node module from git on docker start ([#55](https://github.com/dashevo/platform-test-suite/issues/55)) ([adb1e16](https://github.com/dashevo/platform-test-suite/commit/adb1e1672a0288672b2eaef0bf9effc9212b50ad))
* new topup identity test ([#53](https://github.com/dashevo/platform-test-suite/issues/53)) ([075f09c](https://github.com/dashevo/platform-test-suite/commit/075f09cb211fcda45aff2c75a2222e735f9eab49))


### Code Refactoring

* use Wallet lib instead of getUTXO ([#62](https://github.com/dashevo/platform-test-suite/issues/62))



# 0.14.0 (2020-07-23)


### Features

* use external Travis scripts ([#47](https://github.com/dashevo/platform-test-suite/issues/47))
* update SDK to 3.14.0 ([#45](https://github.com/dashevo/platform-test-suite/issues/45))
* update DPP to 0.14.0 ([#42](https://github.com/dashevo/platform-test-suite/issues/42))
* add document timestamp tests ([#40](https://github.com/dashevo/platform-test-suite/issues/40))
* define npm test scopes ([#31](https://github.com/dashevo/platform-test-suite/issues/31))
* dockerize test suite ([#28](https://github.com/dashevo/platform-test-suite/issues/28))
* functional core tests ([#33](https://github.com/dashevo/platform-test-suite/issues/33))
* implement functional tests ([#38](https://github.com/dashevo/platform-test-suite/issues/38))
