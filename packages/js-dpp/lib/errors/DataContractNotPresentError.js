const ConsensusError = require('./ConsensusError');

class DataContractNotPresentError extends ConsensusError {
  /**
   * @param {string} dataContractId
   */
  constructor(dataContractId) {
    super('Data Contract is not present with Data Contract ID specified in ST Packet');

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

module.exports = DataContractNotPresentError;
