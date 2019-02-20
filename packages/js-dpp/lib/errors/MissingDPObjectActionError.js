const ConsensusError = require('./ConsensusError');

class MissingDPObjectActionError extends ConsensusError {
  constructor(rawDPObject) {
    super('$action is not present');

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

module.exports = MissingDPObjectActionError;
