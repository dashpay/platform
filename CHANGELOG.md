## [0.21.4](https://github.com/dashevo/platform/compare/v0.21.0...v0.21.4) (2021-11-23)


### Bug Fixes

* **dapi-client:** expect 100 but got 122 in SML provider test ([#22](https://github.com/dashevo/platform/issues/22))
* **dapi-client:** retry doesn’t work with 502 errors ([#35](https://github.com/dashevo/platform/issues/35))
* **dapi:** Identifier expects Buffer ([#28](https://github.com/dashevo/platform/issues/28))
* **dashmate:** ajv schema errors ([#14](https://github.com/dashevo/platform/issues/14))
* **dashmate:** reset command doesn't work if setup failed ([#23](https://github.com/dashevo/platform/issues/23))
* **dpp:** rename generateEntropy to entropyGenerator ([#13](https://github.com/dashevo/platform/issues/13))
* **sdk:** dpp hash function import ([#15](https://github.com/dashevo/platform/issues/15))
* **sdk:** override ts-node target for unit tests ([#21](https://github.com/dashevo/platform/issues/21))
* **sdk:** this is undefined during unit tests ([#18](https://github.com/dashevo/platform/issues/18))


### Features

* **dashmate:** force option for `group:stop` command ([#36](https://github.com/dashevo/platform/issues/36))
* **dashmate:** provide docker build logs for verbose mode ([#19](https://github.com/dashevo/platform/issues/19))
* migrate to DashCore 0.18.0.0-beta1 ([#51](https://github.com/dashevo/platform/issues/51))
* **wallet-lib:** do not sync transactions if mnemonic is absent


### Performance Improvements

* **test-suite:** speedup test suite up to 6 times ([#30](https://github.com/dashevo/platform/issues/30))


### Build System
* build only necessary packages ([#27](https://github.com/dashevo/platform/issues/27))


### BREAKING CHANGES

* supports only new DashCore InstantLock format https://github.com/dashpay/dips/blob/master/dip-0022.md



