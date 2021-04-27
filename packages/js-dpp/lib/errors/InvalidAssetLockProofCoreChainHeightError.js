const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockProofCoreHeightError extends ConsensusError {
  /**
   *
   * @param {number} proofCoreChainLockedHeight
   * @param {number} currentCoreChainLockedHeight
   */
  constructor(proofCoreChainLockedHeight, currentCoreChainLockedHeight) {
    super(`Asset Lock proof core chain height ${proofCoreChainLockedHeight} is higher than the current consensus core height ${currentCoreChainLockedHeight}.`);

    this.proofCoreChainLockedHeight = proofCoreChainLockedHeight;
    this.currentCoreChainLockedHeight = currentCoreChainLockedHeight;
  }

  /**
   *
   * @returns {number}
   */
  getProofCoreChainLockedHeight() {
    return this.proofCoreChainLockedHeight;
  }

  /**
   *
   * @returns {number}
   */
  getCurrentCoreChainLockedHeight() {
    return this.currentCoreChainLockedHeight;
  }
}

module.exports = InvalidIdentityAssetLockProofCoreHeightError;
