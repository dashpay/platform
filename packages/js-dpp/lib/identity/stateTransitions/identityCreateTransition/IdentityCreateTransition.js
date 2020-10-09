const hash = require('../../../util/hash');
const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const IdentityPublicKey = require('../../IdentityPublicKey');
const Identifier = require('../../../Identifier');

class IdentityCreateTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityCreateTransition} [rawIdentityCreateTransition]
   */
  constructor(rawIdentityCreateTransition = {}) {
    super(rawIdentityCreateTransition);

    this.publicKeys = [];

    if (Object.prototype.hasOwnProperty.call(rawIdentityCreateTransition, 'publicKeys')) {
      this.setPublicKeys(
        rawIdentityCreateTransition.publicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityCreateTransition, 'lockedOutPoint')) {
      this.setLockedOutPoint(rawIdentityCreateTransition.lockedOutPoint);
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
   * Sets an outPoint. OutPoint is a pointer to the output funding identity creation.
   * Its hash also serves as an identity id.
   * More about the OutPoint can be found in the identity documentation
   *
   * @param {Buffer} lockedOutPoint
   * @return {IdentityCreateTransition}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;

    this.identityId = new Identifier(
      hash(this.lockedOutPoint),
    );

    return this;
  }

  /**
   * @return {Buffer}
   */
  getLockedOutPoint() {
    return this.lockedOutPoint;
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
      lockedOutPoint: this.getLockedOutPoint(),
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
      lockedOutPoint: this.getLockedOutPoint().toString('base64'),
      publicKeys: this.getPublicKeys().map((publicKey) => publicKey.toJSON()),
    };
  }
}

/**
 * @typedef {RawStateTransition & Object} RawIdentityCreateTransition
 * @property {Buffer} lockedOutPoint
 * @property {RawIdentityPublicKey[]} publicKeys
 */

/**
 * @typedef {JsonStateTransition & Object} JsonIdentityCreateTransition
 * @property {Buffer} lockedOutPoint
 * @property {JsonIdentityPublicKey[]} publicKeys
 */

module.exports = IdentityCreateTransition;
