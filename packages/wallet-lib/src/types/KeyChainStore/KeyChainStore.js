class KeyChainStore {
  constructor() {
    this.keyChains = new Map();
    this.masterKeyChainId = null;
  }
}

import _KeyChainStore_addKeyChain from './methods/addKeyChain.js';
KeyChainStore.prototype.addKeyChain = _KeyChainStore_addKeyChain;
import _KeyChainStore_getKeyChain from './methods/getKeyChain.js';
KeyChainStore.prototype.getKeyChain = _KeyChainStore_getKeyChain;
import _KeyChainStore_getKeyChains from './methods/getKeyChains.js';
KeyChainStore.prototype.getKeyChains = _KeyChainStore_getKeyChains;
import _KeyChainStore_makeChildKeyChainStore from './methods/makeChildKeyChainStore.js';
KeyChainStore.prototype.makeChildKeyChainStore = _KeyChainStore_makeChildKeyChainStore;
import _KeyChainStore_getMasterKeyChain from './methods/getMasterKeyChain.js';
KeyChainStore.prototype.getMasterKeyChain = _KeyChainStore_getMasterKeyChain;

export default KeyChainStore;