const ConsensusError = require('./ConsensusError');

class DataContractIdentityNotFoundError extends ConsensusError {
  /**
   * @param {string} dataContractId
   */
  constructor(dataContractId) {
    super(`Data Contract identity ${dataContractId} not found`);

    this.dataContractId = dataContractId;
  }

  /**
   * Get Data Contract ID
   *
   * @return {string}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = DataContractIdentityNotFoundError;
