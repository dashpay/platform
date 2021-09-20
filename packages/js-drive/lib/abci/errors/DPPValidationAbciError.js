const cbor = require('cbor');

const AbstractAbciError = require('./AbstractAbciError');

class DPPValidationAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {AbstractConsensusError} consensusError
   */
  constructor(message, consensusError) {
    const data = {
      arguments: consensusError.getConstructorArguments(),
    };

    super(consensusError.getCode(), message, data);
  }

  /**
   * @returns {{code: number, info: string}}
   */
  getAbciResponse() {
    const info = {
      data: this.getData(),
    };

    return {
      code: this.getCode(),
      info: cbor.encode(info).toString('base64'),
    };
  }
}

module.exports = DPPValidationAbciError;
