const { Transaction } = require('@dashevo/dashcore-lib');

const hash = require('../../../util/hash');

const InstantAssetLockProof = require('./proof/instant/InstantAssetLockProof');
const Identifier = require('../../../identifier/Identifier');

class AssetLock {
  /**
   * @param {RawAssetLock} rawAssetLock
   */
  constructor(rawAssetLock) {
    this.outputIndex = rawAssetLock.outputIndex;
    this.transaction = new Transaction(rawAssetLock.transaction);
    this.proof = new AssetLock.PROOF_CLASSES_BY_TYPES[rawAssetLock.proof.type](rawAssetLock.proof);
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
   * Get asset lock transaction output index
   *
   * @returns {number}
   */
  getOutputIndex() {
    return this.outputIndex;
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
   * Get proof
   *
   * @returns {InstantAssetLockProof}
   */
  getProof() {
    return this.proof;
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
   *
   * @returns {RawAssetLock}
   */
  toObject() {
    return {
      transaction: this.getTransaction().toBuffer(),
      outputIndex: this.getOutputIndex(),
      proof: this.getProof().toObject(),
    };
  }

  /**
   * @returns {JsonAssetLock}
   */
  toJSON() {
    return {
      ...this.toObject(),
      transaction: this.getTransaction().toString('base64'),
      proof: this.getProof().toJSON(),
    };
  }
}

/**
 * @typedef {Object} RawAssetLock
 * @property {Buffer} transaction
 * @property {number} outputIndex
 * @property {RawInstantAssetLockProof} proof
 */

/**
 * @typedef {Object} JsonAssetLock
 * @property {string} transaction
 * @property {number} outputIndex
 * @property {JsonInstantAssetLockProof} proof
 */

AssetLock.PROOF_TYPE_INSTANT = 0;

AssetLock.PROOF_CLASSES_BY_TYPES = {
  [AssetLock.PROOF_TYPE_INSTANT]: InstantAssetLockProof,
};

module.exports = AssetLock;
