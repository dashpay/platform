const WalletLibError = require('./WalletLibError');

class TransactionMetadataNotInStore extends WalletLibError {
  constructor(txid) {
    super(`Transaction metadata is not in store: ${txid}`);
  }
}

module.exports = TransactionMetadataNotInStore;
