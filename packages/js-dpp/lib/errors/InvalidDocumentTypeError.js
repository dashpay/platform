const ConsensusError = require('./ConsensusError');

class InvalidDocumentTypeError extends ConsensusError {
  /**
   * @param {string} type
   * @param {Contract} contract
   */
  constructor(type, contract) {
    super(`Contract ${contract.name} doesn't contain type ${type}`);

    this.name = this.constructor.name;

    this.type = type;
    this.contract = contract;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get type
   *
   * @return {string}
   */
  getType() {
    return this.type;
  }

  /**
   * Get Contract
   *
   * @return {Contract}
   */
  getContract() {
    return this.contract;
  }
}

module.exports = InvalidDocumentTypeError;
