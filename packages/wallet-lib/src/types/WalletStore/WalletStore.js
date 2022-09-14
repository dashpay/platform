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

WalletStore.prototype.createPathState = require('./methods/createPathState');
WalletStore.prototype.getIdentityIdByIndex = require('./methods/getIdentityIdByIndex');
WalletStore.prototype.getIndexedIdentityIds = require('./methods/getIndexedIdentityIds');
WalletStore.prototype.getPathState = require('./methods/getPathState');
WalletStore.prototype.insertIdentityIdAtIndex = require('./methods/insertIdentityIdAtIndex');

module.exports = WalletStore;
