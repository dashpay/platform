## [1.0.0-dev.3](https://github.com/dashpay/platform/compare/v1.0.0-dev.2...v1.0.0-dev.3) (2024-01-16)


### Bug Fixes

* **dapi:** internal errors if broadcasting failed ([#1673](https://github.com/dashpay/platform/issues/1673))

## [1.0.0-dev.2](https://github.com/dashpay/platform/compare/v1.0.0-dev.1...v1.0.0-dev.2) (2024-01-12)


### ⚠ BREAKING CHANGES

* **dashmate:** adjust consensus params and enable re-check (#1669)
* **drive-abci:** internal error if vote extension block is already committed (#1663)

### Bug Fixes

* broadcasting already invalidated transitions ([#1668](https://github.com/dashpay/platform/issues/1668))
* **dashmate:** dapi kills host machine on container stop ([#1670](https://github.com/dashpay/platform/issues/1670))
* **drive-abci:** internal error if vote extension block is already committed ([#1663](https://github.com/dashpay/platform/issues/1663))


### Miscellaneous Chores

* **dashmate:** adjust consensus params and enable re-check ([#1669](https://github.com/dashpay/platform/issues/1669))
* **drive-abci:** fix state transition logging ([#1664](https://github.com/dashpay/platform/issues/1664))
* various logging improvements ([#1666](https://github.com/dashpay/platform/issues/1666))

## [1.0.0-dev.1](https://github.com/dashpay/platform/compare/v0.25.21...v1.0.0-dev.1) (2024-01-11)


### ⚠ BREAKING CHANGES

* invalid state transitions now included into blocks that makes previous chain data invalid. (#1657)
* credit transfer state transition requires revision. (#1634)
* calculated fee amounts are changed (#1656)

### Features

* **drive-abci:** include invalid state transitions into block ([#1657](https://github.com/dashpay/platform/issues/1657))
* **drive-abci:** processing fees for signature verification ([#1656](https://github.com/dashpay/platform/issues/1656))
* **drive-abci:** remove unnecessary validation from check tx and re-check ([#1647](https://github.com/dashpay/platform/issues/1647))
* **sdk:** rs-sdk fetch current epoch ([#1604](https://github.com/dashpay/platform/issues/1604))


### Bug Fixes

* **platform:** credit transfer replay attack ([#1634](https://github.com/dashpay/platform/issues/1634))
* **dapi**: internal error when mempool is full ([#1661](https://github.com/dashpay/platform/issues/1661))


### Miscellaneous Chores

* automatic clippy fixes ([#1528](https://github.com/dashpay/platform/issues/1528), [#1602](https://github.com/dashpay/platform/issues/1602))

### [0.25.21](https://github.com/dashpay/platform/compare/v0.25.20...v0.25.21) (2023-12-28)


### Bug Fixes

* corrupted credits not balanced error ([#1650](https://github.com/dashpay/platform/issues/1650))

### [0.25.20](https://github.com/dashpay/platform/compare/v0.25.19...v0.25.20) (2023-12-21)


### Features

* **dashmate:** more config options for envoy and tenderdash ([#1643](https://github.com/dashpay/platform/issues/1643))


### Bug Fixes

* **drive-abci:** rare process proposal when we prepared tx change ([#1645](https://github.com/dashpay/platform/issues/1645))

### [0.25.19](https://github.com/dashpay/platform/compare/v0.25.18...v0.25.19) (2023-12-19)


### Features

* **dashmate:** add config render command ([#1637](https://github.com/dashpay/platform/issues/1637))


### Bug Fixes

* **drive**: filtering of failed and exceeding limit transactions ([#1639](https://github.com/dashpay/platform/issues/1639))
* runtime error: index out of range 28 with length 28

### Reverts

* **drive:** temporary disable credit transfer transitions ([#1642](https://github.com/dashpay/platform/issues/1642))


### [0.25.18](https://github.com/dashpay/platform/compare/v0.25.17...v0.25.18) (2023-12-12)


### Bug Fixes

* **drive:** temporary disable credit transfer transitions ([#1635](https://github.com/dashpay/platform/issues/1635))

### [0.25.17](https://github.com/dashpay/platform/compare/v0.25.16...v0.25.17) (2023-12-11)

Increment version to overcome already published Dash SDK 3.25.16

### [0.25.16](https://github.com/dashpay/platform/compare/v0.25.15...v0.25.16) (2023-12-06)

### ⚠ BREAKING CHANGES

* **dashmate:** removed `docker.network.bindIp`, please use specific host configuration option (#1630)
* data contracts require position to be defined for object properties
* previously created state is not compatible
* upgrade to Node.JS v20 (#1562)


### Features

* **platform:** document serialization across versions ([#1594](https://github.com/dashpay/platform/issues/1594))
* **dashmate:** configure service listening interfaces ([#1630](https://github.com/dashpay/platform/issues/1630))
* **dashmate:** allow dashmate to update minor core versions ([#1619](https://github.com/dashpay/platform/issues/1619))
* **dashmate:** insight, core block explorer ([#1581](https://github.com/dashpay/platform/issues/1581))
* **dashmate:** update core version to 20.0.1 ([#1588](https://github.com/dashpay/platform/issues/1588))
* **dashmate:** upgrade dashmate to ESM syntax ([#1562](https://github.com/dashpay/platform/issues/1562))
* **package:** bump rust packages
* **sdk:** rs-sdk support for ExtendedEpochInfo::fetch()/fetch_many() ([#1576](https://github.com/dashpay/platform/issues/1576))


### Bug Fixes

* **dashmate:** http API calls fail with `command not found error` ([#1600](https://github.com/dashpay/platform/issues/1600))
* **dapi:** tonik streams hang until first message sent ([#1605](https://github.com/dashpay/platform/issues/1605))
* **dashmate:** missing choices enumerator ([#1595](https://github.com/dashpay/platform/issues/1595))
* **dashmate:** incorrect imports  ([#1591](https://github.com/dashpay/platform/issues/1591))
* drive produces wrong app hash after restart ([#1586](https://github.com/dashpay/platform/issues/1586))
* **dapi:** failure if bloom filter is not set ([#1572](https://github.com/dashpay/platform/issues/1572))
* **dashmate:** incorrect block height color on null remote block height ([#1587](https://github.com/dashpay/platform/issues/1587))

### Performance Improvements

* **dapi:** run a cluster of processes ([#1627](https://github.com/dashpay/platform/issues/1627))


### Build System

* use two faucets for parallel test suite run ([#1615](https://github.com/dashpay/platform/issues/1615))


### Miscellaneous Chores

* **dapi:** logging of the wait for state transition result logic ([#1628](https://github.com/dashpay/platform/issues/1628))
* **dashmate:** update tenderdash to 0.13.4 ([#1631](https://github.com/dashpay/platform/issues/1631))
* remove unused dependencies of rust crates ([#1578](https://github.com/dashpay/platform/issues/1578))
* upgrade to Yarn v4 ([#1562](https://github.com/dashpay/platform/issues/1562))
* upgrade to Node.JS v20 ([#1562](https://github.com/dashpay/platform/issues/1562))
* **dashmate:** remove `platform feature-flag` command ([#1616](https://github.com/dashpay/platform/issues/1616))
* **dashmate:** remove unnecessary WASM DPP ([#1607](https://github.com/dashpay/platform/issues/1607))
* **dashmate:** add platform testnet-37 support ([#1603](https://github.com/dashpay/platform/issues/1603))


### Tests

* **dashmate:** add `dashmate update` unit test ([#1609](https://github.com/dashpay/platform/issues/1609))


### Continuous Integration

* arch dependant yarn unplugged cache ([#1580](https://github.com/dashpay/platform/issues/1580))
* rs-sdk, feature and unnecessary deps testing for rust packages ([#1575](https://github.com/dashpay/platform/issues/1575))


### [0.25.16-rc.6](https://github.com/dashpay/platform/compare/v0.25.16-rc.5...v0.25.16-rc.6) (2023-12-06)


### Features

* **dashmate:** allow dashmate to update minor core versions ([#1619](https://github.com/dashpay/platform/issues/1619))

### Bug Fixes

* **dashmate:** http API calls fail with `command not found error` ([#1600](https://github.com/dashpay/platform/issues/1600))

### Tests

* **dashmate:** add `dashmate update` unit test ([#1609](https://github.com/dashpay/platform/issues/1609))

### Miscellaneous Chores

* **dashmate:** remove `platform feature-flag` command ([#1616](https://github.com/dashpay/platform/issues/1616))
* **dashmate:** remove unnecessary WASM DPP ([#1607](https://github.com/dashpay/platform/issues/1607))

### [0.25.16-rc.5](https://github.com/dashpay/platform/compare/v0.25.16-rc.4...v0.25.16-rc.5) (2023-11-29)


### Bug Fixes

* **dapi:** tonik streams hang until first message sent ([#1605](https://github.com/dashpay/platform/issues/1605))


### Miscellaneous Chores

* **dashmate:** add platform testnet-37 support ([#1603](https://github.com/dashpay/platform/issues/1603))


### [0.25.16-rc.4](https://github.com/dashpay/platform/compare/v0.25.16-rc.3...v0.25.16-rc.4) (2023-11-24)


### ⚠ BREAKING CHANGES

* data contracts require position to be defined for object properties
* previously created state is not compatible

### Features

* **platform:** document serialization across versions ([#1594](https://github.com/dashpay/platform/issues/1594))


### Bug Fixes

* drive produces wrong app hash after restart ([#1586](https://github.com/dashpay/platform/issues/1586))


### [0.25.16-rc.3](https://github.com/dashpay/platform/compare/v0.25.16-rc.2...v0.25.16-rc.3) (2023-11-23)

### Bug Fixes

* **dashmate:** missing choices enumerator ([#1595](https://github.com/dashpay/platform/issues/1595))

### [0.25.16-rc.2](https://github.com/dashpay/platform/compare/v0.25.16-rc.1...v0.25.16-rc.2) (2023-11-22)

### Bug Fixes

* **dashmate:** incorrect imports  ([#1591](https://github.com/dashpay/platform/issues/1591))

### [0.25.16-rc.1](https://github.com/dashpay/platform/compare/v0.25.15...v0.25.16-rc.1) (2023-11-21)

### ⚠ BREAKING CHANGES

* upgrade to Node.JS v20 (#1562)

### Features

* **dashmate:** insight, core block explorer ([#1581](https://github.com/dashpay/platform/issues/1581))
* **dashmate:** update core version to 20.0.1 ([#1588](https://github.com/dashpay/platform/issues/1588))
* **dashmate:** upgrade dashmate to ESM syntax ([#1562](https://github.com/dashpay/platform/issues/1562))
* **package:** bump rust packages
* **sdk:** rs-sdk support for ExtendedEpochInfo::fetch()/fetch_many() ([#1576](https://github.com/dashpay/platform/issues/1576))

### Bug Fixes

* **dapi:** failure if bloom filter is not set ([#1572](https://github.com/dashpay/platform/issues/1572))
* **dashmate:** incorrect block height color on null remote block height ([#1587](https://github.com/dashpay/platform/issues/1587))


### Continuous Integration

* arch dependant yarn unplugged cache ([#1580](https://github.com/dashpay/platform/issues/1580))
* rs-sdk, feature and unnecessary deps testing for rust packages ([#1575](https://github.com/dashpay/platform/issues/1575))

### Miscellaneous Chores

* remove unused dependencies of rust crates ([#1578](https://github.com/dashpay/platform/issues/1578))
* upgrade to Yarn v4 ([#1562](https://github.com/dashpay/platform/issues/1562))
* upgrade to Node.JS v20 ([#1562](https://github.com/dashpay/platform/issues/1562))

### [0.25.15](https://github.com/dashpay/platform/compare/v0.25.13...v0.25.15) (2023-11-05)

### ⚠ BREAKING CHANGES

* dpp: allow only one document transition ([#1555](https://github.com/dashpay/platform/issues/1555))

### Continuous Integration
* remove unused and vulnerable github-api package ([#1571](https://github.com/dashpay/platform/issues/1571))
* bump rust packages versions in the release script by ([#1573](https://github.com/dashpay/platform/issues/1573))
* speed up workflows and reduce costs ([#1545](https://github.com/dashpay/platform/issues/1545))

### Features
* rust software development kit for Dash Platform ([#1475](https://github.com/dashpay/platform/issues/1475))

### [0.25.13](https://github.com/dashpay/platform/compare/v0.25.12...v0.25.13) (2023-11-05)


### Bug Fixes

* **dashmate:** config is not persisted after migration ([#1561](https://github.com/dashpay/platform/issues/1561))

### [0.25.12](https://github.com/dashpay/platform/compare/v0.25.11...v0.25.12) (2023-11-03)


### Code Refactoring

* rename protocol version endpoints ([#1552](https://github.com/dashpay/platform/issues/1552))


### Tests

* **dashmate:** fix migrations test ([#1547](https://github.com/dashpay/platform/issues/1547))


### Miscellaneous Chores

* **dashmate:** testnet-35 support ([#1559](https://github.com/dashpay/platform/issues/1559))
* **dashmate:** update tenderdash to v0.13.3

### [0.25.11](https://github.com/dashpay/platform/compare/v0.25.10...v0.25.11) (2023-11-02)


### Bug Fixes

* **dashmate:** config commands output ([#1556](https://github.com/dashpay/platform/issues/1556))

### [0.25.10](https://github.com/dashpay/platform/compare/v0.25.9...v0.25.10) (2023-11-02)

### Bug Fixes

* **dashmate:** invalid SSL path in the obtain command ([#1553](https://github.com/dashpay/platform/issues/1553))



### [0.25.9](https://github.com/dashpay/platform/compare/v0.25.8...v0.25.9) (2023-11-02)


### ⚠ BREAKING CHANGES

* DAPI proto messages are changed (#1542)
* Consensus rules for Identity Create and TopUp are changed. (#1510)
* Asset Lock Proof structure is changed  (#1510)
* Identity Create Transition balance validation requires correct amount (twice more) (#1510)
* **dashmate:** `enabledCount` is replaced with `masternodeEnabled` in the masternode status output (#1505)
* **dashmate:** SSL keys are now stored in the node's config subdirectory (#1502)

### Features

* **dashmate:** add cli command for core service ([#1501](https://github.com/dashpay/platform/issues/1501))
* **dashmate:** configure dashd command arguments ([#1520](https://github.com/dashpay/platform/issues/1520))
* **dashmate:** docker build command ([#1546](https://github.com/dashpay/platform/issues/1546))
* **dashmate:** docker build command ([#1546](https://github.com/dashpay/platform/issues/1546))
* **dashmate:** move ssl dir ([#1502](https://github.com/dashpay/platform/issues/1502))
* identity funding with asset lock special transactions ([#1510](https://github.com/dashpay/platform/issues/1510))
* **platform:** proto message and query versioning ([#1522](https://github.com/dashpay/platform/issues/1522))
* **platform:** query version upgrade (both votes and status) and epoch info ([#1542](https://github.com/dashpay/platform/issues/1542))


### Bug Fixes

* **dashmate:** payment queue and extend enabled count ([#1505](https://github.com/dashpay/platform/issues/1505))
* **dashmate:** reset command doesn't reset configs ([#1541](https://github.com/dashpay/platform/issues/1541))
* **drive:** mishandling internal errors as validation ones ([#1492](https://github.com/dashpay/platform/issues/1492))
* security advisories in browserify-sign and crypto-js ([#1548](https://github.com/dashpay/platform/issues/1548))


### Performance Improvements

* **dashmate:** disable config auto render ([#1499](https://github.com/dashpay/platform/issues/1499))
* **dashmate:** speedup container cleanup ([#1518](https://github.com/dashpay/platform/issues/1518))


### Continuous Integration

* fix s3 layer cache access forbidden on 8x runners ([#1521](https://github.com/dashpay/platform/issues/1521))


### Tests

* **dashmate:** successful migration test ([#1500](https://github.com/dashpay/platform/issues/1500))
* **test-suite:** add optional bail ([#1488](https://github.com/dashpay/platform/issues/1488))


### Build System

* update rust to 1.73 ([#1529](https://github.com/dashpay/platform/issues/1529))


### Code Refactoring

* remove js-dpp ([#1517](https://github.com/dashpay/platform/issues/1517))


### Documentation

* Update README.md to give information about correctly adding to correct Shell $PATH ([#1550](https://github.com/dashpay/platform/issues/1550))


### Miscellaneous Chores

* adds missing crypto js ([#1538](https://github.com/dashpay/platform/issues/1538))
* **dashmate:** set tenderdash logging level to info ([#1540](https://github.com/dashpay/platform/issues/1540))
* **dpp:** version `InstantAssetLockProof.validate_structure` ([#1549](https://github.com/dashpay/platform/issues/1549))

### [0.25.8](https://github.com/dashpay/platform/compare/v0.25.7...v0.25.8) (2023-10-20)


### Bug Fixes

* **drive-abci:** cached protocol versions ([#1516](https://github.com/dashpay/platform/issues/1516))


### Miscellaneous Chores

* **drive:** remove runtime state logging ([#1511](https://github.com/dashpay/platform/issues/1511))
* **drive:** set correct target for grove logs ([#1512](https://github.com/dashpay/platform/issues/1512))


### Continuous Integration

* c6id.8xlarge runners ([#1514](https://github.com/dashpay/platform/issues/1514))

### [0.25.7](https://github.com/dashpay/platform/compare/v0.25.6...v0.25.7) (2023-10-18)


### Miscellaneous Chores

* **drive:** add more logs ([#1506](https://github.com/dashpay/platform/issues/1506))

### [0.25.6](https://github.com/dashpay/platform/compare/v0.25.5...v0.25.6) (2023-10-18)


### ⚠ BREAKING CHANGES

* **drive:** remove extended quorum info from platform state (#1496)

### Miscellaneous Chores

* **drive:** fix logging levels ([#1495](https://github.com/dashpay/platform/issues/1495))
* **drive:** remove extended quorum info from platform state ([#1496](https://github.com/dashpay/platform/issues/1496))
* logging hex strings ([#1497](https://github.com/dashpay/platform/issues/1497))

### [0.25.3](https://github.com/dashpay/platform/compare/v0.25.2...v0.25.3) (2023-10-12)


### Bug Fixes

* **dashmate:** a testnet node fails to sync ([#1485](https://github.com/dashpay/platform/issues/1485))


### Documentation

* cleanup changelog

### [0.25.2](https://github.com/dashpay/platform/compare/v0.25.1...v0.25.2) (2023-10-11)


### Features

* **dashmate:** force start ([#1481](https://github.com/dashpay/platform/issues/1481))

### [0.25.1](https://github.com/dashpay/platform/compare/v0.25.0...v0.25.1) (2023-10-11)


### Bug Fixes

* **dashmate:** log permissions on linux system ([#1479](https://github.com/dashpay/platform/issues/1479))

## [0.25.0](https://github.com/dashpay/platform/compare/v0.25.0-dev.33...v0.25.0) (2023-10-10)


### ⚠ BREAKING CHANGES

* block results might be different while blockchain replying (#1464)
* **platform:** the default epoch length is changed (#1467)
* **drive-abci:** masternode operator keys are no longer added to the unique tree lookup (#1459)
* **drive:** contracts with arrays won't be valid anymore (#1457)
* **drive-abci:** small differences in serialization of Null value (#1456)
* the DPNS system data contract is changed so the previously created state won't be valid (#1454)
* drive volume is changed so volumes and containers must be recreated. It means platform file must be wiped (#1406)

### Features

* **dashmate:** add epochTime to abci config ([#1468](https://github.com/dashpay/platform/issues/1468))
* **dashmate:** tenderdash log file ([#1396](https://github.com/dashpay/platform/issues/1396))
* mitigate homograph attack in DPNS ([#1454](https://github.com/dashpay/platform/issues/1454))
* **platform:** configurable epoch time (default down to 9.125 days) ([#1467](https://github.com/dashpay/platform/issues/1467))


### Bug Fixes

* consensus error is missing in tx results ([#1458](https://github.com/dashpay/platform/issues/1458))
* **dashmate:** drive logs configuration ([#1406](https://github.com/dashpay/platform/issues/1406))
* **drive-abci:** double state transition with document same unique index ([#1456](https://github.com/dashpay/platform/issues/1456))
* **drive-abci:** masternode identities unique keys ([#1459](https://github.com/dashpay/platform/issues/1459))
* **drive:** deadlock in tenderdash abci client ([#1463](https://github.com/dashpay/platform/issues/1463))
* **drive:** document type doesn't match array value ([#1457](https://github.com/dashpay/platform/issues/1457))
* incorrect invalidation of data contract cache ([#1464](https://github.com/dashpay/platform/issues/1464))


### Documentation

* **dashmate:** typos in README ([#1256](https://github.com/dashpay/platform/issues/1256))


### Continuous Integration

* add missing Drive workflow ([#1461](https://github.com/dashpay/platform/issues/1461))
* disable runs on draft prs ([#1380](https://github.com/dashpay/platform/issues/1380))
* update action dependency versions ([#1449](https://github.com/dashpay/platform/issues/1449))


### Miscellaneous Chores

* add a breaking changes checkbox to the PR template ([#1455](https://github.com/dashpay/platform/issues/1455))
* **dashmate:** update config for testnet-26 ([#1470](https://github.com/dashpay/platform/issues/1470))
* **dashmate:** update core image ([#1469](https://github.com/dashpay/platform/issues/1469))
* **drive:** improve quorum info update logs ([#1444](https://github.com/dashpay/platform/issues/1444))
* **drive:** log grovedb operations ([#1446](https://github.com/dashpay/platform/issues/1446))
* increase scopes for ci ([#1460](https://github.com/dashpay/platform/issues/1460))
* removed old code ([#1471](https://github.com/dashpay/platform/issues/1471))


## [0.25.0-dev.33](https://github.com/dashpay/platform/compare/v0.25.0-dev.32...v0.25.0-dev.33) (2023-10-05)


### Features

* **dashmate:** upgrade core to v20.0.0-beta.2 ([#1436](https://github.com/dashpay/platform/issues/1436))
* **drive:** handlers error codes ([#1394](https://github.com/dashpay/platform/issues/1394))
* **sdk:** add logger to dapi client ([#1420](https://github.com/dashpay/platform/issues/1420))


### Continuous Integration

* **test-suite:** run sdk and wallet functional tests from all packages workflow ([#1438](https://github.com/dashpay/platform/issues/1438))

## [0.25.0-dev.32](https://github.com/dashpay/platform/compare/v0.25.0-dev.31...v0.25.0-dev.32) (2023-09-29)


### Performance Improvements

* **drive:** clear instead of delete for previous masternode version voting ([#1437](https://github.com/dashpay/platform/issues/1437))


### Miscellaneous Chores

* update config and migrations for testnet-25 ([#1435](https://github.com/dashpay/platform/issues/1435))

## [0.25.0-dev.31](https://github.com/dashpay/platform/compare/v0.25.0-dev.30...v0.25.0-dev.31) (2023-09-28)


### ⚠ BREAKING CHANGES

* **drive:** a unique key with that hash already exists (#1429)

### Features

* **drive:** drive-abci verify grovedb CLI ([#1427](https://github.com/dashpay/platform/issues/1427))


### Bug Fixes

* **drive:** a unique key with that hash already exists ([#1429](https://github.com/dashpay/platform/issues/1429))

## [0.25.0-dev.30](https://github.com/dashpay/platform/compare/v0.25.0-dev.29...v0.25.0-dev.30) (2023-09-28)

### Bug Fixes

* **drive:** update grovedb after just in time fix ([#1426](https://github.com/dashpay/platform/issues/1426))
* **drive:** non-deterministic extended quorum info ([#1425](https://github.com/dashpay/platform/issues/1425))

### Security Threats

* **drive:** chaijs/get-func-name vulnerable to ReDoS ([#1431](https://github.com/dashpay/platform/issues/1431))

### Miscellaneous Chores

* update seed ip addresses ([#1424](https://github.com/dashpay/platform/issues/1424))
* update s3 bucket ([#1430](https://github.com/dashpay/platform/issues/1430))


## [0.25.0-dev.29](https://github.com/dashpay/platform/compare/v0.25.0-dev.28...v0.25.0-dev.29) (2023-09-22)
### Features

* **dashmate:** expose tenderdash metics ([#1419](https://github.com/dashpay/platform/issues/1419))


### Bug Fixes

* **dapi:** getTransaction google-protobuf AssertionError ([#1416](https://github.com/dashpay/platform/issues/1416))
* **dashmate:** platform reset failure ([#1415](https://github.com/dashpay/platform/issues/1415))


### Miscellaneous Chores

* **dashmate:** add images migrations for v25 ([#1377](https://github.com/dashpay/platform/issues/1377))
* update testnet genesis config for testnet-24 ([#1413](https://github.com/dashpay/platform/issues/1413))


### Continuous Integration

* make self-hosted actions-cache steps access S3 from correct region ([#1407](https://github.com/dashpay/platform/issues/1407))
* update cache target bucket ([#1418](https://github.com/dashpay/platform/issues/1418))

## [0.25.0-dev.28](https://github.com/dashpay/platform/compare/v0.25.0-dev.27...v0.25.0-dev.28) (2023-09-19)


### Bug Fixes

* **dashmate:** config format is not conventional ([#1410](https://github.com/dashpay/platform/issues/1410))
* **dashmate:** ssl verification container already exists ([#1409](https://github.com/dashpay/platform/issues/1409))
* **drive:** withdrawal transactions query ([#1402](https://github.com/dashpay/platform/issues/1402))


### Styles

* **dpp:** remove unused cbor import ([#1384](https://github.com/dashpay/platform/issues/1384))

## [0.25.0-dev.27](https://github.com/dashpay/platform/compare/v0.25.0-dev.26...v0.25.0-dev.27) (2023-09-18)


### Bug Fixes

* **dashmate:** obtain new certificates with helper failure ([#1403](https://github.com/dashpay/platform/issues/1403))

## [0.25.0-dev.26](https://github.com/dashpay/platform/compare/v0.25.0-dev.25...v0.25.0-dev.26) (2023-09-16)

### Reverted

* drive file logs ([#1400]((https://github.com/dashpay/platform/pull/1400)))


## [0.25.0-dev.25](https://github.com/dashpay/platform/compare/v0.25.0-dev.24...v0.25.0-dev.25) (2023-09-15)

### Reverted

* make actions-cache steps access S3 using the correct region ([#1398](https://github.com/dashpay/platform/pull/1398))


## [0.25.0-dev.24](https://github.com/dashpay/platform/compare/v0.25.0-dev.23...v0.25.0-dev.24) (2023-09-15)


### Bug Fixes

* drive file logs ([#1395](https://github.com/dashpay/platform/issues/1395))


### Continuous Integration

* disable apt install step on macos release job ([#1390](https://github.com/dashpay/platform/issues/1390))
* fix cache mtime ([#1385](https://github.com/dashpay/platform/issues/1385))
* make actions-cache steps access S3 using the correct region ([#1391](https://github.com/dashpay/platform/issues/1391))


### Miscellaneous Chores

* **dashmate:** support new docker version ([#1393](https://github.com/dashpay/platform/issues/1393))
* update tenderdash to v0.13.1 ([#1392](https://github.com/dashpay/platform/issues/1392))

## [0.25.0-dev.23](https://github.com/dashpay/platform/compare/v0.25.0-dev.22...v0.25.0-dev.23) (2023-09-11)


### Bug Fixes

* **drive:** vote extensions are allowed only for the current block and round ([#1387](https://github.com/dashpay/platform/issues/1387))


### Continuous Integration

* macos dashmate build broken due to bad qemu install ([#1374](https://github.com/dashpay/platform/issues/1374))

## [0.25.0-dev.22](https://github.com/dashpay/platform/compare/v0.25.0-dev.21...v0.25.0-dev.22) (2023-09-08)


### Features

* **drive:** improve ABCI logging ([#1382](https://github.com/dashpay/platform/issues/1382))
* support a new dashcore version ([#1368](https://github.com/dashpay/platform/issues/1368))


### Bug Fixes

* **drive:** -32603 error code on broadcast ([#1381](https://github.com/dashpay/platform/issues/1381))
* **drive:** query fix for 1 value and no where clause ([#1378](https://github.com/dashpay/platform/issues/1378))

## [0.25.0-dev.21](https://github.com/dashpay/platform/compare/v0.25.0-dev.20...v0.25.0-dev.21) (2023-09-07)

## [0.25.0-dev.20](https://github.com/dashpay/platform/compare/v0.25.0-dev.19...v0.25.0-dev.20) (2023-09-06)


### Features

* **drive:** better error handling on check_tx ([#1372](https://github.com/dashpay/platform/issues/1372))


### Bug Fixes

* **drive:** deserialization and identity not existing on check_tx ([#1371](https://github.com/dashpay/platform/issues/1371))

## [0.25.0-dev.19](https://github.com/dashpay/platform/compare/v0.25.0-dev.18...v0.25.0-dev.19) (2023-09-06)


### Bug Fixes

* **dpp:** some contract options are updatable ([#1364](https://github.com/dashpay/platform/issues/1364))
* **drive:** invalid mn operator reward type ([#1366](https://github.com/dashpay/platform/issues/1366))

## [0.25.0-dev.18](https://github.com/dashpay/platform/compare/v0.25.0-dev.17...v0.25.0-dev.18) (2023-09-04)


### Features

* contract specified encryption/decryption keys (stage 1 contract bounds) ([#1358](https://github.com/dashpay/platform/issues/1358))


### Bug Fixes

* **dashmate:** helper container is restarting ([#1362](https://github.com/dashpay/platform/issues/1362))


### Miscellaneous Chores

* **dashmate:** bump core to 20.0.0-alpha.6 ([#1361](https://github.com/dashpay/platform/issues/1361))
* **dashmate:** remove sentinel service ([#1354](https://github.com/dashpay/platform/issues/1354))

## [0.25.0-dev.17](https://github.com/dashpay/platform/compare/v0.25.0-dev.16...v0.25.0-dev.17) (2023-08-31)


### ⚠ BREAKING CHANGES

* Some of the WASM DPP methods are disabled
* DataContract methods are renamed
* Raw data contract structure is changed


### Features

* add better JsonSchemaError messages ([#1341](https://github.com/dashpay/platform/issues/1341))
* validate with document type ([#1334](https://github.com/dashpay/platform/issues/1334))
* code versioning ([#1327](https://github.com/dashpay/platform/issues/1327))


### Bug Fixes

* dpp and drive-abci fail to build without default-features ([#1345](https://github.com/dashpay/platform/issues/1345))
* **dpp:** data contract facade and state transition bindings ([#1342](https://github.com/dashpay/platform/issues/1342))
* **dpp:** fixing identity transition bindings ([#1315](https://github.com/dashpay/platform/issues/1315))
* **dpp:** identity constructor ([#1336](https://github.com/dashpay/platform/issues/1336))
* **dpp:** identity facade ([#1329](https://github.com/dashpay/platform/issues/1329))
* **dpp:** wasm binding for DataContract ([#1333](https://github.com/dashpay/platform/issues/1333))
* network start for querying ([#1335](https://github.com/dashpay/platform/issues/1335))
* various fixes and todos for [#1334](https://github.com/dashpay/platform/issues/1334) ([#1337](https://github.com/dashpay/platform/issues/1337))
* wasm DPP binding and other ([#1352](https://github.com/dashpay/platform/issues/1352))


### Continuous Integration

* fix credentials could not be loaded error ([#1320](https://github.com/dashpay/platform/issues/1320))


### Code Refactoring

* rename serialize ([#1338](https://github.com/dashpay/platform/issues/1338))


### Tests

* **dpp:** temporary skip wasm-dpp tests ([#1328](https://github.com/dashpay/platform/issues/1328))
* enable data contract tests ([#1346](https://github.com/dashpay/platform/issues/1346))
* restore identity create ([#1339](https://github.com/dashpay/platform/issues/1339))


### Miscellaneous Chores

* npm audit fix ([#1321](https://github.com/dashpay/platform/issues/1321))
* **sdk:** temporary disable data contracts and documents ([#1331](https://github.com/dashpay/platform/issues/1331))
* update workflow_dispatch
* upgrade to Node.JS v18 LTS ([#1280](https://github.com/dashpay/platform/issues/1280))
* support Core v0.20.0-alpha.4 ([#1357](https://github.com/dashpay/platform/pull/1357))


### [0.24.23](https://github.com/dashpay/platform/compare/v0.24.22...v0.24.23) (2023-08-18)


### Miscellaneous Chores

* npm audit fix ([#1321](https://github.com/dashpay/platform/issues/1321))

### Continuous Integration

* ci: fix credentials could not be loaded error ([#1321](https://github.com/dashpay/platform/issues/1320))

### [0.24.22](https://github.com/dashevo/platform/compare/v0.24.21...v0.24.22) (2023-08-15)


### Features

* **dashmate:** render tenderdash node mode in the service config ([#1311](https://github.com/dashevo/platform/issues/1311))

### [0.24.21](https://github.com/dashpay/platform/compare/v0.24.20...v0.24.21) (2023-08-09)


### Bug Fixes

* **js-drive** tests after upgrade of wasm-bindgen to version 0.2.86 ([#1306](https://github.com/dashpay/platform/issues/1306))
* **release:** upgrade buildbase with wasm-bindgen 0.2.86 ([#1304](https://github.com/dashpay/platform/issues/1304))

### [0.24.20](https://github.com/dashpay/platform/compare/v0.24.19...v0.24.20) (2023-08-07)


### Bug Fixes

* **dashmate:** `--platform` flag is ignored ([#1287](https://github.com/dashpay/platform/issues/1287))
* **dashmate:** load external ip for evo fullnodes ([#1288](https://github.com/dashpay/platform/issues/1288))
* **dashmate:** missing default values in IP and ports form ([#1276](https://github.com/dashpay/platform/issues/1276))
* **dashmate:** some status errors is visible without DEBUG env ([#1299](https://github.com/dashpay/platform/issues/1299))
* **dashmate:** various status output issues ([#1274](https://github.com/dashpay/platform/issues/1274), [#1293](https://github.com/dashpay/platform/issues/1293))
* **dashmate:** invalid migration version ([#1285](https://github.com/dashpay/platform/issues/1285))


### Continuous Integration

* update self-hosted runner tags ([#1271](https://github.com/dashpay/platform/issues/1271))


### Build System

* update `wasm-bindgen-cli` to 0.2.86 ([#1289](https://github.com/dashpay/platform/issues/1289))


### Miscellaneous Chores

* **dashmate:** update Core to v19.3.0 ([#1284](https://github.com/dashpay/platform/issues/1284))


## [0.25.0-dev.16](https://github.com/dashpay/platform/compare/v0.25.0-dev.15...v0.25.0-dev.16) (2023-08-02)


### ⚠ BREAKING CHANGES

* Platform state is modified so previous data won't be valid

### Features

* abci versioning ([#1172](https://github.com/dashpay/platform/issues/1172))
* adapt GroveDB's API changes ([#1099](https://github.com/dashpay/platform/issues/1099))
* build dapi-gprc rust client ([#1182](https://github.com/dashpay/platform/issues/1182))
* **drive:** add block_id_hash, quorum_type and chain_id required to verify proofs to GRPC responses ([#1207](https://github.com/dashpay/platform/issues/1207))
* **drive:** core block reward distribution ([#1135](https://github.com/dashpay/platform/issues/1135))
* **drive:** ensure that chain lock height valid ([#1157](https://github.com/dashpay/platform/issues/1157))
* **drive:** fetch contract history and contract.is_readonly ([#1120](https://github.com/dashpay/platform/issues/1120))
* **drive:** graceful shutdown ([#1154](https://github.com/dashpay/platform/issues/1154))
* **drive:** verify chain lock Core RPC ([#1146](https://github.com/dashpay/platform/issues/1146))
* **drive:** verify instant lock Core RPC ([#1142](https://github.com/dashpay/platform/issues/1142))
* fetch data contract history endpoint ([#1149](https://github.com/dashpay/platform/issues/1149))
* identity credit transfer state transition ([#1138](https://github.com/dashpay/platform/issues/1138))
* remove bad masternodes from validator sets ([#1160](https://github.com/dashpay/platform/issues/1160))
* **sdk:** retry policy for newly created platform entities ([#1143](https://github.com/dashpay/platform/issues/1143))


### Bug Fixes

* **dapi:** invalid json response body ([#1150](https://github.com/dashpay/platform/issues/1150))
* double process proposal from Tenderdash restart ([#1165](https://github.com/dashpay/platform/issues/1165))
* **drive:** core RPC retry all errors ([#1140](https://github.com/dashpay/platform/issues/1140))
* **drive:** do not return an error for non existence contract when verifying ([#1241](https://github.com/dashpay/platform/issues/1241))
* **drive:** wait for core to sync ([#1153](https://github.com/dashpay/platform/issues/1153))
* update abci test state root ([#1144](https://github.com/dashpay/platform/issues/1144))


### Code Refactoring

* misc changes during v0.25 review ([#1121](https://github.com/dashpay/platform/issues/1121))
* update dapi proto file to use either proofs or result ([#1148](https://github.com/dashpay/platform/issues/1148))


### Tests

* **drive:** verify proof signatures in strategy tests ([#1147](https://github.com/dashpay/platform/issues/1147))


### Documentation

* backport changelog from v0.24
* better drive verify docs ([#1171](https://github.com/dashpay/platform/issues/1171))


### Build System

* fix dash sdk ts config
* remove unnecessary yarn installation


### Miscellaneous Chores

* add QuantumExplorer as Code Owner
* backport deps from master
* temp fix rust-dashcore-dependency
* update grovedb version
* update lock file
* upgrade to tenderdash v0.13 ([#1236](https://github.com/dashpay/platform/issues/1236))


### Continuous Integration

* runs not cancelled when PR is closed or merged ([#1234](https://github.com/dashpay/platform/issues/1234))
* s3 cache ([#1167](https://github.com/dashpay/platform/issues/1167))
* select workflow_id to cancel based on head_ref ([#1247](https://github.com/dashpay/platform/issues/1247))
* specify docker mount cache bucket name as variable ([#1252](https://github.com/dashpay/platform/issues/1252))
* switch to multi-runner stack ([#1268](https://github.com/dashpay/platform/issues/1268))

### [0.24.19](https://github.com/dashpay/platform/compare/v0.24.18...v0.24.19) (2023-07-28)


### Bug Fixes

* **dashmate:** `baseImage/build/context` invalid json schema ([#1269](https://github.com/dashpay/platform/issues/1269))

### [0.24.18](https://github.com/dashpay/platform/compare/v0.24.17...v0.24.18) (2023-07-26)


### Bug Fixes

* **dashmate:** the update command expects helper image option ([#1264](https://github.com/dashpay/platform/issues/1264))

### [0.24.17](https://github.com/dashpay/platform/compare/v0.24.16...v0.24.17) (2023-07-26)


### Features

* **dashmate:** pre-build image ([#1259](https://github.com/dashpay/platform/issues/1259))


### Bug Fixes

* **dashmate:** undefined createIpAndPortsForm factory ([#1258](https://github.com/dashpay/platform/issues/1258))
* **dashmate:** version color is red ([#1255](https://github.com/dashpay/platform/issues/1255))


### Code Refactoring

* **dashmate:** default configuration ([#1257](https://github.com/dashpay/platform/issues/1257))

### [0.24.16](https://github.com/dashpay/platform/compare/v0.24.15...v0.24.16) (2023-07-25)


### ⚠ BREAKING CHANGES

* **dashmate:** Removed `dashmate.helper.docker.image` configuration option (#1231)

### Features

* **dashmate:** allow separation of a service build ([#1206](https://github.com/dashpay/platform/issues/1206))
* **dashmate:** reindex reworked ([#1212](https://github.com/dashpay/platform/issues/1212))


### Bug Fixes

* **dashmate:** invalid testnet chain id ([#1233](https://github.com/dashpay/platform/issues/1233))
* **dashmate:** runs invalid helper version ([#1231](https://github.com/dashpay/platform/issues/1231))
* word wrap audit fail ([#1235](https://github.com/dashpay/platform/issues/1235))


### Miscellaneous Chores

* remove envoy build from dashmate and release ([#1232](https://github.com/dashpay/platform/issues/1232))


### Code Refactoring

* **dashmate:** dynamic home dir path ([#1237](https://github.com/dashpay/platform/issues/1237))
* **dashmate:** move all envs definition to `generateEnvs` ([#1246](https://github.com/dashpay/platform/issues/1246))


### Tests

* **dashmate:** increase the reliability of e2e tests ([#1204](https://github.com/dashpay/platform/issues/1204))

### [0.24.15](https://github.com/dashpay/platform/compare/v0.24.14...v0.24.15) (2023-07-10)


### Features

* **dashmate:** interface binding configuration ([#1220](https://github.com/dashpay/platform/issues/1220))

### Bug Fixes

* **dashmate:** service name variable name ([#1225](https://github.com/dashpay/platform/issues/1225))

### Miscellaneous Chores

* **dashmate:** update tenderdash genesis for testnet ([#1223](https://github.com/dashpay/platform/issues/1223))

### Continuous Integration

* temporary ignore gRPC JS vulnerability ([#1221](https://github.com/dashpay/platform/issues/1221))


### [0.24.14](https://github.com/dashpay/platform/compare/v0.24.13...v0.24.14) (2023-07-05)


### Features

* **dashmate:** report pulled images during update ([#1186](https://github.com/dashpay/platform/issues/1186), [#1213](https://github.com/dashpay/platform/issues/1213))


### Miscellaneous Chores

* **dashmate:** bump Core version to 19.2.0 ([#1211](https://github.com/dashpay/platform/issues/1211))

### [0.24.13](https://github.com/dashpay/platform/compare/v0.24.12...v0.24.13) (2023-06-29)


### Bug Fixes

* **dashmate:** invalid migration ([#1209](https://github.com/dashpay/platform/issues/1209))

### [0.24.12](https://github.com/dashpay/platform/compare/v0.24.11...v0.24.12) (2023-06-28)


### ⚠ BREAKING CHANGES

* **dashmate:** Default dashmate helper port changed from 9000 to 9100 (#1194)

### Features

* **dashmate:** configure tenderdash pprof ([#1201](https://github.com/dashpay/platform/issues/1201))
* **dashmate:** setup masternode with DMT ([#1203](https://github.com/dashpay/platform/issues/1203))


### Bug Fixes

* **dashmate:** helper default port was bound to Windows print port ([#1194](https://github.com/dashpay/platform/issues/1194))


### Code Refactoring

* unwanted usage of x11-hash-js ([#1191](https://github.com/dashpay/platform/issues/1191))


### Miscellaneous Chores

* **drive:** downgrade grovedb to supported version ([#1202](https://github.com/dashpay/platform/issues/1202))


### Tests

* **dashmate:** e2e tests ([#1152](https://github.com/dashpay/platform/issues/1152))

### [0.24.11](https://github.com/dashpay/platform/compare/v0.24.10...v0.24.11) (2023-06-23)


### Bug Fixes

* **drive:** cannot read properties of undefined (reading 'toJSON') ([#1196](https://github.com/dashpay/platform/issues/1196))

### [0.24.10](https://github.com/dashpay/platform/compare/v0.24.9...v0.24.10) (2023-06-23)


### Miscellaneous Chores

* **drive:** disable masternode identities update logic ([#1192](https://github.com/dashpay/platform/issues/1192))

### [0.24.9](https://github.com/dashpay/platform/compare/v0.24.8...v0.24.9) (2023-06-22)


### ⚠ BREAKING CHANGES

* **dashmate:** SSL certificates and other configuration files will be deleted with the hard reset command (#1188)

### Features

* **dashmate:** hard reset deletes related files ([#1188](https://github.com/dashpay/platform/issues/1188))


### Bug Fixes

* **dashmate:** download certificate retry logic ([#1187](https://github.com/dashpay/platform/issues/1187))
* merkle root from the diff doesn't match calculated merkle root ([#1189](https://github.com/dashpay/platform/issues/1189))


### Continuous Integration

* fix cancel runs for already merged PRs ([#1185](https://github.com/dashpay/platform/issues/1185))

### [0.24.8](https://github.com/dashpay/platform/compare/v0.24.7...v0.24.8) (2023-06-21)


### Features

* **dashmate:** re-use ZeroSSL private key ([#1180](https://github.com/dashpay/platform/issues/1180))


### Continuous Integration

* cancel runs for already merged PRs ([#1179](https://github.com/dashpay/platform/issues/1179))

### [0.24.7](https://github.com/dashpay/platform/compare/v0.24.6...v0.24.7) (2023-06-21)


### ⚠ BREAKING CHANGES

* **dashmate:** handle already configured certificate in obtain command (#1176)

### Features

* **dashmate:** handle already configured certificate in obtain command ([#1176](https://github.com/dashpay/platform/issues/1176))

### [0.24.6](https://github.com/dashpay/platform/compare/v0.24.5...v0.24.6) (2023-06-19)


### Features

* **dashmate:** add homedir to compose project name ([#1141](https://github.com/dashpay/platform/issues/1141))


### Bug Fixes

* **dashmate:** config/core/rpc/allowIps must be array ([#881](https://github.com/dashpay/platform/issues/881))
* **sdk:** identifier expects buffer with "in" operator in where query ([#1168](https://github.com/dashpay/platform/issues/1168))
* **sdk:** missing and invalid types ([#1156](https://github.com/dashpay/platform/issues/1156))
* **wallet-lib:** instant locks not arriving to HD wallets ([#1126](https://github.com/dashpay/platform/issues/1126))


### Miscellaneous Chores

* bump up dashd version for mainnet in dashmate ([#1132](https://github.com/dashpay/platform/issues/1132))
* update socket io parser ([#1127](https://github.com/dashpay/platform/issues/1127))

### [0.24.5](https://github.com/dashpay/platform/compare/v0.24.4...v0.24.5) (2023-05-22)


### Features

* **dashmate:** descriptions for all possible ZeroSSL errors ([#1107](https://github.com/dashpay/platform/issues/1107))


### Bug Fixes

* **dashmate:** platform should not be enabled on mainnet ([#1112](https://github.com/dashpay/platform/issues/1112))


### Miscellaneous Chores

* **dashmate:** better port labels for mainnet evolution node setup  ([#1106](https://github.com/dashpay/platform/issues/1106))

### [0.24.4](https://github.com/dashpay/platform/compare/v0.24.3...v0.24.4) (2023-05-18)


### Bug Fixes

* **dashmate:** dashmate helper crashing ([#1072](https://github.com/dashpay/platform/issues/1072))
* **dashmate:** unable to find compatible protocol ([#1102](https://github.com/dashpay/platform/issues/1102))


### Continuous Integration

* add a name to PR linter step ([#1103](https://github.com/dashpay/platform/issues/1103))

### [0.24.3](https://github.com/dashpay/platform/compare/v0.24.2...v0.24.3) (2023-05-16)


### Features

* **dashmate:** `no-retry` flag for ssl obtain command ([#1093](https://github.com/dashpay/platform/issues/1093))


### Miscellaneous Chores

* support GA certificates for testnet ([#1092](https://github.com/dashpay/platform/issues/1092))

### [0.24.2](https://github.com/dashpay/platform/compare/v0.24.1...v0.24.2) (2023-05-16)


### Features

* **dashmate:** obtain SSL certificate command ([#1088](https://github.com/dashpay/platform/issues/1088))
* **dpp:** document `$createdAt` and `$updatedAt` validation ([#948](https://github.com/dashpay/platform/issues/948))


### Bug Fixes

* **dashmate:** status command fails with errors ([#1059](https://github.com/dashpay/platform/issues/1059))

### [0.24.1](https://github.com/dashpay/platform/compare/v0.24.0...v0.24.1) (2023-05-15)


### Features

* **dashmate:** platform flag for start, stop and restart commands ([#1063](https://github.com/dashpay/platform/issues/1063))


### Bug Fixes

* **dapi:** can't connect to testnet with default seeds ([#1084](https://github.com/dashpay/platform/issues/1084))
* **dashmate:** check core is started checks everytime ([#1071](https://github.com/dashpay/platform/issues/1071))
* **dashmate:** incorrect reset command prompt in case network is already set up ([#1064](https://github.com/dashpay/platform/issues/1064))
* **dashmate:** outdated docker images and missed migration  ([#1069](https://github.com/dashpay/platform/issues/1069))


### Miscellaneous Chores

* **dashmate:** rename high-performance nodes to evo nodes ([#1062](https://github.com/dashpay/platform/issues/1062))
* **drive:** payout fees only to single well-known Identity ([#1078](https://github.com/dashpay/platform/issues/1078))

## [0.24.0](https://github.com/dashpay/platform/compare/v0.24.0-dev.34...v0.24.0) (2023-05-10)

### Notes

The masternode identities logic is partially disabled due to incomplete Identity V2 implementation and will be enabled back in v0.25.0


### ⚠ BREAKING CHANGES

* Previous blockchain data and state is not compatible
* Previous created compose projects won't be supported. Please destroy them before update (#1055)
* The --platfrom-only flag is renamed to --platform in the reset command (#991)
* Some wasm-dpp APIs are different to js-dpp ones. The only divergencies that were addressed are the ones that were visible in tests, but some others might've been left intact (#848)
* Core version less than 19 and legacy BLS schema are not supported (#771)
* gRPC and HTTP platform ports now handling with the one single port (#752)
* State Transition fees are changed
* Document's query validation logic has minor incompatibilities with previous version

### Features

* **drive:** whitelist and filter banned nodes for validators ([#1034](https://github.com/dashpay/platform/issues/1034))
* **dashmate:** core log file and debug categories ([#913](https://github.com/dashpay/platform/issues/913))
* **dashamte:** better ZeroSSL error messages ([#950](https://github.com/dashpay/platform/issues/950))
* **dashmate:** set random core rpc usename and password on setup ([#973](https://github.com/dashpay/platform/issues/973))
* **dashmate:** verbose `connect ENOENT /var/run/docker.sock` error ([#951](https://github.com/dashpay/platform/issues/951))
* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))
* **dashmate:** check system requirements before setup ([#935](https://github.com/dashpay/platform/issues/935))
* **drive:** handle quorum rotation failure ([#858](https://github.com/dashpay/platform/issues/858))
* wasm-dpp integration ([#848](https://github.com/dashpay/platform/issues/848))
* **dashmate:** build linux tarballs ([#887](https://github.com/dashpay/platform/issues/887))
* **dashmate:** build services before restart ([#894](https://github.com/dashpay/platform/issues/894))
* **dashmate:** exit status with 2 if it's not running ([#896](https://github.com/dashpay/platform/issues/896))
* **dashmate:** implement http json rpc api ([#888](https://github.com/dashpay/platform/issues/888))
* **dashmate:** tenderdash latest block time in status ([#906](https://github.com/dashpay/platform/issues/906))
* **dpp:** serialize consensus errors ([#871](https://github.com/dashpay/platform/issues/871))
* drive verification c bindings ([#860](https://github.com/dashpay/platform/issues/860))
* **dashmate:** add new quroum in dashcore config ([#862](https://github.com/dashpay/platform/issues/862))
* **dashmate:** enable platform option ([#853](https://github.com/dashpay/platform/issues/853))
* **dashmate:** generate self-signed certificates in the `setup` command ([#869](https://github.com/dashpay/platform/issues/869))
* **dashmate:** high-performance nodes registration with `setup` command ([#794](https://github.com/dashpay/platform/issues/794))
* **dashmate:** hint to setup a node on start failure ([#866](https://github.com/dashpay/platform/issues/866))
* **dpp:** add fees API  to rust wasm bindings ([#830](https://github.com/dashpay/platform/issues/830))
* **dpp:** optional execution context in rs-dpp ([#811](https://github.com/dashpay/platform/issues/811))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **rs-dpp:** migrate fees from js-dpp v0.24 ([#851](https://github.com/dashpay/platform/issues/851))
* state transition conversion ([#844](https://github.com/dashpay/platform/issues/844))
* **wasm-dpp:** add tests for state transition basic validator ([#857](https://github.com/dashpay/platform/issues/857))
* **wasm-dpp:** DashPlatformProtocol tests ([#841](https://github.com/dashpay/platform/issues/841))
* **wasm-dpp:** identity transitions additional functionality ([#855](https://github.com/dashpay/platform/issues/855))
* **wasm-dpp:** implement validateStateTransitionStateFactory tests ([#861](https://github.com/dashpay/platform/issues/861))
* **wasm-dpp:** provide external entropy generator to document factory ([#845](https://github.com/dashpay/platform/issues/845))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))
* better Core 19 support ([#832](https://github.com/dashpay/platform/issues/832))
* core version 19.0-beta integration ([#771](https://github.com/dashpay/platform/issues/771))
* **dashmate:** register HPMN for local network ([#796](https://github.com/dashpay/platform/issues/796))
* **dasmate:** pack release script ([#781](https://github.com/dashpay/platform/issues/781))
* **dpp:** identity facade ([#782](https://github.com/dashpay/platform/issues/782))
* **dpp:** integration tests for wasm-dpp document transitions ([#777](https://github.com/dashpay/platform/issues/777))
* **dpp:** wasm bindings for Documents related validations ([#709](https://github.com/dashpay/platform/issues/709))
* **dpp:** wasm-dpp: integration tests for document ([#762](https://github.com/dashpay/platform/issues/762))
* Identity v2 ([#705](https://github.com/dashpay/platform/issues/705))
* platform value abstraction ([#805](https://github.com/dashpay/platform/issues/805))
* proposer signaling of protocol version upgrade and fork activation ([#778](https://github.com/dashpay/platform/issues/778))
* register system data contracts in RS Drive ([#776](https://github.com/dashpay/platform/issues/776))
* **rs-dpp:**  dashpay datatrigger toUserIds better validation ([#799](https://github.com/dashpay/platform/issues/799))
* **rs-dpp:** backport of index_definitions.unique validation ([#802](https://github.com/dashpay/platform/issues/802))
* **rs-dpp:** backports of identity/stateTransition from js-dpp ([#800](https://github.com/dashpay/platform/issues/800))
* **rs-dpp:** introduce `StateTransitionFactory` ([#810](https://github.com/dashpay/platform/issues/810))
* **rs-dpp:** validate indices are backwards compatible backport ([#797](https://github.com/dashpay/platform/issues/797))
* **rs-drive:** verification feature ([#803](https://github.com/dashpay/platform/issues/803))
* **wasm dpp:** validate state transition key signature ([#806](https://github.com/dashpay/platform/issues/806))
* **wasm-dpp:**  wasm bindings for Document Transitions  ([#707](https://github.com/dashpay/platform/issues/707))
* **wasm-dpp:** async state repository ([#766](https://github.com/dashpay/platform/issues/766))
* **wasm-dpp:** data contract facade ([#716](https://github.com/dashpay/platform/issues/716))
* **wasm-dpp:** Fix metadata, metadata tests and backport v23 matedata changes into wasm-dpp ([#819](https://github.com/dashpay/platform/issues/819))
* **wasm-dpp:** implement identity update transition ([#748](https://github.com/dashpay/platform/issues/748))
* **wasm-dpp:** integration tests validate data contract update transition ([#812](https://github.com/dashpay/platform/issues/812))
* **wasm-dpp:** protocol version validator tests ([#823](https://github.com/dashpay/platform/issues/823))
* **wasm-dpp:** remove unused documents factory tests ([#828](https://github.com/dashpay/platform/issues/828))
* **wasm-dpp:** state transition facade ([#814](https://github.com/dashpay/platform/issues/814))
* withdrawals status sync ([#679](https://github.com/dashpay/platform/issues/679))
* allow to get drive status from dashmate helper ([#749](https://github.com/dashpay/platform/issues/749))
* allow to get drive's status from dashmate helper ([#755](https://github.com/dashpay/platform/issues/755))
* **dapi:** use single envoy port for all connections ([#752](https://github.com/dashpay/platform/issues/752))
* **dashmate:** update Core to 18.2.0 ([#735](https://github.com/dashpay/platform/issues/735))
* **drive:** ABCI context logger ([#693](https://github.com/dashpay/platform/issues/693))
* **drive:** log contractId in deliverTx handler ([#730](https://github.com/dashpay/platform/issues/730))
* **drive:** log number of refunded epochs ([#729](https://github.com/dashpay/platform/issues/729))
* integrate wasm Document into JS tests ([#644](https://github.com/dashpay/platform/issues/644))
* varint protocol version ([#758](https://github.com/dashpay/platform/issues/758))
* **wasm-dpp:** implement function to produce generics from JsValue ([#712](https://github.com/dashpay/platform/issues/712))
* **wasm-dpp:** implement identity create transition ([#697](https://github.com/dashpay/platform/issues/697))
* **wasm-dpp:** implement identity topup transition ([#745](https://github.com/dashpay/platform/issues/745))
* **wasm-dpp:** Wasm dpp integration tests validate data contract factory ([#751](https://github.com/dashpay/platform/issues/751))
* credit refunds ([#662](https://github.com/dashpay/platform/issues/662))
* **dashmate:** additional dashd options ([#692](https://github.com/dashpay/platform/issues/692))
* **dashmate:** pass ZeroSSL as command line parameter ([#651](https://github.com/dashpay/platform/issues/651))
* **dashmate:** remove axios from zerossl requests
* **dashmate:** remove axios from zerossl requests
* **dpp:** AbstractConsensusError tests and extensions ([#670](https://github.com/dashpay/platform/issues/670))
* **dpp:** Data Contract Update Transition wasm binding ([#696](https://github.com/dashpay/platform/issues/696))
* **drive:** do not switch to validator quorum which will be removed soon ([#616](https://github.com/dashpay/platform/issues/616))
* multiple documents changes per batch and support for GroveDB 0.9 ([#699](https://github.com/dashpay/platform/issues/699))
* Consensus Errors and ValidationResult bindings ([#643](https://github.com/dashpay/platform/issues/643))
* average estimated processing fees ([#642](https://github.com/dashpay/platform/issues/642))
* **dpp:** bls adapter for WASM DPP ([#633](https://github.com/dashpay/platform/issues/633))
* **drive:** add time and protocolVersion fields to query metadata response ([#611](https://github.com/dashpay/platform/issues/611))
* **drive:** precise fees (dashpay/rs-platform[#170](https://github.com/dashpay/platform/issues/170)), closes [dashpay/rs-platform#181](https://github.com/dashpay/rs-platform/issues/181)
* **drive:** provide latest core chain lock on init chain ([#659](https://github.com/dashpay/platform/issues/659))
* **drive:** support for V0.7 of groveDB ([#665](https://github.com/dashpay/platform/issues/665))
* **drive:** use proposal block execution context in state repository ([#653](https://github.com/dashpay/platform/issues/653))
* **drive:** use single block execution context ([#627](https://github.com/dashpay/platform/issues/627))
* external bls validtor (dashpay/rs-platform[#186](https://github.com/dashpay/platform/issues/186))
* insert with parents for `Document` (dashpay/rs-platform[#189](https://github.com/dashpay/platform/issues/189))
* add `withdrawals` data contract package ([#604](https://github.com/dashpay/platform/issues/604))
* **done:** changes needed for wasm-dpp integration (dashpay/rs-platform[#154](https://github.com/dashpay/platform/issues/154))
* **dpp:** [v23 port]  validate fee calculating worst case operations (dashpay/rs-platform[#160](https://github.com/dashpay/platform/issues/160))
* **dpp:** dashpay datatrigger toUserIds better validation ([#620](https://github.com/dashpay/platform/issues/620))
* **drive:** select the most vital validator set quorums ([#617](https://github.com/dashpay/platform/issues/617))
* **dpp:** initial RS DPP integration ([#483](https://github.com/dashpay/platform/issues/483))
* **drive:** same block execution ([#593](https://github.com/dashpay/platform/issues/593))
* **node:** multiple transactions (dashpay/rs-platform[#155](https://github.com/dashpay/platform/issues/155))
* **drive:** AssetUnlock transactions processing ([#530](https://github.com/dashpay/platform/issues/530))
* withdrawal request queue (dashpay/rs-platform[#149](https://github.com/dashpay/platform/issues/149))
* Public Keys Identities Proofs (dashpay/rs-platform[#151](https://github.com/dashpay/platform/issues/151))
*  [v23 port]  data contract indices validation (dashpay/rs-platform[#26](https://github.com/dashpay/platform/issues/26))
* a temporary plug for dry run (dashpay/rs-platform[#113](https://github.com/dashpay/platform/issues/113))
* ability to get elements by $id directly (dashpay/rs-platform[#61](https://github.com/dashpay/platform/issues/61))
* add `proveDocumentsQuery` to Node.JS binding (dashpay/rs-platform[#115](https://github.com/dashpay/platform/issues/115))
* add `proveQueryMany` to Node.JS binding (dashpay/rs-platform[#122](https://github.com/dashpay/platform/issues/122))
* add constructor for DocumentTransition
* add credits converter
* add Document Transition
* add GroveDB methods to JS wrapper
* add hash implementation for identifier
* add prefixes to errors (dashpay/rs-platform[#101](https://github.com/dashpay/platform/issues/101))
* add proof test to rs-drive query tests (dashpay/rs-platform[#109](https://github.com/dashpay/platform/issues/109))
* allow one character property names
* batch support (dashpay/rs-platform[#111](https://github.com/dashpay/platform/issues/111))
* binding for Node.JS
* **dashmate:** update tenderdash to 0.9.0-dev.1 ([#525](https://github.com/dashpay/platform/issues/525))
* **dashmate:** zeroSSL certificate renewal helper ([#554](https://github.com/dashpay/platform/issues/554))
* **dpp:**  [v23 port] add withdraw puprose to identity public key  (dashpay/rs-platform[#134](https://github.com/dashpay/platform/issues/134))
* **dpp:** [v23 port] Identity Update Transition (dashpay/rs-platform[#138](https://github.com/dashpay/platform/issues/138))
* **dpp:** [v23 port] limit the number of shares for masternode by 16 (dashpay/rs-platform[#141](https://github.com/dashpay/platform/issues/141))
* **dpp:** add wasm-dpp template package ([#529](https://github.com/dashpay/platform/issues/529))
* **dpp:** basic validtion for state transition (dashpay/rs-platform[#133](https://github.com/dashpay/platform/issues/133))
* fees distribution (dashpay/rs-platform[#105](https://github.com/dashpay/platform/issues/105))
* identity create state transition (dashpay/rs-platform[#9](https://github.com/dashpay/platform/issues/9))
* identity credit withdrawal transition (dashpay/rs-platform[#25](https://github.com/dashpay/platform/issues/25))
* identity from_buffer and from_object
* immutibility and contracts that allow document history (dashpay/rs-platform[#79](https://github.com/dashpay/platform/issues/79))
* implement `deleteDocument`
* implement `updateDocument`
* implement applyContact and createDocument
* implement grovedb "bindings" in rs-drive
* implement masternode voting identities ([#467](https://github.com/dashpay/platform/issues/467))
* implement queryDocuments + some fixes
* implementation of Document validator
* Include the DPP into Drive (dashpay/rs-platform[#126](https://github.com/dashpay/platform/issues/126))
* insert identities (dashpay/rs-platform[#99](https://github.com/dashpay/platform/issues/99))
* migrate to ABCI++ ([#464](https://github.com/dashpay/platform/issues/464))
* **node:** introduce GroveDB#proveQuery (dashpay/rs-platform[#112](https://github.com/dashpay/platform/issues/112))
* populate stack for binding errors (dashpay/rs-platform[#39](https://github.com/dashpay/platform/issues/39))
* query drive with sql  (dashpay/rs-platform[#31](https://github.com/dashpay/platform/issues/31)), closes [dashpay/rs-platform#42](https://github.com/dashpay/rs-platform/issues/42)
* **query:** allow query with `$id` (dashpay/rs-platform[#53](https://github.com/dashpay/platform/issues/53))
* recursive conditional subqueries (dashpay/rs-platform[#106](https://github.com/dashpay/platform/issues/106))
* return processing cost for `queryDocuments` (dashpay/rs-platform[#100](https://github.com/dashpay/platform/issues/100))
* sql in clause (dashpay/rs-platform[#52](https://github.com/dashpay/platform/issues/52))
* SSL certificate for DAPI ([#519](https://github.com/dashpay/platform/issues/519))
* storage run time fees, worst case scenario fees and support for contract definition references (dashpay/rs-platform[#95](https://github.com/dashpay/platform/issues/95)), closes [dashpay/rs-platform#87](https://github.com/dashpay/rs-platform/issues/87) [dashpay/rs-platform#93](https://github.com/dashpay/rs-platform/issues/93) [dashpay/rs-platform#92](https://github.com/dashpay/rs-platform/issues/92)
* **tests:** add `reference` js test case (dashpay/rs-platform[#43](https://github.com/dashpay/platform/issues/43))
* update to latest grovedb and some optimization around inserts (dashpay/rs-platform[#120](https://github.com/dashpay/platform/issues/120)), closes [dashpay/rs-platform#119](https://github.com/dashpay/rs-platform/issues/119)
* verbose `startAt` or `startAfter` not found error (dashpay/rs-platform[#76](https://github.com/dashpay/platform/issues/76))


### Bug Fixes

* **dashmate:** external IP detection hangs sometimes ([#1053](https://github.com/dashpay/platform/issues/1053))
* **dapi:** invalid addresses in the whitelist ([#1044](https://github.com/dashpay/platform/issues/1044))
* **dashmate:** reset platform commands hangs ([#1038](https://github.com/dashpay/platform/issues/1038))
* **dashmate:** set permissions for dashcore log file ([#1037](https://github.com/dashpay/platform/issues/1037))
* **drive:** cannot destructure property ‘quorumHash’ of ‘instantLock... ([#1046](https://github.com/dashpay/platform/issues/1046))
* **drive:** cannot read properties of undefined (reading 'toString') ([#1045](https://github.com/dashpay/platform/issues/1045))
* **dashmate:** waitForQuorumConnections deadline of 300000 exceeded ([#1015](https://github.com/dashpay/platform/issues/1015))
* **dashmate:** wrong volume removal retry logic ([#1016](https://github.com/dashpay/platform/issues/1016))
* expect platformNodeID to be a hex string ([#1013](https://github.com/dashpay/platform/issues/1013))
* **dashmate:** "volume is in use" and "no such volume" ([#1005](https://github.com/dashpay/platform/issues/1005))
* **dashmate:** reset platform affects core ([#1001](https://github.com/dashpay/platform/issues/1001))
* **drive:** double init chain leads to side bugs ([#1002](https://github.com/dashpay/platform/issues/1002))
* **dashmate:** BLS private key validate accepts whitespaces ([#998](https://github.com/dashpay/platform/issues/998))
* **dashmate:** can't remove volumes if they not exist ([#997](https://github.com/dashpay/platform/issues/997))
* **dashmate:** show masternode state while it is not synced ([#999](https://github.com/dashpay/platform/issues/999))
* **dashmate:** wrap register masternode command in small terminals ([#996](https://github.com/dashpay/platform/issues/996))
* **dashmate:** multiple issues in the reset command ([#991](https://github.com/dashpay/platform/issues/991))
* **drive:** non-unique masternode voting keys ([#986](https://github.com/dashpay/platform/issues/986))
* **dashmate:** ability to work with non-default docker socket path ([#967](https://github.com/dashpay/platform/issues/967))
* **dashmate:** already configured preset is ignored ([#974](https://github.com/dashpay/platform/issues/974))
* **dashmate:** empty masternode status while syncing ([#970](https://github.com/dashpay/platform/issues/970))
* **dashmate:** form accepts invalid BLS key ([#961](https://github.com/dashpay/platform/issues/961))
* **dashmate:** invalid mn register command in output of dashmate setup ([#959](https://github.com/dashpay/platform/issues/959))
* **dashmate:** reward shares can be negative during HP masternode registration ([#960](https://github.com/dashpay/platform/issues/960))
* **dashmate:** select number of masternodes during local setup freezes ([#957](https://github.com/dashpay/platform/issues/957))
* **dashmate:** yaml package security vulnerability ([#975](https://github.com/dashpay/platform/issues/975))
* **test-suite:** expects identities for invalid mns ([#968](https://github.com/dashpay/platform/issues/968))
* **dashamte:** wrong envoy config path on windows ([#949](https://github.com/dashpay/platform/issues/949))
* **drive:** updating a masternode identity with invalid entry from SML ([#965](https://github.com/dashpay/platform/issues/965))
* **dashamte:** Can't find begoo/index with `yarn dashmate setup` ([#933](https://github.com/dashpay/platform/issues/933))
* DAPI still expected on normal masternodes ([#904](https://github.com/dashpay/platform/issues/904))
* **dapi-client:** platform port is ignored from SML ([#903](https://github.com/dashpay/platform/issues/903))
* **dashmate:** api binds to all interfaces ([#893](https://github.com/dashpay/platform/issues/893))
* **dashmate:** dashmate helper is running under root user ([#895](https://github.com/dashpay/platform/issues/895))
* **dashmate:** dashmate logic doesn't recognize it's ran from helper ([#902](https://github.com/dashpay/platform/issues/902))
* **dashmate:** missing rawblock zmq message in core config ([#770](https://github.com/dashpay/platform/issues/770))
* **dashmate:** undefined wallet for dash-cli ([#786](https://github.com/dashpay/platform/issues/786))
* **dpp:** various fixes in DPP and system contracts ([#907](https://github.com/dashpay/platform/issues/907))
* **drive:** non-deterministic run of mn identities sync ([#910](https://github.com/dashpay/platform/issues/910))
* **drive:** total HPMNs contains all masternodes ([#911](https://github.com/dashpay/platform/issues/911))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* llmqType must be equal to one of the allowed values ([#884](https://github.com/dashpay/platform/issues/884))
* possible overflow issues ([#877](https://github.com/dashpay/platform/issues/877))
* **dashmate:** ambiguous validation errors for file certificates ([#870](https://github.com/dashpay/platform/issues/870))
* **dashmate:** config.isPlatformEnabled is not a function ([#872](https://github.com/dashpay/platform/issues/872))
* **dpp:** incorrect public key validator schema in ST Facade ([#854](https://github.com/dashpay/platform/issues/854))
* **scripts:** update configure_test_network for hpmn ([#863](https://github.com/dashpay/platform/issues/863))
* **wasm-dpp:** fix decoding protocol version varint error to match previous implementation ([#849](https://github.com/dashpay/platform/issues/849))
* **ci:** fix release workflow syntax error ([#808](https://github.com/dashpay/platform/issues/808))
* **dashmate:** make dashmate helper run commands as host user ([#765](https://github.com/dashpay/platform/issues/765))
* **dashmate:** visual fixes for dashmate status ([#787](https://github.com/dashpay/platform/issues/787))
* **dpp:** update jsonschema-rs and enable tests ([#780](https://github.com/dashpay/platform/issues/780))
* **rs-dpp:** fetch latest core chain locked height misuse ([#789](https://github.com/dashpay/platform/issues/789))
* update webpack to resolve npm audit error ([#822](https://github.com/dashpay/platform/issues/822))
* **wasm-dpp:** Identifier and its tests ([#821](https://github.com/dashpay/platform/issues/821))
* **dashmate:** Dash Core container is unable to restart properly under WSL ([#736](https://github.com/dashpay/platform/issues/736))
* **dashmate:** fix migration for configs without platform ([#738](https://github.com/dashpay/platform/issues/738))
* **dashmate:** migrations fixes ([#759](https://github.com/dashpay/platform/issues/759))
* **dpp:** existing property in a new index ([#694](https://github.com/dashpay/platform/issues/694))
* ua-parser-js vulnerability  ([#756](https://github.com/dashpay/platform/issues/756))
* **dashmate:** ZeroSSL certificate cannot be downloaded ([#718](https://github.com/dashpay/platform/issues/718))
* **dpp:** can’t create fingerprint from a document transition ([#723](https://github.com/dashpay/platform/issues/723))
* **drive:** merk caching in contract caching (irony) ([#710](https://github.com/dashpay/platform/issues/710))
* find_duplicates_by_id.rs not compiling ([#702](https://github.com/dashpay/platform/issues/702))
* Starcounter-Jack JSON-Patch Prototype Pollution vulnerability ([#708](https://github.com/dashpay/platform/issues/708))
* **dashmate:** setDay is not a function ([#677](https://github.com/dashpay/platform/issues/677))
* **dashmate:** ZeroSSL certificates are not saved in WSL ([#676](https://github.com/dashpay/platform/issues/676))
* **drive:** initChain handler is not idempotent ([#675](https://github.com/dashpay/platform/issues/675))
* **dashmate:** SSL domain verification config could not be generated in WSL ([#673](https://github.com/dashpay/platform/issues/673))
* build not working because of deprecated wasm-bindgen feature ([#639](https://github.com/dashpay/platform/issues/639))
* **dapi:** fail to reconnect to tenderdash in case of ENOTFOUND ([#621](https://github.com/dashpay/platform/issues/621))
* **dashmate:** broken helper docker image ([#630](https://github.com/dashpay/platform/issues/630))
* **dashmate:** outdated Drive and DAPI images ([#668](https://github.com/dashpay/platform/issues/668))
* **dashmate:** ZeroSSL certificate renewal ([#624](https://github.com/dashpay/platform/issues/624))
* **drive:** invalid create name
* **drive:** multi transactions doesn't work properly ([#636](https://github.com/dashpay/platform/issues/636))
* **drive:** remove ambiguous use
* DataContract.spec.js in wasm-dpp ([#618](https://github.com/dashpay/platform/issues/618))
* **dpp:**  [v23 port] cannot read properties of null (reading 'getBalance') (dashpay/rs-platform[#163](https://github.com/dashpay/platform/issues/163))
* **dpp:** [v23 port] non-deterministic fees due to data contract cache (dashpay/rs-platform[#161](https://github.com/dashpay/platform/issues/161))
* **dpp:** [v23 port] repeated disabling of already disabled identity key (dashpay/rs-platform[#162](https://github.com/dashpay/platform/issues/162))
* renamed method from rs-dpp ([#623](https://github.com/dashpay/platform/issues/623))
* `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** `getConsensusParamsHandler` was handling wrong Tendermint error ([#601](https://github.com/dashpay/platform/issues/601))
* **dashmate:** invalid testnet TenderDash genesis ([#608](https://github.com/dashpay/platform/issues/608))
* **dashmate:** SSL verification server cannot be started ([#606](https://github.com/dashpay/platform/issues/606))
* typo `dash-amte` to `dashmate` ([#599](https://github.com/dashpay/platform/issues/599))
* **dapi-client:** temporary use http protocol by default ([#573](https://github.com/dashpay/platform/issues/573))
* using `ProtocolError ` in `cbor_value_to_json_value ` could lead to a stackoverflow error (dashpay/rs-platform[#164](https://github.com/dashpay/platform/issues/164))
* "number" property type is not implemented (dashpay/rs-platform[#47](https://github.com/dashpay/platform/issues/47))
* `Identity.balance` was of type `i64` but should be `u64` (dashpay/rs-platform[#23](https://github.com/dashpay/platform/issues/23))
* appendStack is not present in NPM package (dashpay/rs-platform[#41](https://github.com/dashpay/platform/issues/41))
* **ci:** support alpha prereleases ([#560](https://github.com/dashpay/platform/issues/560))
* comply with newest grovedb (dashpay/rs-platform[#121](https://github.com/dashpay/platform/issues/121))
* contract parsing errors
* create and update document in different transactions (dashpay/rs-platform[#68](https://github.com/dashpay/platform/issues/68))
* create two documents in different transactions (dashpay/rs-platform[#69](https://github.com/dashpay/platform/issues/69))
* delete empty trees (dashpay/rs-platform[#49](https://github.com/dashpay/platform/issues/49))
* **dpp:** [v23 port]  change allowed security level for withdrawal purpose to critical (dashpay/rs-platform[#140](https://github.com/dashpay/platform/issues/140))
* duplicate batched storage fee update (dashpay/rs-platform[#150](https://github.com/dashpay/platform/issues/150))
* fix build when using grovedb master
* fix paths
* fix some tests
* handle key not found error from grovedb (dashpay/rs-platform[#33](https://github.com/dashpay/platform/issues/33))
* index already exists on update document (dashpay/rs-platform[#64](https://github.com/dashpay/platform/issues/64))
* many insert commit fail (dashpay/rs-platform[#45](https://github.com/dashpay/platform/issues/45))
* merging equal path queries (dashpay/rs-platform[#128](https://github.com/dashpay/platform/issues/128))
* merging required properties in Data Cotnract
* neon security vulnerability (dashpay/rs-platform[#110](https://github.com/dashpay/platform/issues/110))
* non-deterministic apply contract (dashpay/rs-platform[#46](https://github.com/dashpay/platform/issues/46))
* non-present optional fields shouldn't be indexed
* order by on non equal fields (dashpay/rs-platform[#37](https://github.com/dashpay/platform/issues/37))
* owner id and additional tests (dashpay/rs-platform[#59](https://github.com/dashpay/platform/issues/59))
* path_queries can only refer to items and references (dashpay/rs-platform[#88](https://github.com/dashpay/platform/issues/88))
* primary key tree is not present (dashpay/rs-platform[#74](https://github.com/dashpay/platform/issues/74))
* query empty contract fails (dashpay/rs-platform[#65](https://github.com/dashpay/platform/issues/65))
* query validation logic (dashpay/rs-platform[#104](https://github.com/dashpay/platform/issues/104))
* remove prebuilds at pretest stage to prevent random mocha error process killed
* strange error on deletion of specific data set (dashpay/rs-platform[#90](https://github.com/dashpay/platform/issues/90))
* unable to decode contract
* update contract (dashpay/rs-platform[#72](https://github.com/dashpay/platform/issues/72))
* update to latest grovedb and added some tests. (dashpay/rs-platform[#123](https://github.com/dashpay/platform/issues/123))
* use binary zero instead of ascii
* use correct linker
* use slices instead of binary strings to represent the values we intend


### Continuous Integration

* increase release timeouts ([#1032](https://github.com/dashpay/platform/issues/1032))
* add PR linter ([#1025](https://github.com/dashpay/platform/issues/1025))
* add timeouts to self-hosted ci runs ([#1026](https://github.com/dashpay/platform/issues/1026))
* remove drive node.js binding release
* sign MacOs Dashmate release ([#890](https://github.com/dashpay/platform/issues/890))
* dashmate release script fix ([#846](https://github.com/dashpay/platform/issues/846), [#836](https://github.com/dashpay/platform/issues/836))
* increase cache-max-size ([#704](https://github.com/dashpay/platform/issues/704))
* add rust toolchain with wasm target to release workflow
* integrate rust and js build process
* add toolchain to setup rust actions
* build package before lining
* checks for JS and Rust packages
* code scanning ([#626](https://github.com/dashpay/platform/issues/626))
* do not build everything everytime
* fix concurrency for js checks
* fix env syntax in release workflow ([#664](https://github.com/dashpay/platform/issues/664))
* fix rs-checks
* fix workflow names
* increase JS linting timeout
* increase timeouts
* move concurrency to package workflows
* move out workflows from include
* set concurrency based on workflow
* shorten workflow
* release.yml contained wrong indentation ([#597](https://github.com/dashpay/platform/issues/597))
* publish envoy and dashmate-helper docker images after release ([#595](https://github.com/dashpay/platform/issues/595))
* update workflows to use stable toolchain (dashpay/rs-platform[#167](https://github.com/dashpay/platform/issues/167))
* `musl` binaries were not built correctly (dashpay/rs-platform[#58](https://github.com/dashpay/platform/issues/58))
* add deps security check
* add Node.JS workflow
* add release workflow
* add rust-toolchain.toml
* fix aarch64 musl build (dashpay/rs-platform[#36](https://github.com/dashpay/platform/issues/36))
* formatter check
* implement initial ci
* prevent PRs to master (dashpay/rs-platform[#28](https://github.com/dashpay/platform/issues/28))
* rename workflow
* run GitHub action for PR on dev branches (dashpay/rs-platform[#38](https://github.com/dashpay/platform/issues/38))


### Tests

* **dpp:** remove old dpp fixtures from tests ([#971](https://github.com/dashpay/platform/issues/971))
* **rs-drive:** fix path to EntropyGenerator ([#856](https://github.com/dashpay/platform/issues/856))
* **wasm-dpp:** decodeProtocolEntity test ([#834](https://github.com/dashpay/platform/issues/834))
* **drive:** synchronizeMasternodeIdentitiesFactory ([#586](https://github.com/dashpay/platform/issues/586))
* add test with fetching non exist document
* added test for in with only some elements (dashpay/rs-platform[#81](https://github.com/dashpay/platform/issues/81))
* dpns tests (dashpay/rs-platform[#89](https://github.com/dashpay/platform/issues/89))
* encoding (added negative and positive infinity)
* refactor and implementation of document's validator tests


### Build System

* **dashmate:** better versioning ([#988](https://github.com/dashpay/platform/issues/988))
* **dashmate:** prepare MacOs build for notarization ([#985](https://github.com/dashpay/platform/issues/985))
* **dashmate:** remove MacOs malicious software warning ([#977](https://github.com/dashpay/platform/issues/977))
* **dashmate:** fix removing unnecessary packages
* **dashamte:** build arm64 deb file instead of armel ([#943](https://github.com/dashpay/platform/issues/943))
* **dashamte:** remove deb release docker dependencies ([#934](https://github.com/dashpay/platform/issues/934))
* **dashmate:** build only linux tarballs ([#936](https://github.com/dashpay/platform/issues/936))
* **dashmate:** fix deb package release  ([#864](https://github.com/dashpay/platform/issues/864))
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))
* add missing deps ([#824](https://github.com/dashpay/platform/issues/824))
* add bash to Drive's Dockerfile
* continuation of build profile fixing
* fix cargo build profile option
* use debug profile for development
* release script could not find previous tag in some cases ([#558](https://github.com/dashpay/platform/issues/558))


### Code Refactoring

* **dashmate:** always keep platform config ([#868](https://github.com/dashpay/platform/issues/868))
* **dashmate:** move core devnet options to subsection ([#867](https://github.com/dashpay/platform/issues/867))
* change dpp to be based on platform value ([#809](https://github.com/dashpay/platform/issues/809))
* remove various Clippy warnings ([#793](https://github.com/dashpay/platform/issues/793))
* **rs-dpp:** bring error type to one format ([#804](https://github.com/dashpay/platform/issues/804))
* initial cleanup of Drive and DPP document types ([#750](https://github.com/dashpay/platform/issues/750))
* use a trait to remove the need for some functions ([#747](https://github.com/dashpay/platform/issues/747))
* **dashmate:** status command ([#660](https://github.com/dashpay/platform/issues/660))
* **drive:** remove redundant genesis time key ([#722](https://github.com/dashpay/platform/issues/722))
* use FeeResult to collect block fees ([#652](https://github.com/dashpay/platform/issues/652))
* accept DPP entities
* adjust how left_to_right is assigned
* documents refactoring, with batching and fee work (dashpay/rs-platform[#118](https://github.com/dashpay/platform/issues/118))
* error types to handle user errors (dashpay/rs-platform[#77](https://github.com/dashpay/platform/issues/77))
* fix numerous unused include warnings
* query (dashpay/rs-platform[#96](https://github.com/dashpay/platform/issues/96))
* query `$id` property (dashpay/rs-platform[#56](https://github.com/dashpay/platform/issues/56))
* remove a few unneeded `mut` keywords, fixes warnings
* remove unused variable
* replace unused mut variable with todo for calculating cost
* return array type in case of non byteArray
* split GroveDB and Drive
* split query_documents into two methods
* update grovedb (dashpay/rs-platform[#91](https://github.com/dashpay/platform/issues/91))
* use 32 byte identity fields


### Performance Improvements

* **drive:** do not call process proposal after prepare ([#656](https://github.com/dashpay/platform/issues/656))


### Miscellaneous Chores

* bump sentinel to 1.7.3 ([#1057](https://github.com/dashpay/platform/issues/1057))
* **dashmate:** rename compose project name ([#1055](https://github.com/dashpay/platform/issues/1055))
* **dashmate:** update testnet preset ([#1054](https://github.com/dashpay/platform/issues/1054))
* increase client side timeouts ([#1050](https://github.com/dashpay/platform/issues/1050))
* **dapi-client:** upate testnet masternode addresses whitelist ([#1023](https://github.com/dashpay/platform/issues/1023))
* disable protocol version signaling ([#1029](https://github.com/dashpay/platform/issues/1029))
* **dashamte:** remove deprecation warning on start ([#925](https://github.com/dashpay/platform/issues/925))
* **dashmate:** update production dashcore versions for mainnet and testnet ([#840](https://github.com/dashpay/platform/issues/840))
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))
* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))
* **dashmate:** upgrade dashcore version for local networks ([#843](https://github.com/dashpay/platform/issues/843))
* **drive:** temporary disable payout script ([#835](https://github.com/dashpay/platform/issues/835))
* **drive:** temporary disable unstable withdrawal logic ([#831](https://github.com/dashpay/platform/issues/831))
* **sdk:** add eslint ([#827](https://github.com/dashpay/platform/issues/827))
* **dashmate:** sync Tenderdash config with Tenderdash v0.10.0-dev.8 ([#746](https://github.com/dashpay/platform/issues/746))
* update Tenderdash to 0.10.0-dev.8 ([#741](https://github.com/dashpay/platform/issues/741))
* **dpp:** wasm dpp data contract test error types ([#684](https://github.com/dashpay/platform/issues/684))
* **drive:** log synchronize identities properly ([#686](https://github.com/dashpay/platform/issues/686))
* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))
* **drive:** remove txs from logs ([#683](https://github.com/dashpay/platform/issues/683))
* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))
* **drive:** remove txs from logs ([#683](https://github.com/dashpay/platform/issues/683))
* add yarn cache
* **dpp:** switch dpp to working revision
* **drive:** comprehensive logging for same block execution ([#657](https://github.com/dashpay/platform/issues/657))
* ensure consistent Rust dependencies ([#658](https://github.com/dashpay/platform/issues/658))
* fix NPM security vulnerabilities
* ignore target dir for docker
* **release:** bump package version
* **release:** update changelog and bump version to 0.24.0-dev.9 ([#667](https://github.com/dashpay/platform/issues/667))
* remove js-abci from workspaces
* remove lodash per-method deps ([#661](https://github.com/dashpay/platform/issues/661))
* remove package-lock.json
* update to yarn 3.3.0
* update bls-signatures to fix dependencies conflict (dashpay/rs-platform[#184](https://github.com/dashpay/platform/issues/184))
* **dpp:** re-enable limited array support for data contracts (dashpay/rs-platform[#165](https://github.com/dashpay/platform/issues/165))
* **dpp:** [v23 port] allow only asc order for indices (dashpay/rs-platform[#135](https://github.com/dashpay/platform/issues/135))
* update error prefix (dashpay/rs-platform[#67](https://github.com/dashpay/platform/issues/67))
* update to new GroveDB (dashpay/rs-platform[#108](https://github.com/dashpay/platform/issues/108)), closes [dashpay/rs-platform#107](https://github.com/dashpay/rs-platform/issues/107)


### Styles

* fix formatting
* extract field_type
* format json
* remove commented code
* remove todo
* run cargo fmt
* sam's *** naming
* use explicit xor (dashpay/rs-platform[#57](https://github.com/dashpay/platform/issues/57))


## [0.24.0-dev.34](https://github.com/dashpay/platform/compare/v0.24.0-dev.33...v0.24.0-dev.34) (2023-05-08)


### ⚠ BREAKING CHANGES

* Validator rotation logic is changed. Previous blockchain data won't be compatible (#1034)

### Features

* **drive:** whitelist and filter banned nodes for validators ([#1034](https://github.com/dashpay/platform/issues/1034))


### Bug Fixes

* **dapi:** invalid addresses in the whitelist ([#1044](https://github.com/dashpay/platform/issues/1044))
* **dashmate:** reset platform commands hangs ([#1038](https://github.com/dashpay/platform/issues/1038))
* **dashmate:** set permissions for dashcore log file ([#1037](https://github.com/dashpay/platform/issues/1037))
* **drive:** cannot destructure property ‘quorumHash’ of ‘instantLock... ([#1046](https://github.com/dashpay/platform/issues/1046))
* **drive:** cannot read properties of undefined (reading 'toString') ([#1045](https://github.com/dashpay/platform/issues/1045))

## [0.24.0-dev.33](https://github.com/dashpay/platform/compare/v0.24.0-dev.32...v0.24.0-dev.33) (2023-05-05)


### Continuous Integration

* increase release timeouts ([#1032](https://github.com/dashpay/platform/issues/1032))

## [0.24.0-dev.32](https://github.com/dashpay/platform/compare/v0.24.0-dev.31...v0.24.0-dev.32) (2023-05-04)

### ⚠ BREAKING CHANGES

* Previous state won't be valid (#1029)


### Features

* **dashmate:** core log file and debug categories ([#913](https://github.com/dashpay/platform/issues/913))


### Bug Fixes

* **dashmate:** waitForQuorumConnections deadline of 300000 exceeded ([#1015](https://github.com/dashpay/platform/issues/1015))
* **dashmate:** wrong volume removal retry logic ([#1016](https://github.com/dashpay/platform/issues/1016))


### Continuous Integration

* add PR linter ([#1025](https://github.com/dashpay/platform/issues/1025))
* add timeouts to self-hosted ci runs ([#1026](https://github.com/dashpay/platform/issues/1026))


### Miscellaneous Chores

* **dapi-client:** upate testnet masternode addresses whitelist ([#1023](https://github.com/dashpay/platform/issues/1023))
* **drive:** disable protocol version signaling ([#1029](https://github.com/dashpay/platform/issues/1029))



## [0.25.0-dev.2](https://github.com/dashpay/platform/compare/v0.25.0-dev.1...v0.25.0-dev.2) (2023-05-01)

### Miscellaneous Chores

* backports from v0.24


## [0.25.0-dev.1](https://github.com/dashpay/platform/compare/v0.24.0-dev.16...v0.25.0-dev.1) (2023-05-01)

### Features

* abci propose validators ([#954](https://github.com/dashpay/platform/issues/954))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))


### Bug Fixes


* fixes for deployment and clean up ([#1004](https://github.com/dashpay/platform/issues/1004))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* inconsistencies after merge
* init chain core chain lock failure ([#976](https://github.com/dashpay/platform/issues/976))
* **rs-dpp:** json query wrapper incorrectly serializes identifier
* **rs-drive-abci:** Get rs-drive-abci to commit block 1 ([#981](https://github.com/dashpay/platform/issues/981))
* **rs-drive-abci:** start rs-drive-abci using dashmate and pass init_chain ([#941](https://github.com/dashpay/platform/issues/941))
* same block core chain lock height
* **sdk:** provide StateTransitionExecutionContext to validateBasic
* small wasm fix
* **wasm-dpp:** failing build and tests ([#947](https://github.com/dashpay/platform/issues/947))


### Code Refactoring

* **rs-dpp:** use common wrapper for new signature errors
* **rs-drive-abci:** new error wrappers


### Tests

* chainlock quorum rotation test ([#952](https://github.com/dashpay/platform/issues/952))
* **dapi-client:** fix broken SimplifiedMasternodeListDAPIAddressProvider test ([#916](https://github.com/dashpay/platform/issues/916))
* **rs-dpp:** fix tests
* **rs-drive-abci:** fix optional pose_revived_height
* **rs-drive-abci:** set pose_revived_height to None


### Continuous Integration

* remove drive node.js binding release


### Build System

* add missing rust packages to Dockerfiles
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))
* remove js-drive and rs-drive-nodejs from PR CI
* return proper Rust setup


### Miscellaneous Chores

* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* bump base.js (local networks only) dashd version to 20.0.0-alpha.assetlocks.2
* remove unused deps ([#987](https://github.com/dashpay/platform/issues/987))
* remove unused js-drive and rs-drive code
* resolve todos
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))
* use master dash core rpc
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))

## [0.24.0-dev.31](https://github.com/dashpay/platform/compare/v0.24.0-dev.30...v0.24.0-dev.31) (2023-05-01)


### Bug Fixes

* expect platformNodeID to be a hex string ([#1013](https://github.com/dashpay/platform/issues/1013))

## [0.24.0-dev.30](https://github.com/dashpay/platform/compare/v0.24.0-dev.29...v0.24.0-dev.30) (2023-04-30)


### Bug Fixes

* **dashmate:** "volume is in use" and "no such volume" ([#1005](https://github.com/dashpay/platform/issues/1005))


## [0.24.0-dev.29](https://github.com/dashpay/platform/compare/v0.24.0-dev.28...v0.24.0-dev.29) (2023-04-29)


### Bug Fixes

* **dashmate:** reset platform affects core ([#1001](https://github.com/dashpay/platform/issues/1001))
* **drive:** double init chain leads to side bugs ([#1002](https://github.com/dashpay/platform/issues/1002))

## [0.24.0-dev.28](https://github.com/dashpay/platform/compare/v0.24.0-dev.27...v0.24.0-dev.28) (2023-04-28)


### Bug Fixes

* **dashmate:** BLS private key validate accepts whitespaces ([#998](https://github.com/dashpay/platform/issues/998))
* **dashmate:** can't remove volumes if they not exist ([#997](https://github.com/dashpay/platform/issues/997))
* **dashmate:** show masternode state while it is not synced ([#999](https://github.com/dashpay/platform/issues/999))
* **dashmate:** wrap register masternode command in small terminals ([#996](https://github.com/dashpay/platform/issues/996))

## [0.24.0-dev.27](https://github.com/dashpay/platform/compare/v0.24.0-dev.26...v0.24.0-dev.27) (2023-04-28)


### ⚠ BREAKING CHANGES

* The --platfrom-only flag is renamed to --platform (#991)

### Bug Fixes

* **dashmate:** multiple issues in the reset command ([#991](https://github.com/dashpay/platform/issues/991))

## [0.24.0-dev.26](https://github.com/dashpay/platform/compare/v0.24.0-dev.25...v0.24.0-dev.26) (2023-04-27)


### ⚠ BREAKING CHANGES

* Previous state won't be valid due to changes in the sync identities logic

### Bug Fixes

* **drive:** non-unique masternode voting keys ([#986](https://github.com/dashpay/platform/issues/986))


### Build System

* **dashmate:** better versioning ([#988](https://github.com/dashpay/platform/issues/988))
* **dashmate:** prepare MacOs build for notarization ([#985](https://github.com/dashpay/platform/issues/985))

## [0.24.0-dev.25](https://github.com/dashpay/platform/compare/v0.24.0-dev.24...v0.24.0-dev.25) (2023-04-26)


### Features

* **dashmate:** better ZeroSSL error messages ([#950](https://github.com/dashpay/platform/issues/950))
* **dashmate:** set random core rpc username and password on setup ([#973](https://github.com/dashpay/platform/issues/973))
* **dashmate:** verbose `connect ENOENT /var/run/docker.sock` error ([#951](https://github.com/dashpay/platform/issues/951))


### Bug Fixes

* **dashmate:** ability to work with non-default docker socket path ([#967](https://github.com/dashpay/platform/issues/967))
* **dashmate:** empty masternode status while syncing ([#970](https://github.com/dashpay/platform/issues/970))
* **dashmate:** form accepts invalid BLS key ([#961](https://github.com/dashpay/platform/issues/961))
* **dashmate:** invalid mn register command in output of dashmate setup ([#959](https://github.com/dashpay/platform/issues/959))
* **dashmate:** reward shares can be negative during HP masternode registration ([#960](https://github.com/dashpay/platform/issues/960))
* **dashmate:** select number of masternodes during local setup freezes ([#957](https://github.com/dashpay/platform/issues/957))
* **dashmate:** yaml package security vulnerability ([#975](https://github.com/dashpay/platform/issues/975))
* **dashmate:** already configured preset is ignored ([#974](https://github.com/dashpay/platform/issues/974))
* **test-suite:** expects identities for invalid mns ([#968](https://github.com/dashpay/platform/issues/968))


### Build System

* **dashmate:** remove MacOs malicious software warning ([#977](https://github.com/dashpay/platform/issues/977))

## [0.24.0-dev.24](https://github.com/dashpay/platform/compare/v0.24.0-dev.23...v0.24.0-dev.24) (2023-04-24)


### ⚠ BREAKING CHANGES

* Previous state might be invalid since to new sync mn identities logic (#965)

### Features

* **wasm-dpp:** state_transition_fee_validator binding and tests ([#874](https://github.com/dashpay/platform/issues/874))


### Bug Fixes

* **dashmate:** wrong envoy config path on windows ([#949](https://github.com/dashpay/platform/issues/949))
* **drive:** updating a masternode identity with invalid entry from SML ([#965](https://github.com/dashpay/platform/issues/965))

## [0.24.0-dev.23](https://github.com/dashpay/platform/compare/v0.24.0-dev.22...v0.24.0-dev.23) (2023-04-20)


### Build System

* **dashmate:** fix removing unnecessary packages

## [0.24.0-dev.22](https://github.com/dashpay/platform/compare/v0.24.0-dev.21...v0.24.0-dev.22) (2023-04-20)


### Continuous Integration

* remove drive node.js binding release


### Build System

* **dashmate:** build arm64 deb file instead of armel ([#943](https://github.com/dashpay/platform/issues/943))

## [0.24.0-dev.21](https://github.com/dashpay/platform/compare/v0.24.0-dev.20...v0.24.0-dev.21) (2023-04-19)


### Features

* **dashmate:** check system requirements before setup ([#935](https://github.com/dashpay/platform/issues/935))


### Bug Fixes

* **dashmate:** сan't find begoo/index with `yarn dashmate setup` ([#933](https://github.com/dashpay/platform/issues/933))


### Miscellaneous Chores

* **dashmate:** remove deprecation warning on start ([#925](https://github.com/dashpay/platform/issues/925))


### Continuous Integration

* fix release workflow


### Build System

* **dashmate:** remove deb release docker dependencies ([#934](https://github.com/dashpay/platform/issues/934))
* **dashmate:** build only linux tarballs ([#936](https://github.com/dashpay/platform/issues/936))


## [0.24.0-dev.20](https://github.com/dashpay/platform/compare/v0.24.0-dev.19...v0.24.0-dev.20) (2023-04-18)


### ⚠ BREAKING CHANGES

* Some wasm-dpp APIs are different to js-dpp ones. The only visible divergencies were addressed, but many others might've been left unnoticed (#848)

### Features

* **drive:** handle quorum rotation failure ([#858](https://github.com/dashpay/platform/issues/858))
* wasm-dpp integration ([#848](https://github.com/dashpay/platform/issues/848))

## [0.24.0-dev.19](https://github.com/dashpay/platform/compare/v0.24.0-dev.18...v0.24.0-dev.19) (2023-04-17)

### Continuous Integration

* test release workflow

## [0.24.0-dev.18](https://github.com/dashpay/platform/compare/v0.24.0-dev.17...v0.24.0-dev.18) (2023-04-14)


### Features

* **dashmate:** build linux tarballs ([#887](https://github.com/dashpay/platform/issues/887))
* **dashmate:** build services before restart ([#894](https://github.com/dashpay/platform/issues/894))
* **dashmate:** exit status with 2 if it's not running ([#896](https://github.com/dashpay/platform/issues/896))
* **dashmate:** implement http json rpc api ([#888](https://github.com/dashpay/platform/issues/888))
* **dashmate:** tenderdash latest block time in status ([#906](https://github.com/dashpay/platform/issues/906))
* **dpp:** serialize consensus errors ([#871](https://github.com/dashpay/platform/issues/871))
* drive verification c bindings ([#860](https://github.com/dashpay/platform/issues/860))


### Bug Fixes

* DAPI still expected on normal masternodes ([#904](https://github.com/dashpay/platform/issues/904))
* **dapi-client:** platform port is ignored from SML ([#903](https://github.com/dashpay/platform/issues/903))
* **dashmate:** api binds to all interfaces ([#893](https://github.com/dashpay/platform/issues/893))
* **dashmate:** dashmate helper is running under root user ([#895](https://github.com/dashpay/platform/issues/895))
* **dashmate:** dashmate logic doesn't recognize it's ran from helper ([#902](https://github.com/dashpay/platform/issues/902))
* **dashmate:** missing rawblock zmq message in core config ([#770](https://github.com/dashpay/platform/issues/770))
* **dashmate:** undefined wallet for dash-cli ([#786](https://github.com/dashpay/platform/issues/786))
* **dpp:** various fixes in DPP and system contracts ([#907](https://github.com/dashpay/platform/issues/907))
* **drive:** non-deterministic run of mn identities sync ([#910](https://github.com/dashpay/platform/issues/910))
* **drive:** total HPMNs contains all masternodes ([#911](https://github.com/dashpay/platform/issues/911))
* identifier deserialization doesn't work for bincode ([#885](https://github.com/dashpay/platform/issues/885))
* llmqType must be equal to one of the allowed values ([#884](https://github.com/dashpay/platform/issues/884))
* possible overflow issues ([#877](https://github.com/dashpay/platform/issues/877))


### Miscellaneous Chores

* **dashmate:** update production dashcore versions for mainnet and testnet ([#840](https://github.com/dashpay/platform/issues/840))
* **sdk:** add eslint ([#829](https://github.com/dashpay/platform/issues/829))


### Continuous Integration

* sign MacOs Dashmate release ([#890](https://github.com/dashpay/platform/issues/890))

## [0.24.0-dev.17](https://github.com/dashpay/platform/compare/v0.24.0-dev.16...v0.24.0-dev.17) (2023-04-04)


### Features

* **dashmate:** add new quroum in dashcore config ([#862](https://github.com/dashpay/platform/issues/862))
* **dashmate:** enable platform option ([#853](https://github.com/dashpay/platform/issues/853))
* **dashmate:** generate self-signed certificates in the `setup` command ([#869](https://github.com/dashpay/platform/issues/869))
* **dashmate:** high-performance nodes registration with `setup` command ([#794](https://github.com/dashpay/platform/issues/794))
* **dashmate:** hint to setup a node on start failure ([#866](https://github.com/dashpay/platform/issues/866))
* **dpp:** add fees API  to rust wasm bindings ([#830](https://github.com/dashpay/platform/issues/830))
* **dpp:** optional execution context in rs-dpp ([#811](https://github.com/dashpay/platform/issues/811))
* **dpp:** state transition applicator ([#878](https://github.com/dashpay/platform/issues/878))
* **rs-dpp:** migrate fees from js-dpp v0.24 ([#851](https://github.com/dashpay/platform/issues/851))
* state transition conversion ([#844](https://github.com/dashpay/platform/issues/844))
* **wasm-dpp:** add tests for state transition basic validator ([#857](https://github.com/dashpay/platform/issues/857))
* **wasm-dpp:** DashPlatformProtocol tests ([#841](https://github.com/dashpay/platform/issues/841))
* **wasm-dpp:** identity transitions additional functionality ([#855](https://github.com/dashpay/platform/issues/855))
* **wasm-dpp:** implement validateStateTransitionStateFactory tests ([#861](https://github.com/dashpay/platform/issues/861))
* **wasm-dpp:** provide external entropy generator to document factory ([#845](https://github.com/dashpay/platform/issues/845))
* **wasm-dpp:** validate_state_transition_identity_signature binding and test ([#865](https://github.com/dashpay/platform/issues/865))


### Bug Fixes

* **dashmate:** ambiguous validation errors for file certificates ([#870](https://github.com/dashpay/platform/issues/870))
* **dashmate:** config.isPlatformEnabled is not a function ([#872](https://github.com/dashpay/platform/issues/872))
* **dpp:** incorrect public key validator schema in ST Facade ([#854](https://github.com/dashpay/platform/issues/854))
* **scripts:** update configure_test_network for hpmn ([#863](https://github.com/dashpay/platform/issues/863))
* **wasm-dpp:** fix decoding protocol version varint error to match previous implementation ([#849](https://github.com/dashpay/platform/issues/849))


### Tests

* **rs-drive:** fix path to EntropyGenerator ([#856](https://github.com/dashpay/platform/issues/856))


### Code Refactoring

* **dashmate:** always keep platform config ([#868](https://github.com/dashpay/platform/issues/868))
* **dashmate:** move core devnet options to subsection ([#867](https://github.com/dashpay/platform/issues/867))


### Build System

* **dashmate:** fix deb package release  ([#864](https://github.com/dashpay/platform/issues/864))
* operations in configure.sh script were in a wrong order ([#876](https://github.com/dashpay/platform/issues/876))


### Miscellaneous Chores

* `rs-dpp` and `wasm-dpp` updates for integration ([#875](https://github.com/dashpay/platform/issues/875))
* **wasm-dpp:** proper identifier buffer inheritance ([#879](https://github.com/dashpay/platform/issues/879))

## [0.24.0-dev.16](https://github.com/dashpay/platform/compare/v0.24.0-dev.15...v0.24.0-dev.16) (2023-03-22)


### Features

* **wasm-dpp:** decodeProtocolEntity test ([#834](https://github.com/dashpay/platform/issues/834))


### Code Refactoring

* change dpp to be based on platform value ([#809](https://github.com/dashpay/platform/issues/809))


### Miscellaneous Chores

* **dashmate:** upgrade dashcore version for local networks ([#843](https://github.com/dashpay/platform/issues/843))


### Continuous Integration

* dashmate release script fix ([#846](https://github.com/dashpay/platform/issues/846))

## [0.24.0-dev.15](https://github.com/dashpay/platform/compare/v0.24.0-dev.14...v0.24.0-dev.15) (2023-03-21)


### Miscellaneous Chores

* **drive:** temporary disable payout script ([#835](https://github.com/dashpay/platform/issues/835))


### Continuous Integration

* dashmate release script fix ([#836](https://github.com/dashpay/platform/issues/836))

## [0.24.0-dev.14](https://github.com/dashpay/platform/compare/v0.24.0-dev.13...v0.24.0-dev.14) (2023-03-20)


### ⚠ BREAKING CHANGES

* core version 19.0-beta integration (#771)

### Features

* better Core 19 support ([#832](https://github.com/dashpay/platform/issues/832))
* core version 19.0-beta integration ([#771](https://github.com/dashpay/platform/issues/771))
* **dashmate:** register HPMN for local network ([#796](https://github.com/dashpay/platform/issues/796))
* **dasmate:** pack release script ([#781](https://github.com/dashpay/platform/issues/781))
* **dpp:** identity facade ([#782](https://github.com/dashpay/platform/issues/782))
* **dpp:** integration tests for wasm-dpp document transitions ([#777](https://github.com/dashpay/platform/issues/777))
* **dpp:** wasm bindings for Documents related validations ([#709](https://github.com/dashpay/platform/issues/709))
* **dpp:** wasm-dpp: integration tests for document ([#762](https://github.com/dashpay/platform/issues/762))
* Identity v2 ([#705](https://github.com/dashpay/platform/issues/705))
* platform value abstraction ([#805](https://github.com/dashpay/platform/issues/805))
* proposer signaling of protocol version upgrade and fork activation ([#778](https://github.com/dashpay/platform/issues/778))
* register system data contracts in RS Drive ([#776](https://github.com/dashpay/platform/issues/776))
* **rs-dpp:**  dashpay datatrigger toUserIds better validation ([#799](https://github.com/dashpay/platform/issues/799))
* **rs-dpp:** backport of index_definitions.unique validation ([#802](https://github.com/dashpay/platform/issues/802))
* **rs-dpp:** backports of identity/stateTransition from js-dpp ([#800](https://github.com/dashpay/platform/issues/800))
* **rs-dpp:** introduce `StateTransitionFactory` ([#810](https://github.com/dashpay/platform/issues/810))
* **rs-dpp:** validate indices are backwards compatible backport ([#797](https://github.com/dashpay/platform/issues/797))
* **rs-drive:** verification feature ([#803](https://github.com/dashpay/platform/issues/803))
* **wasm dpp:** validate state transition key signature ([#806](https://github.com/dashpay/platform/issues/806))
* **wasm-dpp:**  wasm bindings for Document Transitions  ([#707](https://github.com/dashpay/platform/issues/707))
* **wasm-dpp:** async state repository ([#766](https://github.com/dashpay/platform/issues/766))
* **wasm-dpp:** data contract facade ([#716](https://github.com/dashpay/platform/issues/716))
* **wasm-dpp:** Fix metadata, metadata tests and backport v23 matedata changes into wasm-dpp ([#819](https://github.com/dashpay/platform/issues/819))
* **wasm-dpp:** implement identity update transition ([#748](https://github.com/dashpay/platform/issues/748))
* **wasm-dpp:** integration tests validate data contract update transition ([#812](https://github.com/dashpay/platform/issues/812))
* **wasm-dpp:** protocol version validator tests ([#823](https://github.com/dashpay/platform/issues/823))
* **wasm-dpp:** remove unused documents factory tests ([#828](https://github.com/dashpay/platform/issues/828))
* **wasm-dpp:** state transition facade ([#814](https://github.com/dashpay/platform/issues/814))
* withdrawals status sync ([#679](https://github.com/dashpay/platform/issues/679))


### Bug Fixes

* **ci:** fix release workflow syntax error ([#808](https://github.com/dashpay/platform/issues/808))
* **dashmate:** make dashmate helper run commands as host user ([#765](https://github.com/dashpay/platform/issues/765))
* **dashmate:** visual fixes for dashmate status ([#787](https://github.com/dashpay/platform/issues/787))
* **dpp:** update jsonschema-rs and enable tests ([#780](https://github.com/dashpay/platform/issues/780))
* **rs-dpp:** fetch latest core chain locked height misuse ([#789](https://github.com/dashpay/platform/issues/789))
* update webpack to resolve npm audit error ([#822](https://github.com/dashpay/platform/issues/822))
* **wasm-dpp:** Identifier and its tests ([#821](https://github.com/dashpay/platform/issues/821))


### Code Refactoring

* remove various Clippy warnings ([#793](https://github.com/dashpay/platform/issues/793))
* **rs-dpp:** bring error type to one format ([#804](https://github.com/dashpay/platform/issues/804))


### Build System

* add missing deps ([#824](https://github.com/dashpay/platform/issues/824))


### Documentation

* supported Node.JS version and dashmate command description ([#825](https://github.com/dashpay/platform/issues/825))


### Miscellaneous Chores

* **drive:** temporary disable unstable withdrawal logic ([#831](https://github.com/dashpay/platform/issues/831))
* **sdk:** add eslint ([#827](https://github.com/dashpay/platform/issues/827))

## [0.24.0-dev.13](https://github.com/dashpay/platform/compare/v0.24.0-dev.12...v0.24.0-dev.13) (2023-01-30)


### ⚠ BREAKING CHANGES

* **dapi:** use single envoy port for all connections (#752)

### Features

* allow to get drive status from dashmate helper ([#749](https://github.com/dashpay/platform/issues/749))
* allow to get drive's status from dashmate helper ([#755](https://github.com/dashpay/platform/issues/755))
* **dapi:** use single envoy port for all connections ([#752](https://github.com/dashpay/platform/issues/752))
* **dashmate:** update Core to 18.2.0 ([#735](https://github.com/dashpay/platform/issues/735))
* **drive:** ABCI context logger ([#693](https://github.com/dashpay/platform/issues/693))
* **drive:** log contractId in deliverTx handler ([#730](https://github.com/dashpay/platform/issues/730))
* **drive:** log number of refunded epochs ([#729](https://github.com/dashpay/platform/issues/729))
* integrate wasm Document into JS tests ([#644](https://github.com/dashpay/platform/issues/644))
* varint protocol version ([#758](https://github.com/dashpay/platform/issues/758))
* **wasm-dpp:** implement function to produce generics from JsValue ([#712](https://github.com/dashpay/platform/issues/712))
* **wasm-dpp:** implement identity create transition ([#697](https://github.com/dashpay/platform/issues/697))
* **wasm-dpp:** Wasm dpp integration tests validate data contract factory ([#751](https://github.com/dashpay/platform/issues/751))


### Bug Fixes

* **dashmate:** Dash Core container is unable to restart properly under WSL ([#736](https://github.com/dashpay/platform/issues/736))
* **dashmate:** fix migration for configs without platform ([#738](https://github.com/dashpay/platform/issues/738))
* **dashmate:** migrations fixes ([#759](https://github.com/dashpay/platform/issues/759))
* **dpp:** existing property in a new index ([#694](https://github.com/dashpay/platform/issues/694))
* ua-parser-js vulnerability  ([#756](https://github.com/dashpay/platform/issues/756))


### Miscellaneous Chores

* **dashmate:** sync Tenderdash config with Tenderdash v0.10.0-dev.8 ([#746](https://github.com/dashpay/platform/issues/746))
* update Tenderdash to 0.10.0-dev.8 ([#741](https://github.com/dashpay/platform/issues/741))


### Code Refactoring

* initial cleanup of Drive and DPP document types ([#750](https://github.com/dashpay/platform/issues/750))
* use a trait to remove the need for some functions ([#747](https://github.com/dashpay/platform/issues/747))

### [0.23.2](https://github.com/dashpay/platform/compare/v0.23.0...v0.23.2) (2023-01-19)


### Bug Fixes

* **dapi-client:** missing fetch-polyfill in npm installation ([#743](https://github.com/dashpay/platform/issues/743))
* **dapi-grpc:** unsafe-eval errors in protobuf js files ([#713](https://github.com/dashpay/platform/issues/713))


### [0.23.1](https://github.com/dashpay/platform/compare/v0.23.0...v0.23.1) (2023-01-13)


### Features

* update Platform to Core v18.2.0-rc.4 ([#701](https://github.com/dashpay/platform/issues/701))


### Continuous Integration

* fix incorrect github actions templating syntax ([#689](https://github.com/dashpay/platform/issues/689))
* release dashmate packages ([#669](https://github.com/dashpay/platform/issues/669))


### Miscellaneous Chores

* **dapi-client:** replace axios with fetch ([#690](https://github.com/dashpay/platform/issues/690))
* **dashmate:** update base config to core 18.2.0 ([#706](https://github.com/dashpay/platform/issues/706))
* replace grpc-web with @improbable-eng/grpc-web ([#628](https://github.com/dashpay/platform/issues/628))
* set core in base config to v18.1.1 ([#720](https://github.com/dashpay/platform/issues/720))
* update platform chain id ([#703](https://github.com/dashpay/platform/issues/703))
* use core 18.2.0 on testnet ([#725](https://github.com/dashpay/platform/issues/725))



## [0.24.0-dev.12](https://github.com/dashpay/platform/compare/v0.24.0-dev.11...v0.24.0-dev.12) (2023-01-11)


### ⚠ BREAKING CHANGES

* AbstractStateTransition#calculateFees removed
* State transition fees are calculated differently so previous block data is not valid anymore
* Added new tree to initial structure so previous state is not valid anymore

### Features

* credit refunds ([#662](https://github.com/dashpay/platform/issues/662))
* **dashmate:** additional dashd options ([#692](https://github.com/dashpay/platform/issues/692))
* **dashmate:** pass ZeroSSL as command line parameter ([#651](https://github.com/dashpay/platform/issues/651))
* **dashmate:** remove axios from zerossl requests
* **dashmate:** remove axios from zerossl requests
* **dpp:** AbstractConsensusError tests and extensions ([#670](https://github.com/dashpay/platform/issues/670))
* **dpp:** Data Contract Update Transition wasm binding ([#696](https://github.com/dashpay/platform/issues/696))
* **drive:** do not switch to validator quorum which will be removed soon ([#616](https://github.com/dashpay/platform/issues/616))
* multiple documents changes per batch and support for GroveDB 0.9 ([#699](https://github.com/dashpay/platform/issues/699))
* update Platform to Core v18.2.0-rc.4 ([#701](https://github.com/dashpay/platform/issues/701))


### Bug Fixes

* **dashmate:** ZeroSSL certificate cannot be downloaded ([#718](https://github.com/dashpay/platform/issues/718))
* **drive:** merk caching in contract caching (irony) ([#710](https://github.com/dashpay/platform/issues/710))
* find_duplicates_by_id.rs not compiling ([#702](https://github.com/dashpay/platform/issues/702))
* Starcounter-Jack JSON-Patch Prototype Pollution vulnerability ([#708](https://github.com/dashpay/platform/issues/708))


### Code Refactoring

* **dashmate:** status command ([#660](https://github.com/dashpay/platform/issues/660))


### Continuous Integration

* fix incorrect github actions templating syntax ([#689](https://github.com/dashpay/platform/issues/689))
* increase cache-max-size ([#704](https://github.com/dashpay/platform/issues/704))


### Miscellaneous Chores

* **dapi-client:** replace axios with fetch ([#690](https://github.com/dashpay/platform/issues/690))
* **dashmate:** update base config to core 18.2.0 ([#706](https://github.com/dashpay/platform/issues/706))
* **dpp:** wasm dpp data contract test error types ([#684](https://github.com/dashpay/platform/issues/684))
* **drive:** log synchronize identities properly ([#686](https://github.com/dashpay/platform/issues/686))
* set core in base config to v18.1.1 ([#720](https://github.com/dashpay/platform/issues/720))

## [0.24.0-dev.11](https://github.com/dashpay/platform/compare/v0.24.0-dev.10...v0.24.0-dev.11) (2022-12-20)


### Bug Fixes

* **dashmate:** setDay is not a function ([#677](https://github.com/dashpay/platform/issues/677))
* **dashmate:** ZeroSSL certificates are not saved in WSL ([#676](https://github.com/dashpay/platform/issues/676))
* **drive:** initChain handler is not idempotent ([#675](https://github.com/dashpay/platform/issues/675))


### Continuous Integration

* bump action versions ([#678](https://github.com/dashpay/platform/issues/678))
* release dashmate packages ([#669](https://github.com/dashpay/platform/issues/669))


### Miscellaneous Chores

* **dashmate:** update tenderdash up to 0.10.0-dev.6 ([#674](https://github.com/dashpay/platform/issues/674))

## [0.24.0-dev.10](https://github.com/dashpay/platform/compare/v0.24.0-dev.9...v0.24.0-dev.10) (2022-12-15)


### Features

* Consensus Errors and ValidationResult bindings ([#643](https://github.com/dashpay/platform/issues/643))


### Bug Fixes

* **dashmate**: outdated Drive and DAPI images ([#668](https://github.com/dashpay/platform/issues/668))


### Continuous Integration

* fix entrypoint path in release workflow ([#671](https://github.com/dashpay/platform/issues/671))



## [0.24.0-dev.9](https://github.com/dashpay/platform/compare/v0.23.0...v0.24.0-dev.9) (2022-12-14)


### ⚠ BREAKING CHANGES

* Previous data is not compatible with new Drive

### Features

* average estimated processing fees ([#642](https://github.com/dashpay/platform/issues/642))
* **dpp:** bls adapter for WASM DPP ([#633](https://github.com/dashpay/platform/issues/633))
* **drive:** add time and protocolVersion fields to query metadata response ([#611](https://github.com/dashpay/platform/issues/611))
* **drive:** provide latest core chain lock on init chain ([#659](https://github.com/dashpay/platform/issues/659))
* **drive:** support for V0.7 of groveDB ([#665](https://github.com/dashpay/platform/issues/665))
* **drive:** use proposal block execution context in state repository ([#653](https://github.com/dashpay/platform/issues/653))
* **drive:** use single block execution context ([#627](https://github.com/dashpay/platform/issues/627))


### Bug Fixes

* build not working because of deprecated wasm-bindgen feature ([#639](https://github.com/dashpay/platform/issues/639))
* **dapi:** fail to reconnect to tenderdash in case of ENOTFOUND ([#621](https://github.com/dashpay/platform/issues/621))
* **dashmate:** broken helper docker image ([#630](https://github.com/dashpay/platform/issues/630))
* **dashmate:** ZeroSSL certificate renewal ([#624](https://github.com/dashpay/platform/issues/624))
* **drive:** multi transactions doesn't work properly ([#636](https://github.com/dashpay/platform/issues/636))


### Performance Improvements

* **drive:** do not call process proposal after prepare ([#656](https://github.com/dashpay/platform/issues/656))


### Code Refactoring

* use FeeResult to collect block fees ([#652](https://github.com/dashpay/platform/issues/652))


### Build System

* support Rust and JS packages

### Continuous Integration

* code scanning ([#626](https://github.com/dashpay/platform/issues/626))
* fix env syntax in release workflow ([#664](https://github.com/dashpay/platform/issues/664))
* support Rust and JS packages


### Miscellaneous Chores

* **dpp:** switch dpp to working revision
* **drive:** comprehensive logging for same block execution ([#657](https://github.com/dashpay/platform/issues/657))
* ensure consistent Rust dependencies ([#658](https://github.com/dashpay/platform/issues/658))
* remove lodash per-method deps ([#661](https://github.com/dashpay/platform/issues/661))
* replace grpc-web with @improbable-eng/grpc-web ([#628](https://github.com/dashpay/platform/issues/628))
* merged rs-platform monorepo


## [0.23.0](https://github.com/dashpay/platform/compare/v0.22.16...v0.23.0) (2022-12-05)

### ⚠ BREAKING CHANGES

* Dash Core version lower than 18.1.0 is not supported
* Blockchain data and state structure are changed and incompatible with previous versions
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* Data Contract indices must have 'asc' order (#435)
* Document query logic can behave differently in some cases (#398)
* Identity master key can be used only to update identity
* Use plain proRegTx for masternode identifier
* Incompatible wallet-lib storage format


### Features

* implement headers first synchronization ([#428](https://github.com/dashpay/platform/issues/428))
* precise storage fees ([#619](https://github.com/dashpay/platform/issues/619))
* **dashmate:** add `core reindex` command ([#533](https://github.com/dashevo/platform/issues/533)), closes [#538](https://github.com/dashevo/platform/issues/538)
* update Core to 18.1.0-rc.1 ([#544](https://github.com/dashevo/platform/issues/544), [#526](https://github.com/dashevo/platform/issues/526), [#511](https://github.com/dashevo/platform/issues/511))
* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458), [#474](https://github.com/dashevo/platform/issues/474), [#484](https://github.com/dashevo/platform/issues/484))
* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* Integrate with Tenderdash ABCI++ ([#314](https://github.com/dashevo/platform/issues/314))
* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292), [#477](https://github.com/dashevo/platform/issues/477), [#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** rework storage for multiple key-chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376), [#387](https://github.com/dashevo/platform/issues/387), [#369](https://github.com/dashevo/platform/issues/369), [#370](https://github.com/dashevo/platform/issues/370), [#392](https://github.com/dashevo/platform/issues/392), [#444](https://github.com/dashevo/platform/issues/444))
* **dashmate:** add hardcoded stop grace time before killing services ([#536](https://github.com/dashevo/platform/issues/536))


### Bug Fixes

* **dapi:** Cannot read properties of undefined (reading 'PoSePenalty') ([#612](https://github.com/dashpay/platform/issues/612))
* **test-suite:** `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))
* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))
* **dashmate:** Core RPC is not responding ([#575](https://github.com/dashevo/platform/issues/575))
* **drive:** db is in readonly mode due to the active transaction ([#567](https://github.com/dashevo/platform/issues/567))
* **sdk:** `platform.initialize` is not a function ([#555](https://github.com/dashevo/platform/issues/555))
* **dapi-client:** metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492), [#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from beginning after restart ([#542](https://github.com/dashevo/platform/issues/542))
* **dashmate:** DockerComposeError undefined ([#480](https://github.com/dashevo/platform/issues/480), [#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))
* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))
* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** node can be marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))
* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))
* **dpp:** cannot read properties of null (reading 'getBalance') ([#549](https://github.com/dashevo/platform/issues/549))
* **dashmate**: can't use local seed as a wallet ([#538](https://github.com/dashevo/platform/issues/538))


### Miscellaneous Chores

* add yarn cache ([#637](https://github.com/dashpay/platform/issues/637))
* **drive:** update RS Drive to 0.23.0-dev.9 ([#588](https://github.com/dashpay/platform/issues/588))
* **dashmate:** update testnet credentials ([#571](https://github.com/dashevo/platform/issues/571))
* **dapi-client:** update dapi addresses white list ([#574](https://github.com/dashevo/platform/issues/574))
* **dashmate:** use latest DAPI and Drive 0.23.0-alpha ([#568](https://github.com/dashevo/platform/issues/568))
* **drive:** correct typo in debug message ([#535](https://github.com/dashevo/platform/issues/535))
* **test-suite:** remove unused merk dependency ([#547](https://github.com/dashevo/platform/issues/547))
* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))
* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))
* **scripts**: remove dashmate update ([#550](https://github.com/dashevo/platform/issues/550))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))
* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))
* **wallet-lib:** fixes wallet.spec.js


### Continuous Integration

* update deps ([#591](https://github.com/dashpay/platform/issues/591))
* add dashmate and test suite ([#551](https://github.com/dashevo/platform/issues/551), [#576](https://github.com/dashevo/platform/issues/576))
* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))
* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))
* docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))


### Documentation

* GitHub org change updates ([#590](https://github.com/dashpay/platform/issues/590))
* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))
* **dashmate:** add troubleshooting section ([#431](https://github.com/dashevo/platform/issues/431))
* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))
* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Build System

* **dapi-grpc:** update protoc builder image ([#553](https://github.com/dashpay/platform/issues/553), [#647](https://github.com/dashpay/platform/issues/647))
* remove buildx bake workaround ([#541](https://github.com/dashpay/platform/issues/541))
* release arbitrary pre-release tags ([#552](https://github.com/dashevo/platform/issues/552), [#558](https://github.com/dashevo/platform/issues/558), [#560](https://github.com/dashevo/platform/issues/560), [#562](https://github.com/dashevo/platform/issues/562), [#566](https://github.com/dashevo/platform/issues/566))
* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build



## [0.23.0-alpha.9](https://github.com/dashpay/platform/compare/v0.23.0-alpha.8...v0.23.0-alpha.9) (2022-11-21)


### ⚠ BREAKING CHANGES

* precise storage fees (#619)

### Features

* precise storage fees ([#619](https://github.com/dashpay/platform/issues/619))


### Bug Fixes

* **dapi:** Cannot read properties of undefined (reading 'PoSePenalty') ([#612](https://github.com/dashpay/platform/issues/612))

## [0.24.0-dev.8](https://github.com/dashpay/platform/compare/v0.24.0-dev.7...v0.24.0-dev.8) (2022-11-18)


### Features

* add `withdrawals` data contract package ([#604](https://github.com/dashpay/platform/issues/604))
* **dpp:** dashpay datatrigger toUserIds better validation ([#620](https://github.com/dashpay/platform/issues/620))
* **drive:** select the most vital validator set quorums ([#617](https://github.com/dashpay/platform/issues/617))


### Bug Fixes

* DataContract.spec.js in wasm-dpp ([#618](https://github.com/dashpay/platform/issues/618))
* renamed method from rs-dpp ([#623](https://github.com/dashpay/platform/issues/623))

## [0.24.0-dev.7](https://github.com/dashpay/platform/compare/v0.23.0-dev.6...v0.24.0-dev.7) (2022-11-07)


### ⚠ BREAKING CHANGES

* **drive:** same block execution (#593)

### Features

* **dpp:** initial RS DPP integration ([#483](https://github.com/dashpay/platform/issues/483))
* **drive:** same block execution ([#593](https://github.com/dashpay/platform/issues/593))


### Bug Fixes

* `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))
* **dapi:** `getConsensusParamsHandler` was handling wrong Tendermint error ([#601](https://github.com/dashpay/platform/issues/601))
* **dashmate:** invalid testnet TenderDash genesis ([#608](https://github.com/dashpay/platform/issues/608))
* **dashmate:** SSL verification server cannot be started ([#606](https://github.com/dashpay/platform/issues/606))

## [0.23.0-alpha.8](https://github.com/dashpay/platform/compare/v0.23.0-alpha.7...v0.23.0-alpha.8) (2022-11-04)


### Bug Fixes

* **test-suite:** `featureFlags` test was awaiting blocks that have not been produced ([#602](https://github.com/dashpay/platform/issues/602))


### Continuous Integration

* update deps ([#591](https://github.com/dashpay/platform/issues/591))


### Documentation

* GitHub org change updates ([#590](https://github.com/dashpay/platform/issues/590))


## [0.24.0-dev.6](https://github.com/dashpay/platform/compare/v0.24.0-dev.5...v0.24.0-dev.6) (2022-10-26)


### Bug Fixes

* typo `dash-amte` to `dashmate` ([#599](https://github.com/dashpay/platform/issues/599))

## [0.24.0-dev.5](https://github.com/dashpay/platform/compare/v0.24.0-dev.4...v0.24.0-dev.5) (2022-10-26)


### Bug Fixes

* release.yml contained wrong indentation ([#597](https://github.com/dashpay/platform/issues/597))

## [0.24.0-dev.4](https://github.com/dashpay/platform/compare/v0.24.0-dev.3...v0.24.0-dev.4) (2022-10-26)


### Build System

* publish envoy and dashmate-helper docker images after release ([#595](https://github.com/dashpay/platform/issues/595))

## [0.24.0-dev.3](https://github.com/dashpay/platform/compare/v0.23.0-alpha.7...v0.24.0-dev.3) (2022-10-20)


### Features

* **drive:** AssetUnlock transactions processing ([#530](https://github.com/dashpay/platform/issues/530))


## [0.23.0-alpha.7](https://github.com/dashpay/platform/compare/v0.23.0-alpha.6...v0.23.0-alpha.7) (2022-10-18)


### ⚠ BREAKING CHANGES

* Previous data is not compatible with new Drive


### Bug Fixes

* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))


### Documentation

* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))


### Build System

* **dapi-grpc:** update grpc protoc image ([#553](https://github.com/dashpay/platform/issues/553))
* remove buildx bake workaround ([#541](https://github.com/dashpay/platform/issues/541))


### Miscellaneous Chores

* **drive:** update RS Drive to 0.23.0-dev.9 ([#588](https://github.com/dashpay/platform/issues/588))


## [0.24.0-dev.2](https://github.com/dashpay/platform/compare/v0.23.0-alpha.5...v0.24.0-dev.2) (2022-10-13)


### Bug Fixes

* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))
* **dapi-client:** temporary use http protocol by default ([#573](https://github.com/dashpay/platform/issues/573))
* **dapi:** caching of headers prone to reorgs ([#578](https://github.com/dashpay/platform/issues/578))


### Documentation

* update URL from dashevo to dashpay ([#579](https://github.com/dashpay/platform/issues/579))


### Tests

* **drive:** synchronizeMasternodeIdentitiesFactory ([#586](https://github.com/dashpay/platform/issues/586))


## [0.23.0-alpha.6](https://github.com/dashpay/platform/compare/v0.23.0-alpha.5...v0.23.0-alpha.6) (2022-10-12)


### Bug Fixes

* 0.23-dev.10 version installed instead of alpha ([#581](https://github.com/dashpay/platform/issues/581))


## [0.23.0-alpha.5](https://github.com/dashevo/platform/compare/v0.23.0-alpha.4...v0.23.0-alpha.5) (2022-10-12)


### Continuous Integration

* run dashmate and test suite workflow ([#576](https://github.com/dashevo/platform/issues/576))


### Miscellaneous Chores

* **dashmate:** update testnet credentials ([#571](https://github.com/dashevo/platform/issues/571))

## [0.23.0-alpha.4](https://github.com/dashevo/platform/compare/v0.23.0-alpha.3...v0.23.0-alpha.4) (2022-10-11)


### Bug Fixes

* **dashmate:** Core RPC is not responding ([#575](https://github.com/dashevo/platform/issues/575))


### Miscellaneous Chores

* **dapi-client:** update dapi addresses white list ([#574](https://github.com/dashevo/platform/issues/574))

## [0.23.0-alpha.3](https://github.com/dashevo/platform/compare/v0.23.0-alpha.2...v0.23.0-alpha.3) (2022-10-10)


### Bug Fixes

* **drive:** db is in readonly mode due to the active transaction ([#567](https://github.com/dashevo/platform/issues/567))


### Build System

* fix release script wrong param names ([#566](https://github.com/dashevo/platform/issues/566))


### Miscellaneous Chores

* **dashmate:** use latest DAPI and Drive 0.23.0-alpha ([#568](https://github.com/dashevo/platform/issues/568))


## [0.24.0-dev.1](https://github.com/dashevo/platform/compare/v0.23.0-alpha.2...v0.24.0-dev.1) (2022-10-07)


### ⚠ BREAKING CHANGES

* migrate to ABCI++ (#464)
* Tenderdash 0.8 and lower not supported anymore

### Features

* **dashmate:** update tenderdash to 0.9.0-dev.1 ([#525](https://github.com/dashevo/platform/issues/525))
* **dashmate:** zeroSSL certificate renewal helper ([#554](https://github.com/dashevo/platform/issues/554))
* **dpp:** add wasm-dpp template package ([#529](https://github.com/dashevo/platform/issues/529))
* implement masternode voting identities ([#467](https://github.com/dashevo/platform/issues/467))
* migrate to ABCI++ ([#464](https://github.com/dashevo/platform/issues/464))
* SSL certificate for DAPI ([#519](https://github.com/dashevo/platform/issues/519))



## [0.23.0-alpha.2](https://github.com/dashevo/platform/compare/v0.23.0-alpha.1...v0.23.0-alpha.2) (2022-10-07)


### Features

* **dashmate:** add `core reindex` command ([#533](https://github.com/dashevo/platform/issues/533)), closes [#538](https://github.com/dashevo/platform/issues/538)


### Bug Fixes

* **sdk:** platform.initialize is not a function ([#555](https://github.com/dashevo/platform/issues/555))


### Continuous Integration

* add dashmate and test suite ([#551](https://github.com/dashevo/platform/issues/551))
* release arbitrary pre-release tags ([#552](https://github.com/dashevo/platform/issues/552))


### Build System

* support alpha prereleases ([#560](https://github.com/dashevo/platform/issues/560), #558](https://github.com/dashevo/platform/issues/558))
* release can be made from any branch if type is defined ([#562](https://github.com/dashevo/platform/issues/562))


## [0.23.0-alpha.1](https://github.com/dashevo/platform/compare/v0.22.16...v0.23.0-alpha.1) (2022-09-28)


### ⚠ BREAKING CHANGES

* Dash Core version lower than 18.1.0-rc.1 is not supported
* Blockchain data and state structure are changed and incompatible with previous versions
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* Data Contract indices must have 'asc' order (#435)
* Document query logic can behave differently in some cases (#398)
* Identity master key can be used only to update identity
* Use plain proRegTx for masternode identifier
* Incompatible wallet-lib storage format


### Features

* update Core to 18.1.0-rc.1 ([#544](https://github.com/dashevo/platform/issues/544), [#526](https://github.com/dashevo/platform/issues/526), [#511](https://github.com/dashevo/platform/issues/511))
* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458), [#474](https://github.com/dashevo/platform/issues/474), [#484](https://github.com/dashevo/platform/issues/484))
* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* Integrate with Tenderdash ABCI++ ([#314](https://github.com/dashevo/platform/issues/314))
* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292), [#477](https://github.com/dashevo/platform/issues/477), [#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** rework storage for multiple key-chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376), [#387](https://github.com/dashevo/platform/issues/387), [#369](https://github.com/dashevo/platform/issues/369), [#370](https://github.com/dashevo/platform/issues/370), [#392](https://github.com/dashevo/platform/issues/392), [#444](https://github.com/dashevo/platform/issues/444))
* **dashmate:** add hardcoded stop grace time before killing services ([#536](https://github.com/dashevo/platform/issues/536))


### Bug Fixes

* **dapi-client:** metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492), [#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from beginning after restart ([#542](https://github.com/dashevo/platform/issues/542))
* **dashmate:** DockerComposeError undefined ([#480](https://github.com/dashevo/platform/issues/480), [#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))
* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))
* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** node can be marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))
* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))
* **dpp:** cannot read properties of null (reading 'getBalance') ([#549](https://github.com/dashevo/platform/issues/549))
* **dashmate**: can't use local seed as a wallet ([#538](https://github.com/dashevo/platform/issues/538))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))
* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))
* **wallet-lib:** fixes wallet.spec.js


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build


### Documentation

* **dashmate:** add troubleshooting section ([#431](https://github.com/dashevo/platform/issues/431))
* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))
* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Continuous Integration

* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))
* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))
* docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))


### Miscellaneous Chores

* **drive:** correct typo in debug message ([#535](https://github.com/dashevo/platform/issues/535))
* **test-suite:** remove unused merk dependency ([#547](https://github.com/dashevo/platform/issues/547))
* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))
* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))
* **scripts**: remove dashmate update ([#550](https://github.com/dashevo/platform/issues/550))



## [0.23.0-dev.10](https://github.com/dashevo/platform/compare/v0.22.16...v0.23.0-dev.10) (2022-09-27)


### ⚠ BREAKING CHANGES

* **drive:** masternode identities sync from begging after restart (#542)
* update Dash Core to 18.1.0-alpha.2 (#526)

### Features

* update Dash Core to 18.1.0-alpha.2 ([#526](https://github.com/dashevo/platform/issues/526))


### Bug Fixes

* **dapi-client:** metadata.get is not a function once again ([#531](https://github.com/dashevo/platform/issues/531))
* **dashmate:** homedir fs.exists check ([#537](https://github.com/dashevo/platform/issues/537))
* **drive:** masternode identities sync from begging after restart ([#542](https://github.com/dashevo/platform/issues/542))


### Miscellaneous Chores

* clean up dependencies ([#534](https://github.com/dashevo/platform/issues/534))
* **scripts:** remove comment from env key ([#532](https://github.com/dashevo/platform/issues/532))


### [0.22.16](https://github.com/dashevo/platform/compare/v0.22.15...v0.22.16) (2022-09-08)


### Features

* create test suite's `.env` from deployment tool config ([#518](https://github.com/dashevo/platform/issues/518))


### Bug Fixes

* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))



## [0.23.0-dev.9](https://github.com/dashevo/platform/compare/v0.22.15...v0.23.0-dev.9) (2022-09-05)


### Bug Fixes

* **dashmate:** DockerComposeError undefined ([#513](https://github.com/dashevo/platform/issues/513))
* security vulnerability in elliptic library ([#501](https://github.com/dashevo/platform/issues/501))
* test suite image and environment problems in ci ([#505](https://github.com/dashevo/platform/issues/505))
* **test-suite:** failing assertions due to replication lag ([#502](https://github.com/dashevo/platform/issues/502))


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))


### Tests

* **dapi:** fix broken subscribeToNewBlockHeaders test ([#508](https://github.com/dashevo/platform/issues/508))
* **dapi:** rename test files to mach the naming convention ([#509](https://github.com/dashevo/platform/issues/509))


### [0.22.15](https://github.com/dashevo/platform/compare/v0.22.14...v0.22.15) (2022-08-31)


### Features

* **dashmate:** update v18 for the mainnet nodes ([#514](https://github.com/dashevo/platform/issues/514))


### [0.22.14](https://github.com/dashevo/platform/compare/v0.22.13...v0.22.14) (2022-08-29)


### Features

* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* integrate dash-spv into monorepo


### Documentation

* **dashmate:** update dashmate documentation ([#459](https://github.com/dashevo/platform/issues/459))
* **sdk:** update outdated documentation ([#463](https://github.com/dashevo/platform/issues/463))


### Miscellaneous Chores

* bump dash core to v18.0.1 and sentinel to 1.7.1 ([#511](https://github.com/dashevo/platform/issues/511))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))


### Build System

* `yarn install` fails with Node.JS 16.17.0 ([#507](https://github.com/dashevo/platform/issues/507))
* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))
* **test-suite:** fix docker image build

## [0.23.0-dev.8](https://github.com/dashevo/platform/compare/v0.23.0-dev.7...v0.23.0-dev.8) (2022-08-22)


### Bug Fixes

* **dapi:** getStatus errored if masternode is banned ([#496](https://github.com/dashevo/platform/issues/496))
* Drive and DAPI expect data available on H+1 block ([#497](https://github.com/dashevo/platform/issues/497))
* **test-suite:** failing tests due to replication latency ([#500](https://github.com/dashevo/platform/issues/500))

## [0.23.0-dev.7](https://github.com/dashevo/platform/compare/v0.23.0-dev.6...v0.23.0-dev.7) (2022-08-18)


### Bug Fixes

* DAPI client requests one local single node only ([#479](https://github.com/dashevo/platform/issues/479))
* **dapi-client:** A.metadata.get is not a function ([#492](https://github.com/dashevo/platform/issues/492))
* **dapi-client:** node can bed marked as banned on retriable error ([#482](https://github.com/dashevo/platform/issues/482))
* **dashmate:** docker undefined error ([#480](https://github.com/dashevo/platform/issues/480))
* **dashmate:** gRPC stream timeout ([#481](https://github.com/dashevo/platform/issues/481))
* infinite block production due to fees distribution ([#484](https://github.com/dashevo/platform/issues/484))
* **sdk:** cannot read properties of null (reading 'getMetadata') ([#488](https://github.com/dashevo/platform/issues/488))
* **test-suite:** invalid transaction: Missing inputs ([#494](https://github.com/dashevo/platform/issues/494))


### Miscellaneous Chores

* **dashmate:** update Core to 18.0.0-rc.12


### Build System

* new node alpine image breaks package builds ([#493](https://github.com/dashevo/platform/issues/493))

## [0.23.0-dev.6](https://github.com/dashevo/platform/compare/v0.23.0-dev.5...v0.23.0-dev.6) (2022-08-12)


### ⚠ BREAKING CHANGES

* Correct cumulative fees invalidate previous blockchain data (#474)

### Bug Fixes

* **dpp:** repeated disabling of already disabled identity key ([#477](https://github.com/dashevo/platform/issues/477))
* **drive:** cumulative fees are not reset between blocks ([#474](https://github.com/dashevo/platform/issues/474))
* **drive:** invalid previous block time ([#475](https://github.com/dashevo/platform/issues/475))

## [0.23.0-dev.5](https://github.com/dashevo/platform/compare/v0.23.0-dev.4...v0.23.0-dev.5) (2022-08-05)


### ⚠ BREAKING CHANGES

* Blockchain data and state structure are changed and incompatible with previous versions

### Features

* **drive:** fees distribution ([#458](https://github.com/dashevo/platform/issues/458))


### Miscellaneous Chores

* various linter fixes across packages ([#465](https://github.com/dashevo/platform/issues/465))

## [0.23.0-dev.4](https://github.com/dashevo/platform/compare/v0.23.0-dev.3...v0.23.0-dev.4) (2022-07-12)


### Bug Fixes

* **dashmate:** replace `seeds` by `bootstrap-peers` in config.toml ([#460](https://github.com/dashevo/platform/issues/460))
* **drive:** various fixes in synchronize masternode identities logic and logging ([#461](https://github.com/dashevo/platform/issues/461))


### Build System

* **test-suite:** fix docker image build


## [0.23.0-dev.3](https://github.com/dashevo/platform/compare/v0.22.13...v0.23.0-dev.3) (2022-06-30)


### ⚠ BREAKING CHANGES

* Previous invalid data contracts in blockchain might be valid now (#445)
* `getIdentityIdsByPublicKeyHash` endpoint is removed. `getIdentitiesByPublicKeyHash` now responds with an array of identities, instead of an array of cbored arrays of identities. (#437)
* All indices must have 'asc' order (#435)
* Some state transitions in the chain could change validation result due to changes in fee logic. Previously invalid state transition in chain could become valid since BLS signing is fixed (#392)
* Previously invalidated `DataContractUpdateTransitions` with `unique` equals `false` will become valid (#427)
* Document query logic can behave differently in some cases (#398)

### Features

* **bench:** state transition benchmark ([#418](https://github.com/dashevo/platform/issues/418))
* **dashmate:** add --force flag to stop command ([#434](https://github.com/dashevo/platform/issues/434))
* **dashmate:** upgrade docker compose to v2 ([#441](https://github.com/dashevo/platform/issues/441))
* **dpp:** allow 1 char document type and 1 char property name ([#445](https://github.com/dashevo/platform/issues/445))
* integrate dash-spv into monorepo
* limit the number of shares for masternode by 16 ([#432](https://github.com/dashevo/platform/issues/432))
* move dash-spv in packages after import
* re-enable proof responses ([#440](https://github.com/dashevo/platform/issues/440))
* validate fee calculating worst case operations ([#392](https://github.com/dashevo/platform/issues/392))


### Bug Fixes

* **ci:** docker images incorrectly tagged with v ([#413](https://github.com/dashevo/platform/issues/413))
* **dpp:** data contract index update validation ([#427](https://github.com/dashevo/platform/issues/427))
* **drive:** change transaction is started check ([#451](https://github.com/dashevo/platform/issues/451))
* non-deterministic fees due to data contract cache ([#444](https://github.com/dashevo/platform/issues/444))
* **sdk:** identity update method can't sign publicKeys in some cases ([#421](https://github.com/dashevo/platform/issues/421))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))


### Documentation

* add input description


### Code Refactoring

* **drive:** use RS Drive query validation logic ([#398](https://github.com/dashevo/platform/issues/398))
* simplified public key to identity structure ([#437](https://github.com/dashevo/platform/issues/437))


### Tests

* **dpp:** fix invalid findIndexDuplicates test in DPP ([#448](https://github.com/dashevo/platform/issues/448))


### Miscellaneous Chores

* **dpp:** allow only `asc` order for indices ([#435](https://github.com/dashevo/platform/issues/435))
* **drive:** log synchronize masternode identities ([#449](https://github.com/dashevo/platform/issues/449))
* **test-suite:** move wallet storage persistence in the outer folder ([#416](https://github.com/dashevo/platform/issues/416))
* update readme


### [0.22.13](https://github.com/dashevo/platform/compare/v0.22.12...v0.22.13) (2022-06-17)


### Features

* support DIP24 devnet LLMQ type ([#438](https://github.com/dashevo/platform/issues/438))

### [0.22.12](https://github.com/dashevo/platform/compare/v0.22.11...v0.22.12) (2022-06-07)


### Bug Fixes

* **sdk:** incomplete bundle for web ([#400](https://github.com/dashevo/platform/issues/400))
* **wallet-lib:** separate persistent storage by walletId ([#407](https://github.com/dashevo/platform/issues/407))

### [0.22.11](https://github.com/dashevo/platform/compare/v0.22.10...v0.22.11) (2022-05-31)


### Bug Fixes

* incorrect image versions and variables for testnet config ([#415](https://github.com/dashevo/platform/issues/415))

### [0.22.10](https://github.com/dashevo/platform/compare/v0.22.9...v0.22.10) (2022-05-26)


### Bug Fixes

* CommitmentTxPayload#toBuffer method was using version instead of qfcVersion for serialization ([#410](https://github.com/dashevo/platform/issues/410))


### Continuous Integration

* dispatch trigger and parallelization ([#406](https://github.com/dashevo/platform/issues/406))

### [0.22.9](https://github.com/dashevo/platform/compare/v0.22.8...v0.22.9) (2022-05-24)


### Bug Fixes

* incorrect parsing of commitment payload ([#408](https://github.com/dashevo/platform/issues/408))

### [0.22.8](https://github.com/dashevo/platform/compare/v0.22.7...v0.22.8) (2022-05-23)


### Bug Fixes

* `verifyChainLock` was returning `false` instead of `ResponseQuery` ([#402](https://github.com/dashevo/platform/issues/402))
* **dashmate:** switch `drive` and `dapi` to stable versions ([#381](https://github.com/dashevo/platform/issues/381))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))


## [0.23.0-dev.2](https://github.com/dashevo/platform/compare/v0.23.0-dev.1...v0.23.0-dev.2) (2022-05-20)


### ⚠ BREAKING CHANGES

* Identity master key can be used only to update identity (#384)
* SDK's identity update method now requires correspond private keys. Identity public keys in state transitions must be signed

### Features

* bench suite ([#335](https://github.com/dashevo/platform/issues/335))
* **bench-suite:** add fees to documents benchmark ([#379](https://github.com/dashevo/platform/issues/379))
* **bench-suite:** function benchmark and other improvements ([#344](https://github.com/dashevo/platform/issues/344))
* calculate state transition fees using operations ([#376](https://github.com/dashevo/platform/issues/376))
* create withdrawal keys for masternode identities ([#320](https://github.com/dashevo/platform/issues/320))
* **dpp:** BIP13_SCRIPT_HASH identity public key type ([#353](https://github.com/dashevo/platform/issues/353))
* **dpp:** calculate signature verification costs for fees ([#387](https://github.com/dashevo/platform/issues/387))
* **dpp:** fee operations and execution context ([#369](https://github.com/dashevo/platform/issues/369))
* **drive:** collect fee operation to execution context ([#370](https://github.com/dashevo/platform/issues/370))
* Identity master key can be used only to update identity ([#384](https://github.com/dashevo/platform/issues/384))
* identity public key proofs ([#349](https://github.com/dashevo/platform/issues/349))
* integrate with Tenderdash v0.8-dev ([#314](https://github.com/dashevo/platform/issues/314))


### Bug Fixes

* change allowed security level for withdrawal purpose to critical ([#352](https://github.com/dashevo/platform/issues/352))
* **dapi-grpc:** outdated autogenerated code ([#331](https://github.com/dashevo/platform/issues/331))
* **dashmate:** switch `drive` and `dapi` to stable versions ([#381](https://github.com/dashevo/platform/issues/381))
* **wallet-lib:** hook tx chain broadcast on mempool response ([#388](https://github.com/dashevo/platform/issues/388))


### Documentation

* update badges in individual package readmes ([#361](https://github.com/dashevo/platform/issues/361))


### Continuous Integration

* add `latest-dev` docker tag ([#382](https://github.com/dashevo/platform/issues/382))


### Miscellaneous Chores

* **dashmate:** use 0.23-dev images


### [0.22.7](https://github.com/dashevo/platform/compare/v0.22.6...v0.22.7) (2022-05-02)


### Bug Fixes

* invalid version to parse `CommitmentTxPayload` ([#373](https://github.com/dashevo/platform/issues/373))

### [0.22.6](https://github.com/dashevo/platform/compare/v0.22.5...v0.22.6) (2022-05-02)


### Bug Fixes

* can't parse `CommitmentTxPayload` ([#371](https://github.com/dashevo/platform/issues/371))

### [0.22.5](https://github.com/dashevo/platform/compare/v0.22.4...v0.22.5) (2022-04-29)


### Bug Fixes

* broken QuorumEntry unserialization ([#366](https://github.com/dashevo/platform/issues/366))

### [0.22.4](https://github.com/dashevo/platform/compare/v0.22.3...v0.22.4) (2022-04-29)


### ⚠ BREAKING CHANGES

* Core v0.17 is not supported anymore

### Bug Fixes

* invalid `merkleRootQuorums` calculation ([#362](https://github.com/dashevo/platform/issues/362))

### [0.22.3](https://github.com/dashevo/platform/compare/v0.22.2...v0.22.3) (2022-04-27)


### ⚠ BREAKING CHANGES

* **wallet-lib:** storage layer refactoring (#232)

### Features

* **wallet-lib:** adds balance and metadata information from registered identity ([#337](https://github.com/dashevo/platform/issues/337))
* **wallet-lib:** provide transaction history item as a date object ([#336](https://github.com/dashevo/platform/issues/336))
* **wallet-lib:** rework storage for multiple key chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Bug Fixes

* **dashmate:** broken migrations ([#355](https://github.com/dashevo/platform/issues/355))
* **wallet-lib:** optimize storage version check ([#348](https://github.com/dashevo/platform/issues/348))
* **wallet-lib:** persistent storage regression ([#302](https://github.com/dashevo/platform/issues/302))


### [0.22.2](https://github.com/dashevo/platform/compare/v0.22.1...v0.22.2) (2022-04-21)


### Bug Fixes

* docker-test-suite missing test files


### Tests

* **dpp:** double test in identity validation ([#330](https://github.com/dashevo/platform/issues/330))
* fixes sdk timeouts in platform test suite ([#309](https://github.com/dashevo/platform/issues/309))


### Miscellaneous Chores

* update Core to v0.18.0.0-rc1 ([#351](https://github.com/dashevo/platform/issues/351))


## [0.23.0-dev.1](https://github.com/dashevo/platform/compare/v0.22.0...v0.23.0-dev.1) (2022-04-08)


### ⚠ BREAKING CHANGES

* plain proRegTx for masternode identifier (#318)
* **wallet-lib:** storage layer refactoring (#232)

### Features

* **dpp:** add `withdraw` purpose for `IdentityPublicKey` ([#317](https://github.com/dashevo/platform/issues/317))
* update identity ([#292](https://github.com/dashevo/platform/issues/292))
* **wallet-lib:** rework storage for multiple key chains ([#231](https://github.com/dashevo/platform/issues/231))
* **wallet-lib:** satoshisBalanceImpact in transaction history ([#319](https://github.com/dashevo/platform/issues/319))
* **wallet-lib:** storage layer refactoring ([#232](https://github.com/dashevo/platform/issues/232))


### Bug Fixes

* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))
* do not hash proRegTx for masternode identifier ([#318](https://github.com/dashevo/platform/issues/318))


### Performance Improvements

* **dapi:** cache block headers and chainlocks ([#235](https://github.com/dashevo/platform/issues/235), [#296](https://github.com/dashevo/platform/issues/296))
* **dapi:** remove unnecessary Core RPC calls for core streams ([#194](https://github.com/dashevo/platform/issues/194))


### Continuous Integration

* enable multiarch builds ([#316](https://github.com/dashevo/platform/issues/316))


### Miscellaneous Chores

* **drive:** add more block execution timers ([#329](https://github.com/dashevo/platform/issues/329))


### Tests

* fixes wallet.spec.js

### [0.22.1](https://github.com/dashevo/platform/compare/v0.22.0...v0.22.1) (2022-03-25)


### Bug Fixes

* **dashmate:** cannot read properties of undefined (reading 'masternodeRewardShares’) ([#310](https://github.com/dashevo/platform/issues/310))
* **dashmate:** config/core/miner must have required property 'interval' ([#311](https://github.com/dashevo/platform/issues/311))


### Tests

* fix platform-test-suite-execution in browser environment ([#289](https://github.com/dashevo/platform/issues/289))


## [0.22.0](https://github.com/dashevo/platform/compare/v0.21.8...v0.22.0) (2022-03-21)

### ⚠ BREAKING CHANGES

* `name` is required for document index definition
* `platform.contracts.broadcast` method in SDK renamed to `platform.contracts.publish`
* Identity public key requires `purpose` and `securityLevel` properties
* `$id` property can't be used in document indices
* Indexed properties now require size constraints
* `getIdentitiesByPublicKeyHashes` returns array of arrays of identities
* `getIdentityIdsByPublicKeyHashes` returns array of arrays of identity ids
* Document array properties temporarily cannot be indexed. Will be enabled in v0.23
* Range operations in document queries can be used only in the last where clause
* sorting (`orderBy`) in document queries is required for range operations
* `elementMatch`, `contains` and `includes` operations are temporarily disabled in document query. Will be enabled in v0.23
* `$ref` in data contract is temporarily disabled
* `startAt` and `startAfter` accept now only document id instead of document offset
* `in` operator can be used only in two last where clauses
* Cryptographical proofs for platform state are temporarily disabled. Will be enabled in upcoming releases
* Platform data is not compatible with previous platform versions. Please reset your node.


### Features

* identity public key purpose and security levels ([#46](https://github.com/dashevo/platform/issues/46))
* allow using non-unique Identity public keys ([#168](https://github.com/dashevo/platform/issues/168))
* distribute dashmate with NPM ([#148](https://github.com/dashevo/platform/issues/148))
* create and update masternode identities ([#160](https://github.com/dashevo/platform/issues/160), [#170](https://github.com/dashevo/platform/issues/170), [#257](https://github.com/dashevo/platform/issues/257), [#272](https://github.com/dashevo/platform/issues/272), [#279](https://github.com/dashevo/platform/issues/279), [#287](https://github.com/dashevo/platform/issues/287))
* added WalletStore ([#197](https://github.com/dashevo/platform/issues/197))
* register system contracts on `initChain` ([#182](https://github.com/dashevo/platform/issues/182), [#192](https://github.com/dashevo/platform/issues/192))
* integrate new storage (GroveDB) and secondary indices (RS Drive) ([#77](https://github.com/dashevo/platform/issues/77), [#177](https://github.com/dashevo/platform/issues/177), [#178](https://github.com/dashevo/platform/issues/178), [#199](https://github.com/dashevo/platform/issues/199), [#201](https://github.com/dashevo/platform/issues/201), [#225](https://github.com/dashevo/platform/issues/225), [#259](https://github.com/dashevo/platform/issues/259), [#280](https://github.com/dashevo/platform/issues/280), [#303](https://github.com/dashevo/platform/issues/303))
* fallback to chain asset lock proof ([#297](https://github.com/dashevo/platform/issues/297))
* add an ability to update data contract ([#52](https://github.com/dashevo/platform/issues/52), [#83](https://github.com/dashevo/platform/issues/83), [#223](https://github.com/dashevo/platform/issues/223))
* add required `name` property to index definition ([#74](https://github.com/dashevo/platform/issues/74))
* use document for `startAt` and `startAfter` in document query ([#227](https://github.com/dashevo/platform/pull/227), [#255](https://github.com/dashevo/platform/issues/255))
* **dashmate:** enable mainnet for dashmate ([#2](https://github.com/dashevo/platform/issues/2))
* **dashmate:** json output for status commands ([#31](https://github.com/dashevo/platform/issues/31), [#262](https://github.com/dashevo/platform/issues/262))
* **dashmate:** add an ability to configure node subnet mask ([#237](https://github.com/dashevo/platform/issues/237))
* **dpp:** add `readOnly` flag to `IdentityPublicKey` ([#142](https://github.com/dashevo/platform/issues/142), [#239](https://github.com/dashevo/platform/issues/239))
* **dpp:** allow using BLS key to sign state transitions ([#268](https://github.com/dashevo/platform/issues/268), [#275](https://github.com/dashevo/platform/issues/275))
* **drive:** network address in `ValidatorUpdate` ABCI ([#140](https://github.com/dashevo/platform/issues/140), [#155](https://github.com/dashevo/platform/issues/155), [#184](https://github.com/dashevo/platform/issues/184))
* **drive:** add performance timers to measure block execution ([#281](https://github.com/dashevo/platform/issues/281))
* **dapi:** `subscribeToBlockHeadersWithChainLocks` endpoint ([#153](https://github.com/dashevo/platform/issues/153))
* **wallet-lib:** ChainStore ([#196](https://github.com/dashevo/platform/issues/196))
* **dapi-client:** get and verify block headers with dash-spv ([#211](https://github.com/dashevo/platform/issues/211))
* **dapi-client:** handle asynchronous errors ([#233](https://github.com/dashevo/platform/issues/233))


### Bug Fixes

* **dashmate:** `cannot read properties of undefined (reading 'dpns')` on reset ([#47](https://github.com/dashevo/platform/issues/47))
* **drive:** missed JS ABCI yarn cache ([#156](https://github.com/dashevo/platform/issues/156))
* **build:** `zeromq` build is not working on linux ([#236](https://github.com/dashevo/platform/issues/236))
* cannot install `protobufjs` in some cases ([#266](https://github.com/dashevo/platform/issues/266), [#267](https://github.com/dashevo/platform/issues/267))
* **dashmate:** `rimraf` module could not remove config directory ([#248](https://github.com/dashevo/platform/issues/248))
* **dashmate:** logs were incorrectly mounted ([#261](https://github.com/dashevo/platform/issues/261))
* **drive:** documents have mixed owner ids ([#283](https://github.com/dashevo/platform/issues/283))
* cannot read properties of undefined (reading 'getIp') ([#285](https://github.com/dashevo/platform/issues/285))
* InstantLock waiting period for transaction... ([#293](https://github.com/dashevo/platform/issues/293))
* **dpp:** re2 memory leak ([#301](https://github.com/dashevo/platform/issues/301))
* **drive:** internal error on verify instant lock ([#295](https://github.com/dashevo/platform/issues/295))


### Documentation

* improved sidebar and usage in DAPI client ([#3](https://github.com/dashevo/platform/issues/3))
* provide getTransactionHistory ([#5](https://github.com/dashevo/platform/issues/5))
* minor Readme fixes ([#163](https://github.com/dashevo/platform/issues/163))
* add readme to docs folder ([#175](https://github.com/dashevo/platform/issues/175))
* escape literal '|' in table ([#164](https://github.com/dashevo/platform/issues/164))
* indicate which network(s) this repo supports ([#174](https://github.com/dashevo/platform/issues/174))
* ignore folder with empty docs during build ([#212](https://github.com/dashevo/platform/issues/212))


### Tests

* **wallet-lib:** enable skipped test after the fix for grpc-js lib ([#71](https://github.com/dashevo/platform/issues/71))


### Miscellaneous Chores

* fix wrong version in a release PR title ([#82](https://github.com/dashevo/platform/issues/82))
* missed merk darwin x64 pre-build binary ([#144](https://github.com/dashevo/platform/issues/144))
* undefined "-w" argument in restart script ([#85](https://github.com/dashevo/platform/issues/85))
* **drive:** send initial core chain locked height on init chain ([#180](https://github.com/dashevo/platform/issues/180))
* update to use current @oclif/core ([#154](https://github.com/dashevo/platform/issues/154))
* remove `fixCumulativeFeesBug` feature flag ([#191](https://github.com/dashevo/platform/issues/191))
* update tenderdash and core images ([#188](https://github.com/dashevo/platform/issues/188), [#252](https://github.com/dashevo/platform/issues/252), [#269](https://github.com/dashevo/platform/issues/269))
* **dpp:** temporarily disable $refs in data contract definitions ([#300](https://github.com/dashevo/platform/issues/300))
* **dpp:** size constraints for indexed properties ([#179](https://github.com/dashevo/platform/issues/179), [#273](https://github.com/dashevo/platform/issues/273))


### Build System

* **test-suite:** docker image build doesn't work ([#172](https://github.com/dashevo/platform/issues/172))
* fix configure test suite script for grep 2.5.1 ([#187](https://github.com/dashevo/platform/issues/187))


### Code Refactoring

* **dapi:** rename tx-filter-stream.js to core-streams.js ([#169](https://github.com/dashevo/platform/issues/169))


## [0.22.0-dev.16](https://github.com/dashevo/platform/compare/v0.22.0-dev.15...v0.22.0-dev.16) (2022-03-18)


### ⚠ BREAKING CHANGES

* previously created platform state might be not compatible

### Features

* **dpp:** temporarily disable $refs in data contract definitions ([#300](https://github.com/dashevo/platform/issues/300))
* fallback to chain asset lock proof ([#297](https://github.com/dashevo/platform/issues/297))


### Bug Fixes

* **dpp:** re2 memory leak ([#301](https://github.com/dashevo/platform/issues/301))
* **drive:** document query and delete issues ([#303](https://github.com/dashevo/platform/issues/303))
* **drive:** internal error on verify instant lock ([#295](https://github.com/dashevo/platform/issues/295))

## [0.22.0-dev.15](https://github.com/dashevo/platform/compare/v0.22.0-dev.14...v0.22.0-dev.15) (2022-03-11)


### Bug Fixes

* InstantLock waiting period for transaction.. ([#293](https://github.com/dashevo/platform/issues/293))

## [0.22.0-dev.14](https://github.com/dashevo/platform/compare/v0.22.0-dev.13...v0.22.0-dev.14) (2022-03-10)


### ⚠ BREAKING CHANGES

* The fixed masternode identities logic breaks compatibility with previous invalid state.

### Bug Fixes

* **drive:** non-deterministic behaviour in masternode identities logic  ([#287](https://github.com/dashevo/platform/issues/287))

## [0.22.0-dev.13](https://github.com/dashevo/platform/compare/v0.22.0-dev.12...v0.22.0-dev.13) (2022-03-09)


### Bug Fixes

* cannot read properties of undefined (reading 'getIp') ([#285](https://github.com/dashevo/platform/issues/285))

## [0.22.0-dev.12](https://github.com/dashevo/platform/compare/v0.22.0-dev.11...v0.22.0-dev.12) (2022-03-08)


### Bug Fixes

* **drive:** documents have mixed owner ids ([#283](https://github.com/dashevo/platform/issues/283))

## [0.22.0-dev.11](https://github.com/dashevo/platform/compare/v0.22.0-dev.10...v0.22.0-dev.11) (2022-03-08)


### ⚠ BREAKING CHANGES

* `in` query operator doesn't work with multiple values (#280)

### Features

* **drive:** add performance timers to measure block execution ([#281](https://github.com/dashevo/platform/issues/281))


### Bug Fixes

* `in` query operator doesn't work with multiple values ([#280](https://github.com/dashevo/platform/issues/280))
* can't find masternode raward shares data contract ([#279](https://github.com/dashevo/platform/issues/279))

## [0.22.0-dev.10](https://github.com/dashevo/platform/compare/v0.22.0-dev.9...v0.22.0-dev.10) (2022-03-07)


### Bug Fixes

* **dpp:** Invalid DER format public key ([#275](https://github.com/dashevo/platform/issues/275))

## [0.22.0-dev.9](https://github.com/dashevo/platform/compare/v0.22.0-dev.8...v0.22.0-dev.9) (2022-03-04)


### ⚠ BREAKING CHANGES

* **dpp:** lower indexed string properties constraints (#273)

### Features

* **dpp:** lower indexed string properties constraints ([#273](https://github.com/dashevo/platform/issues/273))


### Bug Fixes

* masternode reward shares ([#272](https://github.com/dashevo/platform/issues/272))

## [0.22.0-dev.8](https://github.com/dashevo/platform/compare/v0.21.8...v0.22.0-dev.8) (2022-03-01)


### ⚠ BREAKING CHANGES

* New state is not compatible with previous versions
* Document queries have limitations compared with previous versions
* Proofs are temporary disabled

### Features

* **dapi-client:** get and verify block headers with dash-spv ([#211](https://github.com/dashevo/platform/issues/211))
* **dapi-client:** handle asynchronous errors ([#233](https://github.com/dashevo/platform/issues/233))
* **dashmate:** add an ability to configure node subnet mask ([#237](https://github.com/dashevo/platform/issues/237))
* **dpp:** allow using BLS key to sign state transitions ([#268](https://github.com/dashevo/platform/issues/268))
* **dpp:** do not allow to index array properties ([#225](https://github.com/dashevo/platform/issues/225))
* **drive:** create/update identities based on SML changes ([#170](https://github.com/dashevo/platform/issues/170))
* integrate RS Drive and GroveDB ([#177](https://github.com/dashevo/platform/issues/177))


### Bug Fixes

* **dashmate:** `group:status` command was missing a `format` flag ([#262](https://github.com/dashevo/platform/issues/262))
* `startAt` and `startAfter` invalid decoding ([#255](https://github.com/dashevo/platform/issues/255))
* **build:** `zeromq` build is not working on linux ([#236](https://github.com/dashevo/platform/issues/236))
* cannot install `protobufjs` in some cases ([#266](https://github.com/dashevo/platform/issues/266))
* **dashmate:** `rimraf` module could not remove config directory ([#248](https://github.com/dashevo/platform/issues/248))
* **dashmate:** logs were incorrectly mounted ([#261](https://github.com/dashevo/platform/issues/261))
* **dpp:** Identity public key `readOnly` flag was read as `undefined` instead of `false` ([#239](https://github.com/dashevo/platform/issues/239))
* **drive:** unable to reconstruct SML ([#257](https://github.com/dashevo/platform/issues/257))
* **drive:** invalid query errors are fatal ([#259](https://github.com/dashevo/platform/issues/259))
* **sdk:** can't update cached data contract ([#223](https://github.com/dashevo/platform/issues/223))


### Documentation

* ignore folder with empty docs during build ([#212](https://github.com/dashevo/platform/issues/212))


### Build System

* `protobufjs` isn't installing from git sometimes ([#267](https://github.com/dashevo/platform/issues/267))


### Miscellaneous Chores

* **dashmate:** update Core to 0.18.0.0-beta4 ([#269](https://github.com/dashevo/platform/issues/269))
* **release:** revert version back
* update tenderdash and core images ([#252](https://github.com/dashevo/platform/issues/252))



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
