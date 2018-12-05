const ConsensusError = require('./ConsensusError');

class InvalidDapObjectScopeIdError extends ConsensusError {
  /**
   * @param {Object} rawDapObject
   */
  constructor(rawDapObject) {
    super('Invalid Dap Object scopeId');

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

module.exports = InvalidDapObjectScopeIdError;
