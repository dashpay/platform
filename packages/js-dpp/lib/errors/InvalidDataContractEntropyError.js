const ConsensusError = require('./ConsensusError');

class InvalidDataContractEntropyError extends ConsensusError {
  /**
   * @param {RawDataContract} rawDataContract
   */
  constructor(rawDataContract) {
    super('Invalid DataContract entropy');

    this.rawDataContract = rawDataContract;
  }

  /**
   * Get raw DataContract
   *
   * @return {RawDataContract}
   */
  getRawDataContract() {
    return this.rawDataContract;
  }
}

module.exports = InvalidDataContractEntropyError;
