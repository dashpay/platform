const cbor = require('cbor');

const AbstractAbciError = require('./AbstractAbciError');

class DPPValidationAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {ConsensusError} consensusError
   */
  constructor(message, consensusError) {
    const data = {
      serializedError: consensusError.serialize(),
    };

    super(consensusError.getCode(), message, data);
  }

  /**
   * Overload method to skip error message in info
   *
   * @returns {{code: number, info: string}}
   */
  getAbciResponse() {
    const info = { data: this.getData() };

    return {
      code: this.getCode(),
      info: cbor.encode(info).toString('base64'),
    };
  }
}

module.exports = DPPValidationAbciError;
