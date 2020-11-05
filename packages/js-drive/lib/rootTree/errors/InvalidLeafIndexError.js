const DriveError = require('../../errors/DriveError');

class InvalidLeafIndexError extends DriveError {
  /**
   * @param {AbstractRootTreeLeaf} leaf
   * @param {index} index
   */
  constructor(leaf, index) {
    super(`Leaf index ${leaf.getIndex()} must correspond to the position ${index} in leaves array`);
  }
}

module.exports = InvalidLeafIndexError;
