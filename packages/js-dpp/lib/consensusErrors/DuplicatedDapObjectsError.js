const ConsensusError = require('./ConsensusError');

class DuplicatedDapObjectsError extends ConsensusError {
  /**
   * @param {Object[]} duplicatedDapObjects
   */
  constructor(duplicatedDapObjects) {
    super('Duplicated Dap Objects in ST Packet');

    this.duplicatedDapObjects = duplicatedDapObjects;
  }

  /**
   * Get Duplicated Dap Objects
   *
   * @return {Object[]}
   */
  getDuplicatedDapObject() {
    return this.duplicatedDapObjects;
  }
}

module.exports = DuplicatedDapObjectsError;
