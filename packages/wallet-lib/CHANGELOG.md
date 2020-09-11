# [7.15.1](https://github.com/dashevo/wallet-lib/compare/v7.15.0...v7.15.1) (2020-09-11)


### Bug Fixes

* false positive merkle blocks shouldn't be imported into the storage ([#185](https://github.com/dashevo/wallet-lib/issues/185))

# [7.15.0](https://github.com/dashevo/wallet-lib/compare/v7.14.0...v7.15.0) (2020-09-04)


### Bug Fixes

* confirmation might come before broadcast ACK ([#183](https://github.com/dashevo/wallet-lib/issues/183))
* outdated create transaction typing ([#180](https://github.com/dashevo/wallet-lib/issues/180))


### Code Refactoring

* switch from getUTXO to subscribeToTransactions ([#119](https://github.com/dashevo/wallet-lib/issues/119))


### BREAKING CHANGES

* removed `subscribeToAddressesTransactions`, `getUTXO` and `getAddressSummary` from the transport
* removed `Account#fetchAddressInfo` method



# [7.14.0](https://github.com/dashevo/wallet-lib/compare/v7.13.4...v7.14.0) (2020-07-23)


### Bug Fixes

* merge conflict artefact issue ([#170](https://github.com/dashevo/wallet-lib/issues/170))
* outdated network option values ([#167](https://github.com/dashevo/wallet-lib/issues/167))


### Features

* run tests against mn-bootstrap instead of devnet ([#168](https://github.com/dashevo/wallet-lib/issues/168))
* update to DAPI Client 0.14 and refactor transport layer ([#163](https://github.com/dashevo/wallet-lib/issues/163))


### Documentation

* readme standard updates ([#165](https://github.com/dashevo/wallet-lib/issues/165))
* update documentation and definitions files ([#154](https://github.com/dashevo/wallet-lib/issues/154))


### BREAKING CHANGES

* `transporter` option is replaced with `transport` that accepts [DAPI Client options](https://github.com/dashevo/dapi-client/blob/1ec21652f1615ba95ea537c38632692f81deefa3/lib/DAPIClient.js#L42-L51) or a Transport instance.



## [7.13.4](https://github.com/dashevo/wallet-lib/compare/v7.13.3...v7.13.4) (2020-07-01)


### Bug Fixes

* simple transaction do not have any 4 inputs limitation ([#158](https://github.com/dashevo/wallet-lib/issues/158)) ([11d8d01](https://github.com/dashevo/wallet-lib/commit/11d8d011a15e9000dfd8dc4bd22c449334835767))
* **account:** forward all storage events ([#159](https://github.com/dashevo/wallet-lib/issues/159)) ([e5c807e](https://github.com/dashevo/wallet-lib/commit/e5c807e1d0132d6fe0538e05f04e760ff0c0b1f3))


### Features

* update dashcore-lib and DAPI Client ([#161](https://github.com/dashevo/wallet-lib/issues/161)) ([81536d2](https://github.com/dashevo/wallet-lib/commit/81536d2235e335fed5fa53752b77260a4a7fa367))



## [7.13.4](https://github.com/dashevo/wallet-lib/compare/v7.13.3...v7.13.4) (2020-07-01)


### Bug Fixes

* simple transaction do not have any 4 inputs limitation ([#158](https://github.com/dashevo/wallet-lib/issues/158))
* **account:** forward all storage events ([#159](https://github.com/dashevo/wallet-lib/issues/159))


### Features

* update dashcore-lib and DAPI Client ([#161](https://github.com/dashevo/wallet-lib/issues/161))



# [7.13.3](https://github.com/dashevo/wallet-lib/compare/v7.13.2...v7.13.3) (2020-06-16)

- **Fixes:**  
    * fix!: createTransaction should be checking for 'recipient' instead of 'address' in 'txOpts.recipients' ([#152](https://github.com/dashevo/wallet-lib/pull/152))
    * fix: transaction hash not present on address ([#151](https://github.com/dashevo/wallet-lib/pull/151))

- **Breaking changes:**   
    * Previously, the documentation stated a usage on `createTransaction()` with multiples recipients as such: `recipients:[{recipient,satoshis}]`.   
    However, the code where still referring and expecting recipients `recipients:[{address,satoshis}]`.  
    This version fixes that inconsistency. 
     
# [7.13.2](https://github.com/dashevo/wallet-lib/compare/v7.13.1...v7.13.2) (2020-06-15)

- **Features:**
    * feature: Worker will now have ability to return a value on onStart and onExecute ([#149](https://github.com/dashevo/wallet-lib/pull/149))
    
- **Fixes:**
    * fix: comportement on new address with existing transaction in store ([#147](https://github.com/dashevo/wallet-lib/pull/147))
    * fix: SyncUp plugin not awaiting long enough ([#149](https://github.com/dashevo/wallet-lib/pull/149))

# [7.13.1](https://github.com/dashevo/wallet-lib/compare/v7.13.0...v7.13.1) (2020-06-15)

- **Fixes:**
    * fix(Storage): identityIds being restate to empty array ([#143](https://github.com/dashevo/wallet-lib/pull/143))

# [7.13.0](https://github.com/dashevo/wallet-lib/compare/v7.1.4...v7.13.0) (2020-06-13)

- **Feat:**
    * sync of identities associated with wallet ([#142](https://github.com/dashevo/wallet-lib/pull/142))

- **Breaking changes:**
    * `Account#getIdentityHDKey` is removed in favor of `Account#getIdentityHDKeyByIndex(identityIndex, keyIndex)`
    * `debug` option temporary disabled

# [7.1.4](https://github.com/dashevo/wallet-lib/compare/v7.1.3...v7.1.4) (2020-06-11)
    
- **Builds, Tests:**
    - test: create a new wallet in functional tests (#140)
    - build: simplify distributive and Travis CI builds (#139)

# [7.1.3](https://github.com/dashevo/wallet-lib/compare/v7.1.2...v7.1.3) (2020-06-10)
    
- **Chore:**
    - chore: Update dashcore-lib version (#138)

# [7.1.2](https://github.com/dashevo/wallet-lib/compare/v7.1.1...v7.1.2) (2020-06-10)
    
- **Feat:**
    - feat: TransactionOrderer (#136)

# [7.1.1](https://github.com/dashevo/wallet-lib/compare/v7.1.0...v7.1.1) (2020-06-03)
    
- **Fixes:**
    - fix: broadcastTransaction not throwing an error when a transaction wasn't broadcasted (#133)
    - fix: internal UTXO on Output format and getUTXO returning UnspentOutput + refactor initial sync up (#135)

# [7.1.0](https://github.com/dashevo/wallet-lib/compare/v7.0.0...v7.1.0) (2020-06-03)
    
- **Fixes:**
    - fix: unavailable previous transactions history (#131)
    - fix: transporter.resolve to extend passed options (#130)

# [7.0.0](https://github.com/dashevo/wallet-lib/compare/v6.1.2...v7.0.0) (2020-06-01)

- **Impr:**
    - impr!: removed eventemitter2 (#128)
    
- **Fixes:**
    - fix!: handling errors on account init (#127)
    
- **Chore, Docs & Tests:**
    - tests: replace browser.js to wallet.js in karma.conf (#126)
    
# [6.1.2](https://github.com/dashevo/wallet-lib/compare/v6.1.1...v6.1.2) (2020-05-22)

- **Fixes:**
    - fix: update evonet seeds (#120)
    
- **Chore, Docs & Tests:**
    - tests: added karma and functional browser test (#121)
    - style: removed logger.error & improved error message (#118)
    
# [6.1.1](https://github.com/dashevo/wallet-lib/compare/v6.1.0...v6.1.1) (2020-05-22)

- **Fixes:**
    - fix: update evonet seeds (#120)
    
# [6.1.0](https://github.com/dashevo/wallet-lib/compare/v6.0.0...v6.1.0) (2020-04-23)

- **Features:** 
    - Feat(Transporter): added .getBestBlock / .getBestBlockHeader (#110 )

- **Fixes:**
    - Fix : Support for DAPIClient.getUTXO with more than 1000 utxos (#111 )
    - Fix: Empty confirmed balance (#109)
    - Refact: Removed Identity Types + dpp (#114)
    - Fix: Removed palinka, updated seeds (#117)

- **Chore, Docs & Tests:**
    - Doc: fixed link and duplicates (#113)
    - Tests: refactorate + fakenet (#115)

# [6.0.0](https://github.com/dashevo/wallet-lib/compare/v5.0.3...v6.0.0) (2020-03-10)


- **breaking:**
  - Wallet:
    - Wallet({transport}) is now Wallet({transporter}) (#102)
  - Account:
    - account.transport is now account.transporter (#102)
    - account.transport.transport is now account.transporter.client (#102)
    - fetchTransactionInfo() is removed. Use getTransaction() instead. (#102)
    - .getTransactionHistory() removed (#102, 01d5b31)
  - Transporter:
    - new Transporter() is now invalid, use Transporters.resolve(arg) instead. (#102)
  - Storage:
    - Storage cannot be assigned an events anymore (storage.parentEvents now). (#102)
    - ChainWorker:
    - ChainWorker became a ChainPlugin using subscribeToBlock() (#102)
  - misc:
    - all events payload will now be returned under form {type, payload} (#102)
    - all events are now accessed via .on() instead of .events.on() (#102)
    - all events are to be emmited using .emit() instead of .events.emit() (#102)
    - format of transactions internally has changed (returns a proper Dashcore Transaction object) (#102)
    - internal reference to blockheight changed to blockHeight (#102)
    - format of blocks internally has changed (returns a proper Dashcore Block object) (#102)
    - format of utxo internally has changed (returns a proper Dashcore UTXO object) (#102)

- **Feat**:
  - Wallet: 
    - Sweep paper wallet (#83)
    - Allow to generate a new privateKey (4e120f6)
  - Account:
    - added debug parameters (#102)
    - Added account.getBlockHeader(identifier) method (#102)
    - account.cacheBlockHeaders is now a available option (def: true)
  - Storage:
    - added Storage.importBlockHeader (#102)
    - added Storage.getBlockHeader (#102)
    - added Storage.searchBlockHeader (#102)
  - Transporter: 
    - Transporter arg can take devnetName when type is DAPI (connects to palinka instead of evonet). (#102)
    - subscribeToAddressesTransaction() (#102)
    - subscribeToBlocks() (#102)
    - subscribeToBlockHeaders() - temporary for BloomFilters (#102)
  - Workers: 
    - Workers support onStart() method. (#102)
  - Plugins:
    - Plugins support onStart() method and send a PLUGIN/pluginName/STARTED event. (#102)
- **Impr**: 
  - moved from('event') to EventEmitter2 + wildcard support (5241ce1, 4db66d6, d20df76)
- **Fix**:
  - KeyChain: 
    - .getKeyForPath when SINGLE_ADDRESS mode is now returned as PrivateKey (#102)
  - Account:
    - sequential account index + transporter missing method reporting #103
- **Perf**:
  - removed localforage from default adapter. #104
- **Test**: 
  - Sweep wallet test + integration (ebbd0f8, 
6bd24a3)
  - FakeDevnet class (db46b05)

# [5.0.3](https://github.com/dashevo/wallet-lib/compare/v5.0.2...v5.0.3) (2020-02-01)

- **Feat**:
  - Account:
    - getIdentityHDKey (#99)
- **Fix**: 
    - typos (#98)
