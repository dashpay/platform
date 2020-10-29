# [0.16.0](https://github.com/dashevo/mn-bootstrap/compare/v0.15.1...v0.16.0) (2020-10-29)


### Bug Fixes

* "No available addresses" in setup command on the platform init step ([#164](https://github.com/dashevo/mn-bootstrap/issues/164))


### Features

* make `NODE_ENV` and logging level configurable ([#172](https://github.com/dashevo/mn-bootstrap/issues/172))
* obtain and pass DPNS contract block height ([#170](https://github.com/dashevo/mn-bootstrap/issues/170), [#173](https://github.com/dashevo/mn-bootstrap/issues/173))
* update to Dash SDK 0.16 ([#160](https://github.com/dashevo/mn-bootstrap/issues/163), [#163](https://github.com/dashevo/mn-bootstrap/issues/163), [#163](https://github.com/dashevo/mn-bootstrap/issues/163), [#166](https://github.com/dashevo/mn-bootstrap/issues/166))
* restart command ([#152](https://github.com/dashevo/mn-bootstrap/issues/152))
* switch insight-api docker image to shumkov/insight-api:3.0.0 ([#157](https://github.com/dashevo/mn-bootstrap/issues/157))
* update Dash Core to 0.16 ([#153](https://github.com/dashevo/mn-bootstrap/issues/153), [#155](https://github.com/dashevo/mn-bootstrap/issues/155))


### Documentation

* cannot mint dash on evonet ([#171](https://github.com/dashevo/mn-bootstrap/issues/171))


### BREAKING CHANGES

* `platform.dpns.contractId` config options is moved to `platform.dpns.contract.id`
* data created with 0.15 version and less in not compatible. Please reset your node before upgrade
* see [Drive breaking changes](https://github.com/dashevo/js-drive/releases/tag/v0.16.0)
* see [DAPI breaking changes](https://github.com/dashevo/dapi/releases/tag/v0.16.0)



## [0.15.1](https://github.com/dashevo/mn-bootstrap/compare/v0.15.0...v0.15.1) (2020-09-08)


### Bug Fixes

* services.core.ports contains an invalid type ([#149](https://github.com/dashevo/mn-bootstrap/issues/149))



# [0.15.0](https://github.com/dashevo/mn-bootstrap/compare/v0.14.0...v0.15.0) (2020-09-04)


### Bug Fixes

* ignored mint address option ([#143](https://github.com/dashevo/mn-bootstrap/issues/143))
* Dash Client was created before Tendermint is started ([#131](https://github.com/dashevo/mn-bootstrap/issues/131))
* gRPC buffer size settings in NGINX was too small ([#127](https://github.com/dashevo/mn-bootstrap/issues/127))
* transaction filter stream doesn't work with gRPC-Web ([#116](https://github.com/dashevo/mn-bootstrap/issues/116))


### Features

* replace env files and presets with new `config` command ([#119](https://github.com/dashevo/mn-bootstrap/issues/119), [#138](https://github.com/dashevo/mn-bootstrap/issues/138))
* remove unnecessary block generation ([#141](https://github.com/dashevo/mn-bootstrap/issues/141))
* block mining with local development ([#137](https://github.com/dashevo/mn-bootstrap/issues/137))
* move container datadirs to named docker volumes ([#123](https://github.com/dashevo/mn-bootstrap/issues/123), [#139](https://github.com/dashevo/mn-bootstrap/issues/139), [#140](https://github.com/dashevo/mn-bootstrap/issues/140), [#142](https://github.com/dashevo/mn-bootstrap/issues/142))
* nginx responds with unimplemented in case of unsupported version ([#134](https://github.com/dashevo/mn-bootstrap/issues/134))
* move `subscribeToTransactionsWithProofs` to `Core` service ([#121](https://github.com/dashevo/mn-bootstrap/issues/121))
* use new DPNS contract ([#117](https://github.com/dashevo/mn-bootstrap/issues/117))
* generate empty blocks every 3 minutes ([#114](https://github.com/dashevo/mn-bootstrap/issues/114))
* use `generateToAddress` instead of `generate` ([#111](https://github.com/dashevo/mn-bootstrap/issues/111))
* add docker image update support to setup-for-local-development ([#113](https://github.com/dashevo/mn-bootstrap/issues/113))


### Code Refactoring

* use MongoDB init script to initiate replica ([#147](https://github.com/dashevo/mn-bootstrap/issues/147))
* remove getUTXO dependency for SDK ([#133](https://github.com/dashevo/mn-bootstrap/issues/139))


### BREAKING CHANGES

* node data from `data` dir is not using anymore and should be removed
* see [Drive breaking changes](https://github.com/dashevo/js-drive/releases/tag/v0.15.0)
* see [DAPI breaking changes](https://github.com/dashevo/dapi/releases/tag/v0.15.0)



# [0.14.0](https://github.com/dashevo/mn-bootstrap/compare/v0.13.4...v0.14.0) (2020-07-24)


### Bug Fixes

* missing `build` section for `tx_filter_stream_service` service ([#94](https://github.com/dashevo/mn-bootstrap/issues/94))
* missing env variables for `dapi-tx-filter-stream` service ([#99](https://github.com/dashevo/mn-bootstrap/issues/99))
* faucet inputs where locked after platform initialization script ([#88](https://github.com/dashevo/mn-bootstrap/issues/88))
* original Tendermint image creates wrong mount points ([#86](https://github.com/dashevo/mn-bootstrap/issues/86))


### Features

* update Evonet preset to 0.14 ([#108](https://github.com/dashevo/mn-bootstrap/issues/108), [#105](https://github.com/dashevo/mn-bootstrap/issues/105))
* update Drive and DAPI versions to 0.14 ([#98](https://github.com/dashevo/mn-bootstrap/issues/98))
* implement `status` command ([#49](https://github.com/dashevo/mn-bootstrap/issues/49), [#93](https://github.com/dashevo/mn-bootstrap/issues/93), [#96](https://github.com/dashevo/mn-bootstrap/issues/96))
* move from Listr to Listr2 ([#84](https://github.com/dashevo/mn-bootstrap/issues/84))
* implement `setup-for-local-development` command ([#82](https://github.com/dashevo/mn-bootstrap/issues/82), [#101](https://github.com/dashevo/mn-bootstrap/issues/101))
* implement `update` option for `start` command ([#80](https://github.com/dashevo/mn-bootstrap/issues/80))
* build docker images from local directories ([#59](https://github.com/dashevo/mn-bootstrap/issues/59), [#66](https://github.com/dashevo/mn-bootstrap/issues/66), [#90](https://github.com/dashevo/mn-bootstrap/issues/90))


### Documentation

* document `status` command in README ([#97](https://github.com/dashevo/mn-bootstrap/issues/97))
* add release date badge ([#85](https://github.com/dashevo/mn-bootstrap/issues/85))
* add development usage for local docker build ([#67](https://github.com/dashevo/mn-bootstrap/issues/67))


### BREAKING CHANGES

* data created with previous versions of Dash Platform is incompatible we the new one, so you need to reset data before you start the node



## [0.13.4](https://github.com/dashevo/mn-bootstrap/compare/v0.13.3...v0.13.4) (2020-06-18)


### Bug Fixes

* tendermint throw fatal error on start in linux environment ([#76](https://github.com/dashevo/mn-bootstrap/issues/76))



## [0.13.3](https://github.com/dashevo/mn-bootstrap/compare/v0.13.2...v0.13.3) (2020-06-18)


### Bug Fixes

* parsing docker container name on first start ([#75](https://github.com/dashevo/mn-bootstrap/issues/75))



## [0.13.2](https://github.com/dashevo/mn-bootstrap/compare/v0.13.1...v0.13.2) (2020-06-16)


### Bug Fixes

* DAPI rate limits disabled for evonet for some reason ([#73](https://github.com/dashevo/mn-bootstrap/issues/73))



## [0.13.1](https://github.com/dashevo/mn-bootstrap/compare/v0.12.6...v0.13.1) (2020-06-12)


### Features

* update Evonet configs ([fd0158a](https://github.com/dashevo/mn-bootstrap/commit/fd0158a45f1c624628fe7a2735124db1c9f20338))



# [0.13.0](https://github.com/dashevo/mn-bootstrap/compare/v0.12.6...v0.13.0) (2020-06-09)


### Bug Fixes

* do not start stopped services on the docker deamon restart ([#55](https://github.com/dashevo/mn-bootstrap/issues/55))
* switch to dashpay org for sentinel ([#62](https://github.com/dashevo/mn-bootstrap/issues/62))


### Features

* start/stop node commands ([#45](https://github.com/dashevo/mn-bootstrap/issues/45), [#48](https://github.com/dashevo/mn-bootstrap/issues/48))
* data reset command ([#43](https://github.com/dashevo/mn-bootstrap/issues/43), [#60](https://github.com/dashevo/mn-bootstrap/issues/60))
* masternode registration commands ([#30](https://github.com/dashevo/mn-bootstrap/issues/30), [#44](https://github.com/dashevo/mn-bootstrap/issues/44), [#54](https://github.com/dashevo/mn-bootstrap/issues/54), [#69](https://github.com/dashevo/mn-bootstrap/issues/69))
* remove sleep from docker compose ([#57](https://github.com/dashevo/mn-bootstrap/issues/57))
* allow to start full node ([#42](https://github.com/dashevo/mn-bootstrap/issues/42))
* update configs and docker images ([#64](https://github.com/dashevo/mn-bootstrap/issues/42))


### Documentation

* update README.md to clarify install instructions ([#33](https://github.com/dashevo/mn-bootstrap/issues/33), [#65](https://github.com/dashevo/mn-bootstrap/issues/65))


### BREAKING CHANGES

* Dash Platform v0.12 data in incompatible with 0.13, so you need to reset data before you start the node



# [0.12.6](https://github.com/dashevo/mn-bootstrap/compare/v0.12.5...v0.12.6) (2020-05-23)


### Features

* update Evonet configs ([#56](https://github.com/dashevo/mn-bootstrap/issues/56))



# [0.12.5](https://github.com/dashevo/mn-bootstrap/compare/v0.12.4...v0.12.5) (2020-05-01)


### Bug Fixes

* use updated sentinel image ([#41](https://github.com/dashevo/mn-bootstrap/issues/41))



# [0.12.4](https://github.com/dashevo/mn-bootstrap/compare/v0.12.3...v0.12.4) (2020-04-30)


### Bug Fixes

* MongoDB replica set doesn't work sometimes ([#40](https://github.com/dashevo/mn-bootstrap/issues/40)) ([a5e31cd](https://github.com/dashevo/mn-bootstrap/commit/a5e31cd341bfd3e18240e3ee4c8f5dfeebfd249c))



# [0.12.3](https://github.com/dashevo/mn-bootstrap/compare/v0.12.2...v0.12.3) (2020-04-28)


### Bug Fixes

* outdated genesis config for Tendermint ([#37](https://github.com/dashevo/mn-bootstrap/issues/37))
* outdated persistent node IDs in Tendermint config ([#38](https://github.com/dashevo/mn-bootstrap/issues/38))



## [0.12.2](https://github.com/dashevo/mn-bootstrap/compare/v0.12.1...v0.12.2) (2020-04-22)


### Bug Fixes

* update DPNS identities for evonet ([#31](https://github.com/dashevo/mn-bootstrap/issues/31))


## [0.12.1](https://github.com/dashevo/mn-bootstrap/compare/v0.11.1...v0.12.0) (2020-04-21)


## Bug Fixes

* `latest` envoy docker image tag is not present anymore ([#29](https://github.com/dashevo/mn-bootstrap/issues/29))


# [0.12.0](https://github.com/dashevo/mn-bootstrap/compare/v0.11.1...v0.12.0) (2020-04-19)


### Bug Fixes

* dash-cli doesn't work without default config ([#18](https://github.com/dashevo/mn-bootstrap/issues/18))
* explicitly load core conf file ([#23](https://github.com/dashevo/mn-bootstrap/issues/23))
* invalid gRPC Web configuration ([#25](https://github.com/dashevo/mn-bootstrap/issues/25), [#26](https://github.com/dashevo/mn-bootstrap/issues/26))
* remove spork private key from сore config ([#11](https://github.com/dashevo/mn-bootstrap/issues/11))


### Code Refactoring

* tidy up services and configs ([#27](https://github.com/dashevo/mn-bootstrap/issues/27))


### Features

* add testnet preset ([#15](https://github.com/dashevo/mn-bootstrap/issues/15))
* update to new Drive ([#21](https://github.com/dashevo/mn-bootstrap/issues/21), [#24](https://github.com/dashevo/mn-bootstrap/issues/24))


### BREAKING CHANGES

* data and config dir paths are changed
* `tendermint` service now called `drive_tendermint`
* `machine` is removed due to merging Machine into Drive
* new version of Drive is incompatible with 0.11 so you need to wipe data before run 0.12:
  * drop `drive_mongodb` and `drive_leveldb` volumes
  * `docker-commpose --env-file=.env.<PRESET> run drive_tendermint unsafe_reset_all`


## 0.11.1 (2020-03-17)


### Bug Fixes

*  update configs for Evonet ([#7](https://github.com/dashevo/mn-bootstrap/issues/7))


# 0.11.0 (2020-03-09)


### Features

* update configurations and docker-compose file for `local` and `evonet` envs ([230ea62](https://github.com/dashevo/mn-bootstrap/commit/230ea62a856b986127eb3b8e52bf7a19a5169818))


### BREAKING CHANGES

* `testnet` and `mainnet` is not supported anymore
