const SCHEMA = {
  lastKnownBlock: {
    height: 'number',
  },
};

class WalletStore {
  constructor(walletId) {
    this.walletId = walletId;

    this.state = {
      mnemonic: null,
      paths: new Map(),
      identities: new Map(),
      lastKnownBlock: {
        height: -1,
      },
    };
  }

  /**
   * Updates last known block value
   * @param height - height of a last known block
   */
  updateLastKnownBlock(height) {
    if (this.state.lastKnownBlock.height >= height) {
      return;
    }

    this.state.lastKnownBlock.height = height;
  }
}

WalletStore.prototype.SCHEMA = SCHEMA;

WalletStore.prototype.createPathState = require('./methods/createPathState');
WalletStore.prototype.exportState = require('./methods/exportState');
WalletStore.prototype.getIdentityIdByIndex = require('./methods/getIdentityIdByIndex');
WalletStore.prototype.getIndexedIdentityIds = require('./methods/getIndexedIdentityIds');
WalletStore.prototype.getPathState = require('./methods/getPathState');
WalletStore.prototype.importState = require('./methods/importState');
WalletStore.prototype.insertIdentityIdAtIndex = require('./methods/insertIdentityIdAtIndex');

module.exports = WalletStore;
