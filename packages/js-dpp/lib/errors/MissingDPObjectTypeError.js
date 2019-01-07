const ConsensusError = require('./ConsensusError');

class MissingDPObjectTypeError extends ConsensusError {
  constructor(rawDPObject) {
    super('$type is not present');

    this.rawDPObject = rawDPObject;
  }

  /**
   * Get raw DP Object
   *
   * @return {Object}
   */
  getRawDPObject() {
    return this.rawDPObject;
  }
}

module.exports = MissingDPObjectTypeError;
