const WalletLibError = require('./WalletLibError');

class BlockHeaderNotInStore extends WalletLibError {
  constructor(identifier) {
    const getErrorMessageOf = () => `Blockheader is not in store : ${identifier} `;

    super(getErrorMessageOf((identifier)));
  }
}
module.exports = BlockHeaderNotInStore;
