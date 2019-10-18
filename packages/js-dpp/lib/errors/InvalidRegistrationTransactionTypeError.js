const ConsensusError = require('./ConsensusError');

class InvalidRegistrationTransactionTypeError extends ConsensusError {
  /**
   * @param {Object} rawTransaction
   */
  constructor(rawTransaction) {
    super(`Expected registration transaction but received transaction with type ${rawTransaction.type}.`);

    this.rawTransaction = rawTransaction;
  }

  /**
   * Get raw transaction
   *
   * @return {Object}
   */
  getRawTransaction() {
    return this.rawTransaction;
  }
}

module.exports = InvalidRegistrationTransactionTypeError;
