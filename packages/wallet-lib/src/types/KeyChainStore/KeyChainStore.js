class KeyChainStore {
  constructor() {
    this.keyChains = new Map();
    this.masterKeyChainId = null;
  }
}

KeyChainStore.prototype.addKeyChain = require('./methods/addKeyChain');
KeyChainStore.prototype.getKeyChain = require('./methods/getKeyChain');
KeyChainStore.prototype.getKeyChains = require('./methods/getKeyChains');
KeyChainStore.prototype.makeChildKeyChainStore = require('./methods/makeChildKeyChainStore');
KeyChainStore.prototype.getMasterKeyChain = require('./methods/getMasterKeyChain');

module.exports = KeyChainStore;
