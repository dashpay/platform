const ConsensusError = require('./ConsensusError');

const DapObject = require('../dapObject/DapObject');

class DapObjectNotFoundError extends ConsensusError {
  /**
   * @param {DapObject} dapObject
   */
  constructor(dapObject) {
    const noun = {
      [DapObject.ACTIONS.UPDATE]: 'Updated',
      [DapObject.ACTIONS.DELETE]: 'Deleted',
    };

    super(`${noun[dapObject.getAction()]} Dap Object not found`);

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

module.exports = DapObjectNotFoundError;
