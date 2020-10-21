const WalletLibError = require('./WalletLibError');

class InvalidTransaction extends WalletLibError {
  constructor() {
    super('A valid transaction object or it\'s hex representation is required');
  }
}

module.exports = InvalidTransaction;
