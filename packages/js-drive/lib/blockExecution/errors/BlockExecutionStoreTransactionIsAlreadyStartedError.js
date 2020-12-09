const DriveError = require('../../errors/DriveError');

class BlockExecutionStoreTransactionIsAlreadyStartedError extends DriveError {
  /**
   * Indicates, if Transaction was started when it should't
   */
  constructor() {
    super('Transaction is already started');
  }
}

module.exports = BlockExecutionStoreTransactionIsAlreadyStartedError;
