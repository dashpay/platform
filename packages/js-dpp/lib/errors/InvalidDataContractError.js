const ConsensusError = require('./ConsensusError');

class InvalidDataContractError extends ConsensusError {
  /**
   * @param {DataContract} dataContract
   * @param {RawSTPacket} rawSTPacket
   */
  constructor(dataContract, rawSTPacket) {
    super('Invalid Data Contract for ST Packet validation');

    this.dataContract = dataContract;
    this.rawSTPacket = rawSTPacket;
  }

  /**
   * Get Data Contract ID
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * Get raw ST Packet
   *
   * @return {RawSTPacket}
   */
  getRawSTPacket() {
    return this.rawSTPacket;
  }
}

module.exports = InvalidDataContractError;
