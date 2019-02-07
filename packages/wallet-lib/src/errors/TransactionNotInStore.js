const WalletLibError = require('./WalletLibError');

class TransactionNotInStore extends WalletLibError {
  constructor(txid) {
    const getErrorMessageOf = () => `Transaction is not in store : ${txid} `;

    super(getErrorMessageOf((txid)));
  }
}
module.exports = TransactionNotInStore;
