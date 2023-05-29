const DriveError = require('../../errors/DriveError');

class NotEnoughBlocksForValidSMLError extends DriveError {
  /**
   * @param {number} blockHeight
   */
  constructor(blockHeight) {
    super(`${blockHeight} blocks are not enough to obtain comprehensive SML. Needs 16 block diffs minimum`);

    this.blockHeight = blockHeight;
  }

  /**
   * Get block height
   *
   * @return {number}
   */
  getBlockHeight() {
    return this.blockHeight;
  }
}

module.exports = NotEnoughBlocksForValidSMLError;
