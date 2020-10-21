const ConsensusError = require('./ConsensusError');

class DataContractNotPresentError extends ConsensusError {
  /**
   * @param {Identifier|Buffer} dataContractId
   */
  constructor(dataContractId) {
    super('Data Contract is not present with Data Contract ID specified in ST Packet');

    this.dataContractId = dataContractId;
  }

  /**
   * Get Data Contract ID
   *
   * @return {Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = DataContractNotPresentError;
