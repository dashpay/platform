const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const Identifier = require('../../../identifier/Identifier');
const AssetLock = require('../assetLock/AssetLock');

class IdentityTopUpTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityTopUpTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(rawStateTransition);

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'identityId')) {
      this.setIdentityId(rawStateTransition.identityId);
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'assetLock')) {
      this.setAssetLock(new AssetLock(rawStateTransition.assetLock));
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
   * @param {AssetLock} assetLock
   * @return {IdentityTopUpTransition}
   */
  setAssetLock(assetLock) {
    this.assetLock = assetLock;

    return this;
  }

  /**
   * @return {AssetLock}
   */
  getAssetLock() {
    return this.assetLock;
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
      assetLock: this.getAssetLock().toObject(),
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
      assetLock: this.getAssetLock().toJSON(),
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
 * @property {RawAssetLock} assetLock
 * @property {Buffer} identityId
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityTopUpTransition
 * @property {JsonAssetLock} assetLock
 * @property {string} identityId
 */

module.exports = IdentityTopUpTransition;
