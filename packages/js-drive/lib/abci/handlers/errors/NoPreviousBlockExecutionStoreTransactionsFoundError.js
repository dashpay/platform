const DriveError = require('../../../errors/DriveError');

class NoPreviousBlockExecutionStoreTransactionsFoundError extends DriveError {
  constructor() {
    super('No previous block execution store transactions found');
  }
}

module.exports = NoPreviousBlockExecutionStoreTransactionsFoundError;
