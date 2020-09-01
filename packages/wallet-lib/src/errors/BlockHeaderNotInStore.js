const WalletLibError = require('./WalletLibError');

class BlockHeaderNotInStore extends WalletLibError {
  constructor(identifier) {
    super(`Blockheader is not in store: ${identifier}`);
  }
}
module.exports = BlockHeaderNotInStore;
