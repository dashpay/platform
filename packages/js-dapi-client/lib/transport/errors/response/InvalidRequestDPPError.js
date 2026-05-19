import ResponseError from './ResponseError.js';

class InvalidRequestDPPError extends ResponseError {
  /**
   *
   * @param {AbstractConsensusError} consensusError
   * @param {object} data
   * @param {DAPIAddress} dapiAddress
   */
  constructor(consensusError, data, dapiAddress) {
    super(consensusError.getCode(), consensusError.message, data, dapiAddress);

    this.consensusError = consensusError;
  }

  /**
   * @returns {AbstractConsensusError}
   */
  getConsensusError() {
    return this.consensusError;
  }
}

export default InvalidRequestDPPError;
