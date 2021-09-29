const cbor = require('cbor');

const AbstractAbciError = require('./AbstractAbciError');

class DPPValidationAbciError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {AbstractConsensusError} consensusError
   */
  constructor(message, consensusError) {
    const args = consensusError.getConstructorArguments();

    const data = { };
    if (args.length > 0) {
      data.arguments = args;
    }

    super(consensusError.getCode(), message, data);
  }

  /**
   * @returns {{code: number, info: string}}
   */
  getAbciResponse() {
    const info = { };

    const data = this.getData();

    let encodedInfo;
    if (Object.keys(data).length > 0) {
      info.data = data;

      encodedInfo = cbor.encode(info).toString('base64');
    }

    return {
      code: this.getCode(),
      info: encodedInfo,
    };
  }
}

module.exports = DPPValidationAbciError;
