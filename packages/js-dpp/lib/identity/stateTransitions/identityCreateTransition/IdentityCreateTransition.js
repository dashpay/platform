const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const IdentityPublicKey = require('../../IdentityPublicKey');
const AssetLock = require('../assetLock/AssetLock');

class IdentityCreateTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityCreateTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(rawStateTransition);

    this.publicKeys = [];

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'publicKeys')) {
      this.setPublicKeys(
        rawStateTransition.publicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
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
    return stateTransitionTypes.IDENTITY_CREATE;
  }

  /**
   * Set asset lock
   *
   * @param {AssetLock} assetLock
   * @return {IdentityCreateTransition}
   */
  setAssetLock(assetLock) {
    this.assetLock = assetLock;

    this.identityId = assetLock.createIdentifier();

    return this;
  }

  /**
   * @return {AssetLock}
   */
  getAssetLock() {
    return this.assetLock;
  }

  /**
   * @return {IdentityPublicKey[]}
   */
  getPublicKeys() {
    return this.publicKeys;
  }

  /**
   * Replaces existing set of public keys with a new one
   * @param {IdentityPublicKey[]} publicKeys
   * @return {IdentityCreateTransition}
   */
  setPublicKeys(publicKeys) {
    this.publicKeys = publicKeys;

    return this;
  }

  /**
   * Adds public keys to the existing public keys array
   * @param {IdentityPublicKey[]} publicKeys
   * @return {IdentityCreateTransition}
   */
  addPublicKeys(publicKeys) {
    this.publicKeys.push(...publicKeys);

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
   * Get raw state transition
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawIdentityCreateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    return {
      ...super.toObject(options),
      assetLock: this.getAssetLock().toObject(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toObject()),
    };
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonIdentityCreateTransition}
   */
  // eslint-disable-next-line no-unused-vars
  toJSON() {
    return {
      ...super.toJSON(),
      assetLock: this.getAssetLock().toJSON(),
      publicKeys: this.getPublicKeys().map((publicKey) => publicKey.toJSON()),
    };
  }
}

/**
 * @typedef {RawStateTransition & Object} RawIdentityCreateTransition
 * @property {RawAssetLock} assetLock
 * @property {RawIdentityPublicKey[]} publicKeys
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityCreateTransition
 * @property {JsonAssetLock} assetLock
 * @property {JsonIdentityPublicKey[]} publicKeys
 */

module.exports = IdentityCreateTransition;
