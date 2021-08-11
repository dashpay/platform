const { InstantLock, Transaction } = require('@dashevo/dashcore-lib');
const hash = require('../../../../util/hash');
const Identifier = require('../../../../identifier/Identifier');

class InstantAssetLockProof {
  /**
   * @param {RawInstantAssetLockProof} rawAssetLockProof
   */
  constructor(rawAssetLockProof) {
    this.instantLock = InstantLock.fromBuffer(rawAssetLockProof.instantLock);
    this.transaction = new Transaction(rawAssetLockProof.transaction);
    this.outputIndex = rawAssetLockProof.outputIndex;
  }

  /**
   * Get proof type
   *
   * @returns {number}
   */
  getType() {
    return InstantAssetLockProof.type;
  }

  /**
   * Get asset lock transaction output index
   *
   * @returns {number}
   */
  getOutputIndex() {
    return this.outputIndex;
  }

  /**
   * Get transaction outPoint
   * @return {Buffer}
   */
  getOutPoint() {
    return this.transaction.getOutPointBuffer(this.getOutputIndex());
  }

  /**
   * Get transaction output
   *
   * @returns {Output}
   */
  getOutput() {
    return this.transaction.outputs[this.getOutputIndex()];
  }

  /**
   * Create identifier
   *
   * @returns {Identifier}
   */
  createIdentifier() {
    return new Identifier(
      hash(this.getTransaction().getOutPointBuffer(this.getOutputIndex())),
    );
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
   * Get asset lock transaction
   *
   * @returns {Transaction}
   */
  getTransaction() {
    return this.transaction;
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
      transaction: this.getTransaction().toBuffer(),
      outputIndex: this.getOutputIndex(),
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
      transaction: this.getTransaction().toString('base64'),
      outputIndex: this.getOutputIndex(),
    };
  }
}

/**
 * @typedef {Object} RawInstantAssetLockProof
 * @property {number} type
 * @property {Buffer} instantLock
 * @property {Buffer} transaction
 * @property {number} outputIndex
 */

/**
 * @typedef {Object} JsonInstantAssetLockProof
 * @property {number} type
 * @property {string} instantLock
 * @property {string} transaction
 * @property {number} outputIndex
 */

InstantAssetLockProof.type = 0;

module.exports = InstantAssetLockProof;
