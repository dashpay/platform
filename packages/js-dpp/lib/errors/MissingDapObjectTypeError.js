const ConsensusError = require('./ConsensusError');

class MissingDapObjectTypeError extends ConsensusError {
  constructor(rawDapObject) {
    super('$type is not present');

    this.rawDapObject = rawDapObject;
  }

  /**
   * Get raw Dap Object
   *
   * @return {Object}
   */
  getRawDapObject() {
    return this.rawDapObject;
  }
}

module.exports = MissingDapObjectTypeError;
