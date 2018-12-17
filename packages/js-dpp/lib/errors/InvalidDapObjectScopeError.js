const ConsensusError = require('./ConsensusError');

class InvalidDapObjectScopeError extends ConsensusError {
  /**
   * @param {DapObject} dapObject
   */
  constructor(dapObject) {
    super('Invalid Dap Object scope');

    this.dapObject = dapObject;
  }

  /**
   * Get Dap Object
   *
   * @return {DapObject}
   */
  getDapObject() {
    return this.dapObject;
  }
}

module.exports = InvalidDapObjectScopeError;
