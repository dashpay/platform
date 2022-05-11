const AbstractOperation = require('./AbstractOperation');

const DPPError = require('../../../errors/DPPError');

const {
  VERIFY_SIGNATURE_COSTS,
} = require('../constants');

class SignatureVerificationOperation extends AbstractOperation {
  /**
   * @param {number} signatureType
   */
  constructor(signatureType) {
    super();

    if (!VERIFY_SIGNATURE_COSTS[signatureType]) {
      throw new DPPError(`Operation cost for verification of identity key type ${signatureType} is not defined`);
    }

    this.signatureType = signatureType;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getProcessingCost() {
    return VERIFY_SIGNATURE_COSTS[this.signatureType];
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return 0;
  }

  /**
   * @return {{signatureType: number, type: string}}
   */
  toJSON() {
    return {
      type: 'signatureVerification',
      signatureType: this.signatureType,
    };
  }

  /**
   * @param {{signatureType: number, type: string}} json
   * @return {SignatureVerificationOperation}
   */
  static fromJSON(json) {
    return new SignatureVerificationOperation(json.signatureType);
  }
}

module.exports = SignatureVerificationOperation;
