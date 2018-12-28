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
   * Get contract ID
   *
   * @return {DapContract}
   */
  getTransaction() {
    return this.transaction;
  }
}

module.exports = InvalidTransactionTypeError;
