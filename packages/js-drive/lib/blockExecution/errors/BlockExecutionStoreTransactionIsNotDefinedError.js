const DriveError = require('../../errors/DriveError');

class BlockExecutionStoreTransactionIsNotDefinedError extends DriveError {
  /**
   * Indicates, if Transaction is not defined in BlockExecutionStoreTransactions
   * @param {string} name
   */
  constructor(name) {
    super(`Transaction ${name} is not defined`);

    this.transactionName = name;
  }

  getName() {
    return this.transactionName;
  }
}

module.exports = BlockExecutionStoreTransactionIsNotDefinedError;
