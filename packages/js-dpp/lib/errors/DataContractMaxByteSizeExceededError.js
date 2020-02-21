const ConsensusError = require('./ConsensusError');

class DataContractMaxByteSizeExceededError extends ConsensusError {
  /**
   * @param {RawDataContract} dataContract
   */
  constructor(dataContract) {
    super(`Maximum Data Contract size of ${DataContractMaxByteSizeExceededError.MAX_SIZE} bytes is reached`);

    this.dataContract = dataContract;
  }

  /**
   * Get data contract
   *
   * @return {RawDataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

DataContractMaxByteSizeExceededError.MAX_SIZE = 15 * 1024; // 15 Kb

module.exports = DataContractMaxByteSizeExceededError;
