const WalletLibError = require('./WalletLibError');


class InvalidDashcoreTransaction extends WalletLibError {
  constructor(tx, reason = 'A Dashcore transaction object is required') {
    super(`${reason}: ${tx.toString()}`);
  }
}

module.exports = InvalidDashcoreTransaction;
