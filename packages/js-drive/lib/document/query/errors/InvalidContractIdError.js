const ValidationError = require('./ValidationError');

class InvalidContractIdError extends ValidationError {
  /**
   *
   * @param {string} contractId
   */
  constructor(contractId) {
    super(`Invalid contract ID: ${contractId}`);

    this.contractId = contractId;
  }

  /**
   * Invalid contract id
   *
   * @returns {string}
   */
  getContractId() {
    return this.contractId;
  }
}

module.exports = InvalidContractIdError;
