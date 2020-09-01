const WalletLibError = require('./WalletLibError');

class TransactionNotInStore extends WalletLibError {
  constructor(txid) {
    super(`Transaction is not in store: ${txid}`);
  }
}

module.exports = TransactionNotInStore;
