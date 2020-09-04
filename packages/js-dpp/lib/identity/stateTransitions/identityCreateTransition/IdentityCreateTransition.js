const bs58 = require('bs58');

const hash = require('../../../util/hash');
const AbstractStateTransition = require('../../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const IdentityPublicKey = require('../../IdentityPublicKey');

class IdentityCreateTransition extends AbstractStateTransition {
  /**
   * @param {RawIdentityCreateTransition} [rawIdentityCreateTransition]
   */
  constructor(rawIdentityCreateTransition) {
    super(rawIdentityCreateTransition);

    this.publicKeys = [];

    if (rawIdentityCreateTransition) {
      if (rawIdentityCreateTransition.publicKeys) {
        this.setPublicKeys(
          rawIdentityCreateTransition.publicKeys
            .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
        );
      }

      this
        .setLockedOutPoint(rawIdentityCreateTransition.lockedOutPoint);
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
   * @param {string} lockedOutPoint
   * @return {IdentityCreateTransition}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;
    this.identityId = bs58.encode(
      hash(Buffer.from(lockedOutPoint, 'base64')),
    );

    return this;
  }

  /**
   * @return {string}
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
   * Returns base58 representation of the future identity id
   *
   * @return {string}
   */
  getIdentityId() {
    return this.identityId;
  }

  /**
   * Returns Owner ID
   *
   * @return {string}
   */
  getOwnerId() {
    return this.identityId;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature]
   *
   * @return {Object}
   */
  toObject(options = {}) {
    return {
      ...super.toObject(options),
      lockedOutPoint: this.getLockedOutPoint(),
      publicKeys: this.getPublicKeys().map((publicKey) => publicKey.toJSON()),
    };
  }

  /**
   * Create state transition from JSON
   *
   * @param {RawIdentityCreateTransition} rawStateTransition
   *
   * @return {IdentityCreateTransition}
   */
  static fromJSON(rawStateTransition) {
    return new IdentityCreateTransition(
      AbstractStateTransition.translateJsonToObject(rawStateTransition),
    );
  }
}

/**
 * @typedef {Object} RawIdentityCreateTransition
 * @extends AbstractStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {string} lockedOutPoint
 * @property {RawIdentityPublicKey[]} publicKeys
 * @property {number|null} signaturePublicKeyId
 * @property {string|null} signature
 */

module.exports = IdentityCreateTransition;
