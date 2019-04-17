const WalletLibError = require('./WalletLibError');


class InvalidDashcoreTransaction extends WalletLibError {
  constructor(tx) {
    const getErrorMessageOf = () => 'A Dashcore transaction object is required';
    super(getErrorMessageOf((tx.toString())));
  }
}

module.exports = InvalidDashcoreTransaction;
