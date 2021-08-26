const DPPError = require('../DPPError');

class ConsensusError extends DPPError {
  /**
   * @return {number}
   */
  getCode() {
    // Mitigate recursive dependency

    // eslint-disable-next-line global-require
    const codes = require('./codes');

    const code = Object.keys(codes)
      .find((c) => this.constructor === codes[c]);

    if (!code) {
      throw new Error('Error code is not defined');
    }

    return parseInt(code, 10);
  }
}

module.exports = ConsensusError;
