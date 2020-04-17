const ConsensusError = require('./ConsensusError');

class InvalidIdentityOutPointError extends ConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid Identity out point: ${message}`);
  }
}

module.exports = InvalidIdentityOutPointError;
