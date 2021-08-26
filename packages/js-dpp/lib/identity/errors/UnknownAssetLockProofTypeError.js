const DPPError = require('../../errors/DPPError');

class UnknownAssetLockProofTypeError extends DPPError {
  /**
   *
   * @param {number} type
   */
  constructor(type) {
    super('Unknown Asset lock proof type');

    this.type = type;
  }

  /**
   *
   * @returns {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = UnknownAssetLockProofTypeError;
