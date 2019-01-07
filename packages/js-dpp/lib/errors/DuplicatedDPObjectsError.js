const ConsensusError = require('./ConsensusError');

class DuplicatedDPObjectsError extends ConsensusError {
  /**
   * @param {Object[]} duplicatedDPObjects
   */
  constructor(duplicatedDPObjects) {
    super('Duplicated DPObjects in ST Packet');

    this.duplicatedDPObjects = duplicatedDPObjects;
  }

  /**
   * Get Duplicated DPObjects
   *
   * @return {Object[]}
   */
  getDuplicatedDPObjects() {
    return this.duplicatedDPObjects;
  }
}

module.exports = DuplicatedDPObjectsError;
