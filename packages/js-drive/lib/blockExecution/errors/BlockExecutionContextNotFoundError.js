const DriveError = require('../../errors/DriveError');

class BlockExecutionContextNotFoundError extends DriveError {
  /**
   *
   * @param {number} round
   */
  constructor(round) {
    super(`BlockExecutionContext for round ${round} not found`);

    this.round = round;
  }

  /**
   *
   * @return {number}
   */
  getRound() {
    return this.round;
  }
}

module.exports = BlockExecutionContextNotFoundError;
