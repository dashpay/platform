class WalletStore {
  constructor(walletId) {
    this.walletId = walletId;
    this.reset();
  }

  reset() {
    this.state = {
      mnemonic: null,
      paths: new Map(),
      identities: new Map(),
    };
  }
}

import _WalletStore_createPathState from './methods/createPathState.js';
WalletStore.prototype.createPathState = _WalletStore_createPathState;
import _WalletStore_getIdentityIdByIndex from './methods/getIdentityIdByIndex.js';
WalletStore.prototype.getIdentityIdByIndex = _WalletStore_getIdentityIdByIndex;
import _WalletStore_getIndexedIdentityIds from './methods/getIndexedIdentityIds.js';
WalletStore.prototype.getIndexedIdentityIds = _WalletStore_getIndexedIdentityIds;
import _WalletStore_getPathState from './methods/getPathState.js';
WalletStore.prototype.getPathState = _WalletStore_getPathState;
import _WalletStore_insertIdentityIdAtIndex from './methods/insertIdentityIdAtIndex.js';
WalletStore.prototype.insertIdentityIdAtIndex = _WalletStore_insertIdentityIdAtIndex;

export default WalletStore;