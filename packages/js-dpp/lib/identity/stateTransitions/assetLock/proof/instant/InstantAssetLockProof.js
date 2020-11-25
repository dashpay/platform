const { InstantLock } = require('@dashevo/dashcore-lib');

class InstantAssetLockProof {
  /**
   * @param {RawInstantAssetLockProof} rawAssetLockProof
   */
  constructor(rawAssetLockProof) {
    this.instantLock = InstantLock.fromBuffer(rawAssetLockProof.instantLock);
  }

  /**
   * Get proof type
   *
   * @returns {number}
   */
  getType() {
    return 0;
  }

  /**
   * Get Instant Lock
   *
   * @returns {InstantLock}
   */
  getInstantLock() {
    return this.instantLock;
  }

  /**
   * Get plain object representation
   *
   * @returns {RawInstantAssetLockProof}
   */
  toObject() {
    return {
      type: this.getType(),
      instantLock: this.getInstantLock().toBuffer(),
    };
  }

  /**
   * Get JSON representation
   *
   * @returns {JsonInstantAssetLockProof}
   */
  toJSON() {
    return {
      type: this.getType(),
      instantLock: this.getInstantLock().toBuffer().toString('base64'),
    };
  }
}

/**
 * @typedef {Object} RawInstantAssetLockProof
 * @property {number} type
 * @property {Buffer} instantLock
 */

/**
 * @typedef {Object} JsonInstantAssetLockProof
 * @property {number} type
 * @property {string} instantLock
 */

module.exports = InstantAssetLockProof;
