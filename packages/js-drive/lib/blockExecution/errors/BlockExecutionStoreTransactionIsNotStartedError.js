const DriveError = require('../../errors/DriveError');

class BlockExecutionStoreTransactionIsNotStartedError extends DriveError {
  /**
   * Indicates, if Transaction was not started when is should
   */
  constructor() {
    super('Block execution transaction is not started');
  }
}

module.exports = BlockExecutionStoreTransactionIsNotStartedError;
