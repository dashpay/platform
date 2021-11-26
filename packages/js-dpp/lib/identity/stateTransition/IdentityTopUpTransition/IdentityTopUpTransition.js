const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identifier = require('../../../identifier/Identifier');
const createAssetLockProofInstance = require('../assetLockProof/createAssetLockProofInstance');

class IdentityTopUpTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityTopUpTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(rawStateTransition);

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'identityId')) {
      this.setIdentityId(rawStateTransition.identityId);
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'assetLockProof')) {
      this.setAssetLockProof(createAssetLockProofInstance(rawStateTransition.assetLockProof));
    }
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.IDENTITY_TOP_UP;
  }

  /**
   * Set Asset Lock
   *
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @return {IdentityTopUpTransition}
   */
  setAssetLockProof(assetLockProof) {
    this.assetLockProof = assetLockProof;

    return this;
  }

  /**
   * @return {InstantAssetLockProof|ChainAssetLockProof}
   */
  getAssetLockProof() {
    return this.assetLockProof;
  }

  /**
   * Returns base58 representation of the identity id top up
   *
   * @param {Buffer} identityId
   * @return {IdentityTopUpTransition}
   */
  setIdentityId(identityId) {
    this.identityId = Identifier.from(identityId);

    return this;
  }

  /**
   * Returns identity id
   *
   * @return {Identifier}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Returns Owner ID
   *
   * @return {Identifier}
   */
  getOwnerId() {
    return this.identityId;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawIdentityTopUpTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawStateTransition = {
      ...super.toObject(options),
      identityId: this.getIdentityId(),
      assetLockProof: this.getAssetLockProof().toObject(),
    };

    if (!options.skipIdentifiersConversion) {
      rawStateTransition.identityId = this.getIdentityId().toBuffer();
    }

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonIdentityTopUpTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      identityId: this.getIdentityId().toString(),
      assetLockProof: this.getAssetLockProof().toJSON(),
    };
  }

  /**
   * Returns ids of topped up identities
   *
   * @return {Identifier[]}
   */
  getModifiedDataIds() {
    return [this.getIdentityId()];
  }
}

/**
 * @typedef {RawStateTransition & Object} RawIdentityTopUpTransition
 * @property {RawInstantAssetLockProof|RawChainAssetLockProof} assetLockProof
 * @property {Buffer} identityId
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityTopUpTransition
 * @property {JsonInstantAssetLockProof|JsonChainAssetLockProof} assetLockProof
 * @property {string} identityId
 */

module.exports = IdentityTopUpTransition;
