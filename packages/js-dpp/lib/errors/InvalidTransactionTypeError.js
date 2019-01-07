const ConsensusError = require('./ConsensusError');

class InvalidTransactionTypeError extends ConsensusError {
  /**
   * @param {Transaction} transaction
   */
  constructor(transaction) {
    super('Invalid transaction type');

    this.transaction = transaction;
  }

  /**
   * Get transaction
   *
   * @return {Object}
   */
  getTransaction() {
    return this.transaction;
  }
}

module.exports = InvalidTransactionTypeError;
