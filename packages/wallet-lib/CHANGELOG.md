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
