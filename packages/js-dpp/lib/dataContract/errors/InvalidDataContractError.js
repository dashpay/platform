const DPPError = require('../../errors/DPPError');

class InvalidDataContractError extends DPPError {
  /**
   * @param {AbstractConsensusError[]} errors
   * @param {RawDataContract} rawDataContract
   */
  constructor(errors, rawDataContract) {
    let message = `Invalid Data Contract: "${errors[0].message}"`;
    if (errors.length > 1) {
      message = `${message} and ${errors.length - 1} more`;
    }

    super(message);

    this.errors = errors;
    this.rawDataContract = rawDataContract;
  }

  /**
   * Get validation errors
   *
   * @return {AbstractConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw Data Contract
   *
   * @return {RawDataContract}
   */
  getRawDataContract() {
    return this.rawDataContract;
  }
}

module.exports = InvalidDataContractError;
