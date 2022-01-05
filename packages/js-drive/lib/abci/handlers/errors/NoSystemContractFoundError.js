const DriveError = require('../../../errors/DriveError');

class NoSystemContractFoundError extends DriveError {
  /**
   * @param {string} contractName
   * @param {Identifier} contractId
   * @param {number} height
   */
  constructor(contractName, contractId, height) {
    super(`${contractName} contract with ID ${contractId} on height ${height} was not found`);

    this.contractName = contractName;
    this.contractId = contractId;
    this.height = height;
  }

  /**
   * @return {string}
   */
  getContractName() {
    return this.contractName;
  }

  /**
   * Get contract id
   *
   * @return {Identifier}
   */
  getContractId() {
    return this.contractId;
  }

  /**
   * Get height
   *
   * @return {number}
   */
  getHeight() {
    return this.height;
  }
}

module.exports = NoSystemContractFoundError;
