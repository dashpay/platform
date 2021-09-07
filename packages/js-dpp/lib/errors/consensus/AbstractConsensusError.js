const DPPError = require('../DPPError');

const CONSTRUCTOR_ARGUMENTS_SYMBOL = Symbol.for('constructorArguments');

/**
 * @abstract
 */
class AbstractConsensusError extends DPPError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(message);

    this[CONSTRUCTOR_ARGUMENTS_SYMBOL] = [];
  }

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

  /**
   * Get array of the error's arguments
   *
   * @returns {*[]}
   */
  getConstructorArguments() {
    return this[CONSTRUCTOR_ARGUMENTS_SYMBOL];
  }

  /**
   * Set the error's arguments.
   * Must be called from the constructor
   *
   * @protected
   * @param {Object|Array} args
   */
  setConstructorArguments(args) {
    this[CONSTRUCTOR_ARGUMENTS_SYMBOL] = Array.from(args);
  }
}

module.exports = AbstractConsensusError;
