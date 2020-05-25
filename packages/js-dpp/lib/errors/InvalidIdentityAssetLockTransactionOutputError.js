const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockTransactionOutputError extends ConsensusError {
  /**
   * @param {string} message
   * @param {Object} output
   */
  constructor(message, output) {
    super(`Invalid identity lock transaction output: ${message}`);

    this.output = output;
  }

  /**
   * Get lock transaction output
   *
   * @return {Object}
   */
  getOutput() {
    return this.output;
  }
}

module.exports = InvalidIdentityAssetLockTransactionOutputError;
