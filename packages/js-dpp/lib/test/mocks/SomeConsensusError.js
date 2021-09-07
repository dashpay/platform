const AbstractConsensusError = require('../../errors/consensus/AbstractConsensusError');

class SomeConsensusError extends AbstractConsensusError {
  constructor(message) {
    super(message);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @returns {number}
   */
  getCode() {
    return Number.MAX_SAFE_INTEGER;
  }
}

module.exports = SomeConsensusError;
