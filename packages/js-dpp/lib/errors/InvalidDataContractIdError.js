const ConsensusError = require('./ConsensusError');

class InvalidDataContractIdError extends ConsensusError {
  /**
   * @param {RawDataContract} rawDataContract
   */
  constructor(rawDataContract) {
    super('Invalid DataContract id');

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

module.exports = InvalidDataContractIdError;
