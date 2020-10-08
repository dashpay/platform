const WalletLibError = require('./WalletLibError');


class InvalidDashcoreTransaction extends WalletLibError {
  constructor(tx, reason = 'A Dashcore Transaction object or valid rawTransaction is required') {
    super(`${reason}: ${tx.toString()}`);
  }
}

module.exports = InvalidDashcoreTransaction;
