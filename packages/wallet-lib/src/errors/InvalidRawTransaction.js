const WalletLibError = require('./WalletLibError');


class InvalidTransaction extends WalletLibError {
  constructor(rawtx) {
    const getErrorMessageOf = () => 'A valid transaction object or it\'s hex representation is required';
    super(getErrorMessageOf((rawtx)));
  }
}

module.exports = InvalidTransaction;
