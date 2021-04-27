const Identifier = require('../../../../identifier/Identifier');
const hash = require('../../../../util/hash');

class ChainAssetLockProof {
  /**
   * @param {RawChainAssetLockProof} rawAssetLockProof
   */
  constructor(rawAssetLockProof) {
    this.coreChainLockedHeight = rawAssetLockProof.coreChainLockedHeight;
    this.outPoint = rawAssetLockProof.outPoint;
  }

  /**
   * Get proof type
   *
   * @returns {number}
   */
  getType() {
    return ChainAssetLockProof.type;
  }

  /**
   * Get Asset Lock proof core height
   *
   * @returns {number}
   */
  getCoreChainLockedHeight() {
    return this.coreChainLockedHeight;
  }

  /**
   * Get outPoint
   *
   * @return {Buffer}
   */
  getOutPoint() {
    return this.outPoint;
  }

  /**
   * Create identifier
   *
   * @returns {Identifier}
   */
  createIdentifier() {
    return new Identifier(
      hash(this.getOutPoint()),
    );
  }

  /**
   * Get plain object representation
   *
   * @returns {RawChainAssetLockProof}
   */
  toObject() {
    return {
      type: this.getType(),
      coreChainLockedHeight: this.getCoreChainLockedHeight(),
      outPoint: this.getOutPoint(),
    };
  }

  /**
   * Get JSON representation
   *
   * @returns {JsonChainAssetLockProof}
   */
  toJSON() {
    return {
      type: this.getType(),
      coreChainLockedHeight: this.getCoreChainLockedHeight(),
      outPoint: this.getOutPoint().toString('base64'),
    };
  }
}

/**
 * @typedef {Object} RawChainAssetLockProof
 * @property {number} type
 * @property {number} coreChainLockedHeight
 * @property {Buffer} outPoint
 */

/**
 * @typedef {Object} JsonChainAssetLockProof
 * @property {number} type
 * @property {number} coreChainLockedHeight
 * @property {string} outPoint
 */

ChainAssetLockProof.type = 1;

module.exports = ChainAssetLockProof;
